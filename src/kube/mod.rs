use crate::error::Result;
use log::{debug, error, warn};
use std::{net::SocketAddr, process::Command, str::FromStr, sync::Arc};
use tokio::sync::RwLock;

mod yaml;
use yaml::*;

#[derive(Debug, Clone)]
pub(crate) enum ServiceKind {
    TCP,
    UDP,
}

impl FromStr for ServiceKind {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "TCP" => Ok(Self::TCP),
            "UDP" => Ok(Self::UDP),
            _ => Err(Self::Err::new("unknown service type")),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum EndpointStatus {
    Healthy,
    Removed,
}

#[derive(Debug, Clone)]
pub(crate) struct Endpoint {
    pub addr: SocketAddr,
    pub status: EndpointStatus,
    pub counter_up: u64,
    pub counter_down: u64,
}

impl Endpoint {
    pub fn reset_counter(&mut self) -> &Self {
        self.counter_up = 0;
        self.counter_down = 0;
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Service {
    pub name: String,
    pub kind: ServiceKind,
    pub endpoints: Vec<Endpoint>,
    pub yaml: String,
}

impl Service {
    // construct a Service from yaml
    fn new(yml_str: String) -> Result<Option<Service>> {
        let svc = serde_yaml::from_str::<ServiceRepr>(&yml_str)?;
        let subsets: Vec<SubsetRepr> = svc.subsets;
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
                        status: EndpointStatus::Healthy,
                        counter_up: 0,
                        counter_down: 0,
                    };
                    eps.push(ep);
                }
            }
        }
        if eps.len() == 0 {
            return Ok(None);
        }

        Ok(Some(Service {
            name: svc.metadata.name,
            kind: ServiceKind::TCP,
            endpoints: eps,
            yaml: yml_str,
        }))
    }
}

fn exec(cmdline: &str) -> Result<String> {
    let mut cmd = Command::new("bash");
    let cmd = cmd.arg("-c").arg(cmdline);

    let status = cmd.status()?;
    debug!("command status: {:?}", status);
    let output = cmd.output().expect("failed to execute process");

    if !status.success() {
        let err = String::from_utf8_lossy(&output.stderr[..]);
        error!("failed to run cmd: {}", err);
        return Err(crate::error::Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            err,
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout[..]).to_string();
    Ok(stdout)
}

pub(crate) fn get_svcs() -> Result<Vec<Arc<RwLock<Service>>>> {
    let names = get_svc_names()?;
    let mut svcs = Vec::<Arc<RwLock<Service>>>::new();
    for n in names {
        let svc = get_svc(n)?;
        if svc.is_none() {
            continue;
        }
        svcs.push(Arc::new(RwLock::new(svc.unwrap())))
    }
    Ok(svcs)
}

fn get_svc_names() -> Result<Vec<String>> {
    let stdout = exec("set -eo pipefail; kubectl get svc | grep ClusterIP | gawk '{print $1}'")?;
    let lines: Vec<String> = stdout.lines().map(|el| el.to_owned()).collect();
    Ok(lines)
}

fn get_svc(svc_name: String) -> Result<Option<Service>> {
    let yml_str = exec(&format!(
        "set -eo pipefail; kubectl get ep {} -o yaml",
        svc_name
    ))?;
    Service::new(yml_str)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    const YML_STR: &str = "
apiVersion: v1
kind: Endpoints
metadata:
  creationTimestamp: 2019-03-20T07:23:28Z
  name: account
  namespace: default
  resourceVersion: \"82479279\"
  selfLink: /api/v1/namespaces/default/endpoints/account
  uid: 0ec10531-4ae1-11e9-9c9c-f86eee307061
subsets:
- addresses:
  - ip: 172.16.61.84
  - ip: 172.16.61.85
  - ip: 172.16.61.86
  - ip: 172.16.61.87
  - ip: 172.16.61.88
  - ip: 172.16.61.90
  ports:
  - name: port80
    port: 31000
    protocol: TCP
  - name: port82
    port: 31002
    protocol: TCP
  - name: port81
    port: 31001
    protocol: TCP";

    #[test]
    fn get_svc_names() {
        super::get_svc_names();
    }

    #[test]
    fn service_from_str() {
        let svc = super::ServiceRepr::from_str(YML_STR);
        println!("{:?}", svc);
    }

    #[test]
    fn service_new() {
        let svc = super::Service::new(String::from(YML_STR));
        println!("{:?}", svc);
    }
}
