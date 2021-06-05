use crate::error::Result;
use log::{debug, error, warn};
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
    pub yaml: ServiceRepr,
}

impl Service {
    // construct a Service from yaml
    pub fn new(yml_str: String, threshold: Threshold) -> Result<Option<Self>> {
        let svc_repr = serde_yaml::from_str::<ServiceRepr>(&yml_str)?;
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
            yaml: svc_repr,
        }))
    }

    // remove ep and return new version
    pub fn remove_ep(&mut self, i: usize) -> Result<()> {
        let mut ep = &mut self.endpoints[i];
        let ep_addr = ep.addr;
        debug!("should remove ep: {:?}", ep_addr);
        ep.status = EndpointStatus::Removed;
        let ep_ip = ep_addr.ip();
        for subset in &mut self.yaml.subsets {
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
        let yml = self.yaml.to_yaml()?;
        super::apply_svc(&self.name, &yml)?;
        let yml = super::get_svc_repr(&self.name)?;
        let new_svc = super::yaml::ServiceRepr::from_str(&yml)?;
        self.our_version = new_svc.metadata.resource_version;
        debug!("ep removed, new version: {:?}", self.our_version);
        Ok(())
    }

    pub fn restore_ep(&mut self, i: usize) {
        debug!("should restore ep: {:?}", self.endpoints[i]);
    }
}
