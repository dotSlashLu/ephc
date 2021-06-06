use log::{debug, error};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::kube::Service;

pub(crate) async fn probe(
    svcs: Arc<RwLock<HashMap<String, Arc<RwLock<Service>>>>>,
    connect_timeout: u64,
) {
    let svcs = svcs.read().await;
    for svc in svcs.values() {
        let svc = svc.clone();
        probe_svc(svc, connect_timeout).await;
    }
}

async fn probe_svc(svc: Arc<RwLock<Service>>, connect_timeout: u64) {
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
                std::time::Duration::from_millis(connect_timeout),
                async move {
                    let mut svc = svc_clone.write().await;
                    let ep = &mut svc.endpoints[i];
                    let res = tokio::net::TcpStream::connect(ep.addr).await;
                    match res {
                        Ok(_) => {
                            debug!("{:?} connected", ep.addr);
                            if ep.up() {
                                svc.restore_ep(i);
                            }
                        }
                        Err(e) => {
                            error!("failed to connect to {:?}, {}", ep.addr, e);
                            if ep.down() {
                                svc.remove_ep(i);
                            };
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
        let res = join_res.unwrap();
        if res.is_err() {
            let svc_clone = svc.clone();
            let mut svc_w = svc_clone.write().await;
            let ep = &mut svc_w.endpoints[i];
            error!("failed to connect to {:?}: timed out", ep.addr);
            let remove = ep.down();
            if remove {
                svc_w.remove_ep(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::kube;
    use kube::Threshold;
    use std::{net::SocketAddr, str::FromStr, sync::Arc};
    use tokio::sync::RwLock;

    // #[tokio::test]
    // async fn do_probe() {
    //     let eps = vec![
    //         kube::Endpoint {
    //             addr: SocketAddr::from_str("127.0.0.1:44307").unwrap(),
    //             status: kube::EndpointStatus::Healthy,
    //             counter: kube::Counter {
    //                 up: 0,
    //                 down: 0,
    //             },
    //             threshold: Threshold {
    //                 restore: 3,
    //                 remove: 3
    //             }
    //         },
    //         kube::Endpoint {
    //             addr: SocketAddr::from_str("127.0.0.1:80").unwrap(),
    //             status: kube::EndpointStatus::Healthy,
    //             counter: kube::Counter {
    //                 up: 0,
    //                 down: 0,
    //             },
    //             threshold: Threshold {
    //                 restore: 3,
    //                 remove: 3
    //             }
    //         },
    //     ];

    //     let svc = Arc::new(RwLock::new(kube::Service {
    //         name: "test".to_owned(),
    //         kind: kube::ServiceKind::TCP,
    //         endpoints: eps,
    //         yaml: "".to_owned(),
    //     }));

    //     super::do_probe(svc.clone()).await;

    //     let svc_clone = svc.read().await;
    //     println!("eps after edit: {:?}", svc_clone.endpoints);
    // }
}
