use log::{debug, error};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::kube::Service;

pub(crate) async fn probe(
    svcs: Arc<RwLock<HashMap<String, Arc<RwLock<Service>>>>>,
    connect_timeout: u64,
) -> Option<tokio::sync::TryLockError> {
    let svcs = match svcs.try_write() {
        Err(e) => {
            debug!("services locked, should not schedule new probe");
            return Some(e);
        }
        Ok(svcs) => svcs,
    };
    for svc in svcs.values() {
        let svc = svc.clone();
        probe_svc(svc, connect_timeout).await;
    }
    None
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
                    let addr = ep.addr;
                    let res = tokio::net::TcpStream::connect(ep.addr).await;
                    match res {
                        Ok(_) => {
                            debug!("{:?} connected", ep.addr);
                            if !ep.up() {
                                return;
                            }
                            match svc.restore_ep(i).await {
                                Err(e) => error!("failed to restore ep: {:?}: {}", addr, e),
                                _ => (),
                            }
                        }
                        Err(e) => {
                            error!("failed to connect to {:?}, {}", ep.addr, e);
                            if !ep.down() {
                                return;
                            }
                            match svc.remove_ep(i).await {
                                Err(e) => error!("failed to remove ep: {:?}: {}", addr, e),
                                _ => (),
                            }
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
            let addr = ep.addr;
            error!("failed to connect to {:?}: timed out", ep.addr);
            let remove = ep.down();
            if remove {
                match svc_w.remove_ep(i).await {
                    Err(e) => error!("failed to remove ep: {:?}: {}", addr, e),
                    _ => (),
                }
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
    //             protocol: kube::Protocol::TCP,
    //             status: kube::EndpointStatus::Healthy,
    //             counter: kube::Counter { up: 0, down: 0 },
    //             threshold: Threshold {
    //                 restore: 3,
    //                 remove: 3,
    //             },
    //         },
    //         kube::Endpoint {
    //             addr: SocketAddr::from_str("127.0.0.1:80").unwrap(),
    //             protocol: kube::Protocol::TCP,
    //             status: kube::EndpointStatus::Healthy,
    //             counter: kube::Counter { up: 0, down: 0 },
    //             threshold: Threshold {
    //                 restore: 3,
    //                 remove: 3,
    //             },
    //         },
    //     ];

    //     let yml_str = "
    //     apiVersion: v1
    //     kind: Endpoints
    //     metadata:
    //       creationTimestamp: 2019-03-20T07:23:28Z
    //       name: ephc-test
    //       namespace: default
    //       resourceVersion: \"82479279\"
    //       selfLink: /api/v1/namespaces/default/endpoints/ephc-test
    //       uid: 0ec10531-4ae1-11e9-9c9c-f86eee307061
    //     subsets:
    //     - addresses:
    //       - ip: 172.0.1.4
    //       - ip: 172.0.1.5
    //       - ip: 172.0.1.6
    //       ports:
    //       - name: port80
    //         port: 31000
    //         protocol: TCP
    //       - name: port82
    //         port: 31002
    //         protocol: TCP
    //       - name: port81
    //         port: 31001
    //         protocol: TCP";

    //     let svc = Arc::new(RwLock::new(kube::Service {
    //         name: "test".to_owned(),
    //         endpoints: eps,
    //         our_version: "0".to_owned(),
    //         repr: kube::yaml::ServiceRepr::from_str(yml_str).unwrap(),
    //     }));

    //     super::probe_svc(svc.clone(), 100).await;

    //     let svc_clone = svc.read().await;
    //     println!("eps after edit: {:?}", svc_clone.endpoints);
    // }
}
