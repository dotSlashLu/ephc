use log::error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::kube::{Service, EndpointStatus};

pub(crate) async fn probe(svcs: Vec<Service>) {
    for svc in svcs {
        let svc = Arc::new(RwLock::new(svc));
        do_probe(svc).await;
    }
}

async fn do_probe(svc: Arc<RwLock<Service>>) {
    let mut jhs = Vec::<(JoinHandle<_>, usize)>::new();

    let l = {
        let svc_clone = svc.clone();
        let svc_clone = svc_clone.read().await;
        svc_clone.endpoints.len()
    };

    for i in 0..l {
        let svc_clone = svc.clone();

        jhs.push((
            tokio::spawn(tokio::time::timeout(
                std::time::Duration::from_millis(crate::CONNECT_TIMEOUT),
                async move {
                    let mut svc = svc_clone.write().await;
                    let mut ep = &mut svc.endpoints[i];
                    let res = tokio::net::TcpStream::connect(ep.addr).await;
                    match res {
                        Ok(_) => {
                            println!("{:?} connected", ep.addr)
                        }
                        Err(e) => {
                            println!("failed to connect to {:?}, {}", ep.addr, e);
                            ep.status = EndpointStatus::Removed;
                        }
                    }
                },
            )),
            i,
        ));
    }

    for (jh, i) in jhs {
        let join_res = jh.await;
        if join_res.is_err() {
            error!("failed to join! task: {}", join_res.unwrap_err());
            continue;
        }
        // let (res, ep) = join_res.unwrap();gt
        let res = join_res.unwrap();
        if res.is_err() {
            let svc_clone = svc.read().await;
            let ep = &svc_clone.endpoints[i];
            error!("failed to connect to {:?}: timed out", ep);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::kube;
    use std::{net::SocketAddr, str::FromStr, sync::Arc};
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn do_probe() {
        let eps = vec![
            kube::Endpoint {
                addr: SocketAddr::from_str("127.0.0.1:44307").unwrap(),
                status: kube::EndpointStatus::Healthy,
                counter_up: 0,
                counter_down: 0,
            },
            kube::Endpoint {
                addr: SocketAddr::from_str("127.0.0.1:80").unwrap(),
                status: kube::EndpointStatus::Healthy,
                counter_up: 0,
                counter_down: 0,
            },
        ];

        let svc = Arc::new(RwLock::new(kube::Service {
            name: "test".to_owned(),
            kind: kube::ServiceKind::TCP,
            endpoints: eps,
            yaml: "".to_owned(),
        }));

        super::do_probe(svc.clone()).await;
        
        let svc_clone = svc.read().await;
        println!("eps after edit: {:?}", svc_clone.endpoints);
    }
}
