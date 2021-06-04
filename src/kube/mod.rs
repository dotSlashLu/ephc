use crate::error::Result;
use log::{debug, error};
use std::time::SystemTime;
use std::{io::Write, str::FromStr};
use std::{process::Command, sync::Arc};
use tokio::sync::RwLock;

mod endpoint;
mod service;
mod yaml;

pub use endpoint::*;
pub use service::*;

use self::yaml::ServiceRepr;

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

pub(crate) fn get_svcs(
    allow: Option<Vec<&'static str>>,
    block: Option<Vec<&'static str>>,
    t: Threshold,
) -> Result<Vec<Arc<RwLock<Service>>>> {
    let names: Vec<String> = match allow {
        Some(allow) => allow.iter().map(|n| (*n).to_owned()).collect(),
        None => get_svc_names(block)?,
    };
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

fn get_svc_names(block: Option<Vec<&'static str>>) -> Result<Vec<String>> {
    let block = match block {
        Some(l) => l,
        None => vec!["kubernetes"],
    };

    let stdout = exec("set -eo pipefail; kubectl get svc | grep ClusterIP | gawk '{print $1}'")?;
    let mut lines: Vec<String> = stdout.lines().map(|el| el.to_owned()).collect();
    lines.retain(|el| !block.contains(&&el[..]));
    Ok(lines)
}

fn get_svc_repr(svc_name: &str) -> Result<String> {
    exec(&format!(
        "set -eo pipefail; kubectl get ep {} -o yaml",
        svc_name
    ))
}

fn get_svc(svc_name: String, t: Threshold) -> Result<Option<Service>> {
    let yml_str = get_svc_repr(&svc_name)?;
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
        super::get_svc_names(None);
    }

    #[test]
    fn service_from_str() {
        let svc = super::yaml::ServiceRepr::from_str(YML_STR);
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
