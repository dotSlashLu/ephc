use crate::error::Result;
use log::{debug, error, warn};
use std::io::Write;
use std::time::SystemTime;
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
pub(crate) struct Threshold {
    pub restore: u32,
    pub remove: u32,
}
#[derive(Debug, Clone)]
pub(crate) struct Counter {
    pub up: u32,
    pub down: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Endpoint {
    pub addr: SocketAddr,
    pub status: EndpointStatus,
    counter: Counter,
    threshold: Threshold,
}

impl Endpoint {
    pub fn up(&mut self) -> bool {
        match self.status {
            EndpointStatus::Removed => {
                self.counter.up += 1;
                return if self.counter.up >= self.threshold.restore {
                    true
                } else {
                    false
                };
            }
            _ => false,
        }
    }

    pub fn down(&mut self) -> bool {
        match self.status {
            EndpointStatus::Healthy => {
                self.counter.down += 1;
                return if self.counter.down >= self.threshold.remove {
                    true
                } else {
                    false
                };
            }
            _ => false,
        }
    }

    pub fn reset_counter(&mut self) -> &Self {
        self.counter.up = 0;
        self.counter.down = 0;
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Service {
    pub name: String,
    pub kind: ServiceKind,
    pub endpoints: Vec<Endpoint>,
    pub yaml: ServiceRepr,
}

impl Service {
    // construct a Service from yaml
    fn new(yml_str: String, threshold: Threshold) -> Result<Option<Service>> {
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

        let name = svc_repr.metadata.name.clone();
        Ok(Some(Service {
            name: name,
            kind: ServiceKind::TCP,
            endpoints: eps,
            yaml: svc_repr,
        }))
    }

    pub fn remove_ep(&mut self, i: usize) {
        let ep = self.endpoints[i].addr;
        debug!("should remove ep: {:?}", ep);
        let ep_ip = ep.ip();
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
    }

    pub fn restore_ep(&mut self, i: usize) {
        debug!("should restore ep: {:?}", self.endpoints[i]);
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

fn apply_svc(name: &str, yml: &str) -> Result<()> {
    let t = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(e) => {
            error!("failed to get time: {}", e);
            0
        }
    };
    let fname = format!("/tmp/ephc_{}_{}", t, name);
    let mut file = std::fs::File::create(name)?;
    file.write_all(yml.as_bytes())?;

    exec(&format!("set -eo pipefail; kubectl apply -f {}", fname))?;
    Ok(())
}

pub(crate) fn get_svcs(t: Threshold) -> Result<Vec<Arc<RwLock<Service>>>> {
    let names = get_svc_names()?;
    let mut svcs = Vec::<Arc<RwLock<Service>>>::new();
    for n in names {
        let svc = get_svc(n, t.clone())?;
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

fn get_svc(svc_name: String, t: Threshold) -> Result<Option<Service>> {
    let yml_str = exec(&format!(
        "set -eo pipefail; kubectl get ep {} -o yaml",
        svc_name
    ))?;
    Service::new(yml_str, t)
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
        let threshold = super::Threshold {
            restore: 3,
            remove: 3,
        };
        let svc = super::Service::new(String::from(YML_STR), threshold);
        println!("{:?}", svc);
    }
}
