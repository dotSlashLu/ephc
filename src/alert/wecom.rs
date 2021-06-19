use async_trait::async_trait;
use log::{debug, error};
use reqwest;

use super::{AlertChannel, Msg};

pub struct WeCom {
    http: reqwest::Client,
    url: String,
}

impl WeCom {
    pub fn new(url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            url,
        }
    }
}

#[async_trait]
impl AlertChannel for WeCom {
    async fn send(&self, msg: Msg) {
        let msg = format!(
            r#"{{
            "msgtype": "text",
            "text": {{ 
                "content":"{}"
            }}
        }}"#,
            msg.to_string()
        );
        match self.http.post(&self.url).body(msg).send().await {
            Err(e) => error!("failed to send alert message: {}", e),
            _ => debug!("alert message sent"),
        };
    }
}
