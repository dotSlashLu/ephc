use log::error;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::kube::{self, Endpoint};

async fn probe(svcs: Vec<kube::Service>) {
    for svc in svcs {
        let svc = Box::new(Arc::new(Mutex::new(svc)));
        let (ups, downs) = do_probe(svc).await;
    }
}

// TODO: use stream to increase concurrency
async fn do_probe(
    svc: Box<Arc<Mutex<kube::Service>>>,
) -> (Vec<&'static kube::Endpoint>, Vec<&'static kube::Endpoint>) {
    let mut down = Vec::<&kube::Endpoint>::new();
    let mut up = Vec::<&kube::Endpoint>::new();

    let mut jhs = Vec::<(JoinHandle<_>, usize)>::new();
    let svc1 = Arc::clone(&svc);
    let svc1 = svc.lock().await;
    let l = svc1.endpoints.len();
    drop(svc1);
    for i in 0..l {
        jhs.push((
            tokio::spawn(tokio::time::timeout(
                std::time::Duration::from_secs(crate::CONNECT_TIMEOUT),
                async move {
                    let svc = Arc::clone(&svc);
                    let svc = svc.lock().await;
                    tokio::net::TcpStream::connect(svc.endpoints[i].addr).await
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
        // let (res, ep) = join_res.unwrap();
        let res = join_res.unwrap();
        println!("{:?}", res);
        let ep = &Arc::clone(&svc).lock().await.endpoints[i];
        match res {
            Ok(res) => {
                if res.is_err() {
                    error!("failed to connect to {}: {}", ep.addr, res.unwrap_err());
                    down.push(&ep);
                    continue;
                }
                up.push(ep);
            }
            Err(_) => {
                error!("failed to connect to {}: timed out", ep.addr);
                down.push(ep);
            }
        };
    }

    (up, down)
}

#[cfg(test)]
mod tests {
    use crate::kube;
    use std::{net::SocketAddr, str::FromStr};

    #[tokio::test]
    async fn do_probe() {
        let svc = kube::Service {
            name: "test".to_owned(),
            kind: kube::ServiceKind::TCP,
            endpoints: vec![
                kube::Endpoint {
                    addr: SocketAddr::from_str("127.0.0.1:3129").unwrap(),
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
            ],
            yaml: "".to_owned(),
        };
        let (up, down) = super::do_probe(&svc).await;
        println!("up: {:?}, down: {:?}", up, down);
    }
}
