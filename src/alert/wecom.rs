use reqwest;
use async_trait::async_trait;

use super::{AlertChannel, Msg};

pub struct WeCom {
    http: reqwest::Client,
    url: &'static str,
}

impl WeCom {
    pub fn new(url: &'static str) -> Self {
        Self {
            http: reqwest::Client::new(),
            url
        }
    }
}

#[async_trait]
impl AlertChannel for WeCom {
    async fn send(&self, msg: Msg) {
        let msg = format!(r#"{{
            "msgtype": "text",
            "text": {{ 
                "content":"{}"
            }}
        }}"#, msg.to_string());
        let res = self.http.post(self.url).body(msg).send().await;
    }
}