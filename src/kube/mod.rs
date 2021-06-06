use crate::error::Result;
use log::error;
use std::io::Write;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

mod endpoint;
mod service;
pub mod yaml;

pub use endpoint::*;
pub use service::*;

fn exec(cmdline: &str) -> Result<String> {
    let mut cmd = Command::new("bash");
    let cmd = cmd.arg("-c").arg(cmdline);

    let output = cmd
        .stdout(Stdio::piped())
        .output()
        .expect("failed to execute process");

    let status = cmd.status()?;
    if !status.success() {
        let err = String::from_utf8_lossy(&output.stderr[..]);
        error!(
            "failed to run cmd: exit status: {}, stderr: {}",
            status, err
        );
        return Err(crate::error::Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            err,
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout[..]).to_string();
    Ok(stdout)
}

fn apply_svc(name: &str, yml: &str) -> Result<String> {
    let t = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(e) => {
            error!("failed to get time: {}", e);
            0
        }
    };
    let fname = format!("/tmp/ephc_{}_{}", t, name);
    let mut file = std::fs::File::create(&fname)?;
    file.write_all(yml.as_bytes())?;

    exec(&format!("set -eo pipefail; kubectl apply -f {}", &fname))?;

    // get new version after apply
    // all errors are not propagated
    let yml = match get_svc_repr(name) {
        Err(e) => {
            error!("failed to get service repr for {}: {}", name, e);
            return Ok("0".to_owned());
        }
        Ok(yml) => yml,
    };
    let new_svc = match yaml::ServiceRepr::from_str(&yml) {
        Ok(repr) => repr,
        Err(e) => {
            error!("failed to parse yaml for repr {}: {}", name, e);
            return Ok("0".to_owned());
        }
    };
    Ok(new_svc.metadata.resource_version)
}

pub(crate) fn get_svcs(
    allow: &Option<Vec<String>>,
    block: &Option<Vec<String>>,
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

fn get_svc_names(block: &Option<Vec<String>>) -> Result<Vec<String>> {
    let default_block_list = &vec!["kubernetes".to_owned()];
    let block = match block {
        Some(l) => l,
        None => default_block_list,
    };

    let stdout = exec("set -eo pipefail; kubectl get svc | grep ClusterIP | gawk '{print $1}'")?;
    let mut lines: Vec<String> = stdout.lines().map(|el| el.to_owned()).collect();
    lines.retain(|el| !block.contains(el));
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
          selfLink: /api/v1/namespaces/default/endpoints/ephc-test
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
        super::get_svc_names(&None).unwrap();
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
