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
}

impl Service {
    // construct a Service from yaml
    pub fn new(yml_str: String, threshold: Threshold) -> Result<Option<Self>> {
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
        }))
    }

    // TODO: does all eps only contain one subset?
    pub fn remove_ep(&mut self, i: usize) -> Result<()> {
        let ep_addr = &self.endpoints[i].addr;
        info!("removing ep: {:?}", ep_addr);

        // if there're only one ep, do nothing except mark it
        if self.endpoints.len() <= 1 {
            info!(
                "{} is the only ep, do nothing except marking it unhealthy",
                ep_addr
            );
            self.endpoints[i].status = EndpointStatus::Removed;
            return Ok(());
        }

        // if the last ep is going to be removed, meaning every ep is unhealthy,
        // restore all original eps in k8s for quicker restoration
        //
        // TODO: right now, when only part of the eps are up, the remaining down
        //  eps still remains in k8s
        if self.repr.subsets[0].addresses.len() == 1 {
            info!("all eps marked as removed, restoring all eps in k8s");
            self.endpoints[i].status = EndpointStatus::Removed;
            let mut original_repr = ServiceRepr::from_str(&self.repr.yaml)?;
            original_repr.unset_api_version();
            super::apply_svc(&self.name, &original_repr.to_yaml()?)?;
            return Ok(());
        }

        let ep_ip = ep_addr.ip();
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
        super::apply_svc(&self.name, &yml)?;
        let yml = super::get_svc_repr(&self.name)?;
        let new_svc = super::yaml::ServiceRepr::from_str(&yml)?;
        self.our_version = new_svc.metadata.resource_version;

        let ep = &mut self.endpoints[i];
        ep.reset_counter();
        ep.status = EndpointStatus::Removed;

        info!(
            "ep {} removed, new version: {:?}",
            ep.addr, self.our_version
        );
        Ok(())
    }

    // TODO: does all eps only contain one subsets
    pub fn restore_ep(&mut self, i: usize) -> Result<()> {
        let mut ep = &mut self.endpoints[i];
        let ep_addr = ep.addr;
        info!("restoring ep: {:?}", ep_addr);

        let ep_ip = ep_addr.ip();
        // ep was marked down without changing repr, just mark it will do
        if self.repr.subsets[0].addresses.contains(&AddressRepr {
            ip: ep_ip.to_string(),
        }) {
            ep.reset_counter();
            ep.status = EndpointStatus::Healthy;
            info!("ep {} restored without changing k8s", ep_ip);
            return Ok(());
        }

        self.repr.subsets[0].addresses.push(AddressRepr {
            ip: ep_ip.to_string(),
        });

        let yml = self.repr.to_yaml()?;
        super::apply_svc(&self.name, &yml)?;
        let yml = super::get_svc_repr(&self.name)?;
        let new_svc = super::yaml::ServiceRepr::from_str(&yml)?;
        self.our_version = new_svc.metadata.resource_version;

        ep.reset_counter();
        ep.status = EndpointStatus::Healthy;

        info!("ep {} restored, new version: {:?}", ep_ip, self.our_version);
        Ok(())
    }
}
