use log::{debug, error, info};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::RwLock,
    time::{self, Duration},
};

mod error;
mod kube;
mod probe;

// how many ups will be re-added to ep
const HEALTHY_UPS: u8 = 1;
// how many downs will be removed from ep
const REMOVED_DOWNS: u8 = 3;
// ms to consider an ep is down
const CONNECT_TIMEOUT: u64 = 100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();

    let services = Arc::new(RwLock::new(
        HashMap::<String, Arc<RwLock<kube::Service>>>::new(),
    ));

    // TODO: take from cli
    let refresh_interval = 1;
    let probe_interval = 1000;

    let svcs = services.clone();
    let mut interval = time::interval(Duration::from_secs(refresh_interval));
    let jh_refresh = tokio::task::spawn(async move {
        loop {
            interval.tick().await;
            info!("refresh service list");
            let t = kube::Threshold {
                restore: 3,
                remove: 3,
            };
            let res = match kube::get_svcs(Some(vec!["ephc-test"]), None, t) {
                Ok(res) => res,
                Err(e) => {
                    error!("failed to get services: {}", e);
                    vec![]
                }
            };
            let mut svcs_writer = svcs.write().await;
            for svc in res {
                let svc_clone = svc.clone();
                let svc_reader = svc_clone.read().await;
                match svcs_writer.get(&svc_reader.name) {
                    Some(old) => {
                        let old = old.clone();
                        let old_reader = old.read().await;
                        if old_reader.our_version == svc_reader.our_version {
                            debug!("service {} not changed", svc_reader.name);
                            continue;
                        }
                        debug!(
                            "service {} changed from outside, replacing",
                            svc_reader.name
                        );
                        svcs_writer.insert(svc_reader.name.clone(), svc);
                    }
                    None => {
                        svcs_writer.insert(svc_reader.name.clone(), svc);
                    }
                }
            }
        }
    });

    let svcs = services.clone();
    let mut interval = time::interval(Duration::from_millis(probe_interval));
    let jh_probe = tokio::task::spawn(async move {
        loop {
            interval.tick().await;
            debug!("start probing");
            let svcs = svcs.clone();
            probe::probe(svcs).await;
            debug!("finished probing");
        }
    });

    jh_refresh.await.unwrap();
    jh_probe.await.unwrap();

    let services = services.read().await;
    for svc in services.values() {
        let svc = svc.read().await;
        debug!("svc after hc: {:?}", svc);
    }

    Ok(())
}

fn init() {
    env_logger::init();
}
