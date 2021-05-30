use log::error;
use std::net::SocketAddr;

use crate::kube;

async fn probe(svcs: &Vec<kube::Service>) {
    for svc in svcs {
        let (ups, downs) = do_probe(svc).await;
    };
}

// TODO: use stream to increase concurrency
async fn do_probe(svc: &kube::Service) -> (Vec<&kube::Endpoint>, Vec<&kube::Endpoint>) {
    let mut down = Vec::<&kube::Endpoint>::new();
    let mut up = Vec::<&kube::Endpoint>::new();
    for ep in &svc.endpoints {
        match tokio::time::timeout(
            std::time::Duration::from_secs(crate::CONNECT_TIMEOUT),
            tokio::net::TcpStream::connect(ep.addr),
        )
        .await
        {
            Ok(res) => {
                if res.is_err() {
                    error!("failed to connect to {}: {}", ep.addr, res.unwrap_err());
                    down.push(&ep);
                    continue;
                }
                up.push(&ep);
            }
            Err(_) => {
                error!("failed to connect to {}: timed out", ep.addr);
                down.push(&ep)
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
