use crate::error::Result;
use log::{error, info, warn};
use std::{net::SocketAddr, str::FromStr};

use super::endpoint::*;
use super::yaml::*;

#[derive(Debug, Clone)]
pub(crate) struct Service {
    pub name: String,
    pub endpoints: Vec<Endpoint>,
    // update this after updating k8s, if the new version got from k8s changed
    // means this service has been changed from outside and all members need
    // to be refreshed
    pub our_version: String,
    // yaml representation of the service
    pub repr: ServiceRepr,
    pub alerter: std::sync::Arc<crate::alert::Alert>,
}

impl Service {
    // construct a Service from yaml
    pub fn new(
        yml_str: String,
        threshold: Threshold,
        alerter: std::sync::Arc<crate::alert::Alert>,
    ) -> Result<Option<Self>> {
        let mut svc_repr = serde_yaml::from_str::<ServiceRepr>(&yml_str)?;
        svc_repr.yaml = yml_str;
        let subsets: &Vec<SubsetRepr> = &svc_repr.subsets;
        let mut eps = Vec::<Endpoint>::new();
        for subset in subsets {
            for port in &subset.ports {
                if port.protocol == "UDP" {
                    warn!("we don't support UDP for now");
                    continue;
                }

                for addr in &subset.addresses {
                    let addr = SocketAddr::from_str(&format!("{}:{}", addr.ip, port.port))?;
                    let ep = Endpoint {
                        addr,
                        protocol: Protocol::from_str(&port.protocol)?,
                        status: EndpointStatus::Healthy,
                        counter: Counter { up: 0, down: 0 },
                        threshold: threshold.clone(),
                    };
                    eps.push(ep);
                }
            }
        }
        if eps.len() == 0 {
            return Ok(None);
        }

        Ok(Some(Service {
            name: svc_repr.metadata.name.clone(),
            endpoints: eps,
            our_version: svc_repr.metadata.resource_version.clone(),
            repr: svc_repr,
            alerter,
        }))
    }

    // TODO: Does all eps only contain one subset?
    pub async fn remove_ep(&mut self, i: usize) -> Result<()> {
        let ep_addr = self.endpoints[i].addr.clone();
        let ep_ip = ep_addr.ip();
        info!("removing ep: {:?}", ep_addr);
        self.alerter
            .alert(crate::alert::Msg::EpDown(
                self.name.clone(),
                ep_addr.to_string(),
            ))
            .await;

        // if there're only one ep, do nothing except mark it
        if self.endpoints.len() <= 1 {
            info!(
                "{} is the only ep, do nothing except marking it unhealthy",
                ep_addr
            );
            self.endpoints[i].status = EndpointStatus::Removed;
            return Ok(());
        }

        // If the last ep is going to be removed, meaning every ep is unhealthy,
        // restore all original eps in k8s for quicker restoration
        //
        if self.repr.subsets[0].addresses.len() == 1 {
            self.alerter
                .alert(crate::alert::Msg::AllEpDown(self.name.clone()))
                .await;
            info!("all eps marked as removed, restoring all eps in k8s");
            for ep in &mut self.endpoints {
                if ep.addr.ip() == ep_ip {
                    ep.set_status(EndpointStatus::Removed);
                }
            }
            let original_repr = ServiceRepr::from_str(&self.repr.yaml)?;
            let new_version = super::apply_svc(&self.name, &original_repr.to_yaml()?)?;
            self.repr = original_repr;
            self.our_version = new_version;
            return Ok(());
        }

        for subset in &mut self.repr.subsets {
            subset.addresses.retain(|addr| {
                let ip = match std::net::IpAddr::from_str(&addr.ip) {
                    Ok(ip) => ip,
                    Err(_) => {
                        error!("failed to parse {}", addr.ip);
                        return false;
                    }
                };
                if ip == ep_ip {
                    return false;
                }
                true
            });
        }

        let yml = self.repr.to_yaml()?;
        let new_version = super::apply_svc(&self.name, &yml)?;
        self.our_version = new_version;

        // mark all eps with the same IP as removed
        for ep in &mut self.endpoints {
            if ep.addr.ip() == ep_ip {
                ep.set_status(EndpointStatus::Removed);
            }
        }

        info!("ep {} removed, new version: {:?}", ep_ip, self.our_version);
        Ok(())
    }

    // TODO: Does all eps only contain one subsets?
    pub async fn restore_ep(&mut self, i: usize) -> Result<()> {
        let ep_addr = &self.endpoints[i].addr;
        info!("restoring ep: {:?}", ep_addr);
        self.alerter
            .alert(crate::alert::Msg::EpUp(
                self.name.clone(),
                ep_addr.to_string(),
            ))
            .await;
        let ep_ip = ep_addr.ip();

        // only restore this IP from k8s when all ports of this IP are up
        let mut n_ip_eps = 0;
        let mut n_ip_eps_healthy = 0;
        for (k, ep) in self.endpoints.iter().enumerate() {
            if i == k {
                n_ip_eps += 1;
                n_ip_eps_healthy += 1;
                continue;
            }
            if ep.addr.ip() != ep_ip {
                continue;
            }
            n_ip_eps += 1;
            if ep.status == EndpointStatus::Healthy {
                n_ip_eps_healthy += 1;
            }
        }

        if n_ip_eps != n_ip_eps_healthy {
            info!(
                "only {} of {} addrs of this IP have turned healthy, \
                    won't actually restore",
                n_ip_eps_healthy, n_ip_eps
            );
            let ep = &mut self.endpoints[i];
            ep.set_status(EndpointStatus::Healthy);
            return Ok(());
        }

        // An address is marked removed but remains in k8s indicates all
        //  addresses were restored since every one of them are down.
        // When one IP turned healthy again, mark all of eps as healthy and let
        //  the next turn of probe to remove the down ones.
        //
        // TODO: Right now, when only part of the eps are up, the remaining down
        //  eps still remains in k8s
        //  But they will be removed in the next turn of probe, so this priority
        //  is low
        if self.repr.subsets[0].addresses.contains(&AddressRepr {
            ip: ep_ip.to_string(),
        }) {
            info!(
                "one of all unhealthy endpoints restored, marking all \
                endpoints healthy."
            );
            for ep in &mut self.endpoints {
                ep.set_status(EndpointStatus::Healthy);
            }
            return Ok(());
        } else {
            self.repr.subsets[0].addresses.push(AddressRepr {
                ip: ep_ip.to_string(),
            });
        }

        let yml = self.repr.to_yaml()?;
        let new_version = super::apply_svc(&self.name, &yml)?;
        self.our_version = new_version;

        let ep = &mut self.endpoints[i];
        ep.set_status(EndpointStatus::Healthy);

        info!("ep {} restored, new version: {:?}", ep_ip, self.our_version);
        Ok(())
    }
}
