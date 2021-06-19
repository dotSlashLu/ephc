use async_trait::async_trait;
use log::error;

pub mod wecom;

#[async_trait]
pub trait AlertChannel {
    async fn send(&self, msg: Msg);
}

pub enum Msg {
    // service name, ep addr
    EpDown(String, String),
    // service name, ep addr
    EpUp(String, String),
    // service name
    AllEpDown(String),
}

impl Msg {
    fn to_string(&self) -> String {
        let default_cluster_name = &"unknown".to_owned();
        let cluster_name = crate::CFG
            .cluster_name
            .as_ref()
            .unwrap_or(default_cluster_name);
        match self {
            Msg::EpDown(svc, addr) => {
                format!(
                    "☠ ENDPOINT DOWN\nCluster: {}\nService: {}\nendpoint: {}",
                    cluster_name, svc, addr
                )
            }
            Msg::EpUp(svc, addr) => {
                format!(
                    "👍 ENDPOINT UP\nCluster: {}\nService: {}\nendpoint: {}",
                    cluster_name, svc, addr
                )
            }
            Msg::AllEpDown(svc) => format!(
                "☠☠☠ ALL ENDPOINTS DOWN\nCluster: {}\nService: {}",
                cluster_name, svc
            ),
        }
    }
}

pub struct Alert {
    channel: Option<Box<dyn AlertChannel + Send + Sync>>,
}

impl Alert {
    // pub fn new(channel: Option<Box<dyn AlertChannel + Send + Sync>>) -> Self {
    //     Self { channel }
    // }

    pub fn from_url_scheme(url: &Option<String>) -> Self {
        if url.is_none() {
            return Self { channel: None };
        }
        let url = url.as_ref().unwrap();
        if url == "" {
            return Self { channel: None };
        }
        let mut url_parts = url.split("://");
        if let Some(scheme) = url_parts.nth(0) {
            let realurl: String = url_parts.collect::<Vec<&str>>().join("://");
            return match scheme {
                "wecom" => {
                    let channel = wecom::WeCom::new(realurl);
                    Self {
                        channel: Some(Box::new(channel)),
                    }
                }
                _ => {
                    error!("unknown alert channel {}", scheme);
                    return Self { channel: None };
                }
            };
        } else {
            error!("Invalid alert url: {}, see help", url);
            return Self { channel: None };
        }
    }

    pub async fn alert(&self, msg: Msg) {
        if self.channel.is_none() {
            return;
        }
        self.channel.as_ref().unwrap().send(msg).await
    }
}

impl std::fmt::Debug for Alert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Alert")
            .field("channel", &"".to_owned())
            .finish()
    }
}

impl std::default::Default for Alert {
    fn default() -> Self {
        Self { channel: None }
    }
}
