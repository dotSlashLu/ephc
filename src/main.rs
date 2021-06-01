use std::sync::Arc;
use tokio::{sync::RwLock, time::{self, Duration}};
use log::{debug, info, error};

mod error;
mod kube;
mod probe;

// how many ups will be re-added to ep
const HEALTHY_UPS: u8 = 1;
// how many downs will be removed from ep
const REMOVED_DOWNS: u8 = 3;
// ms to consider an ep is down
const CONNECT_TIMEOUT: u64 = 100;

struct Ephc {
    services: Arc<RwLock<Vec<kube::Service>>>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();

    let ephc = Ephc {
        services: Arc::new(RwLock::new(vec![]))
    };

    // TODO: take from cli
    let refresh_interval = 1;
    let probe_interval = 1000;

    let svcs = ephc.services.clone();
    let mut interval = time::interval(Duration::from_secs(refresh_interval));
    let jh_refresh = tokio::task::spawn(async move {
        loop {
            interval.tick().await;
            info!("refresh service list");
            let res = match kube::get_svcs() {
                Ok(res) => res,
                Err(e) => {
                    error!("failed to get services: {}", e);
                    vec![]
                }
            };
            *svcs.write().await = res;
            break
        }
    });

    let svcs = ephc.services.clone();
    let mut interval = time::interval(Duration::from_millis(probe_interval));
    let jh_probe = tokio::task::spawn(async move {
        loop {
            interval.tick().await;
            debug!("start probing");
            probe::probe(svcs);
            break
        }
    });
   

    jh.await;
    Ok(())
}

fn init() {
    env_logger::init();
}
