use lazy_static::lazy_static;
use log::{debug, error, info};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::RwLock,
    time::{self, Duration},
};

lazy_static! {
    static ref CFG: cmd::AppOpt = cmd::init();
}

mod alert;
mod cmd;
mod error;
mod kube;
mod probe;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let alert = Arc::new(alert::Alert::from_url_scheme(&CFG.alert_channel));

    let services = Arc::new(RwLock::new(
        HashMap::<String, Arc<RwLock<kube::Service>>>::new(),
    ));

    let svcs = services.clone();
    let mut interval = time::interval(Duration::from_secs(CFG.refresh_interval));
    let opt_clone = CFG.clone();
    let jh_refresh = tokio::task::spawn(async move {
        loop {
            interval.tick().await;
            info!("refresh service list");
            let t = kube::Threshold {
                restore: opt_clone.restore,
                remove: opt_clone.remove,
            };
            let res = match kube::get_svcs(
                &opt_clone.allow_list,
                &opt_clone.block_list,
                t,
                alert.clone(),
            ) {
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
                        let version = svc_reader.our_version.parse::<u64>().unwrap_or_else(|e| {
                            debug!("failed to parse new resourceVersion: {}", e);
                            0
                        });
                        let old_version =
                            old_reader.our_version.parse::<u64>().unwrap_or_else(|e| {
                                debug!("failed to parse old resourceVersion: {}", e);
                                0
                            });

                        if version == old_version {
                            debug!("service {} not changed", svc_reader.name);
                            continue;
                        }
                        if version < old_version {
                            error!(
                                "service {} got version {} under our version {}",
                                svc_reader.name, version, old_version
                            );
                            continue;
                        }

                        info!(
                            "service {} changed from outside, replacing",
                            svc_reader.name
                        );
                        debug!(
                            "new version: {}, our version: {}",
                            svc_reader.our_version, old_reader.our_version
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
    let probe_interval = CFG.probe_interval;
    let mut interval = time::interval(Duration::from_millis(probe_interval));
    let jh_probe = tokio::task::spawn(async move {
        loop {
            interval.tick().await;
            debug!("start probing");
            let svcs = svcs.clone();
            probe::probe(svcs, CFG.connection_timeout).await;
            debug!("finished probing");
        }
    });

    jh_refresh.await.unwrap();
    jh_probe.await.unwrap();

    Ok(())
}
