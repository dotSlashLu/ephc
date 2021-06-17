use async_trait::async_trait;

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
        match self {
            Msg::EpDown(svc, addr) => format!("☠ Service {} endpoint {} is DOWN", svc, addr),
            Msg::EpUp(svc, addr) => format!("👍 Service {} endpoint {} is UP", svc, addr),
            Msg::AllEpDown(svc) => format!("☠☠☠ Service {} all endpoints DOWN", svc),
        }
    }
}

pub struct Alert {
    channel: Box<dyn AlertChannel + Send + Sync>,
}

impl Alert {
    pub fn new(channel: Box<dyn AlertChannel + Send + Sync>) -> Self {
        Self {
            channel
        }
    }

    pub async fn alert(&self, msg: Msg) {
        self.channel.send(msg).await
    }
}

impl std::fmt::Debug for Alert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Alert")
        .field("channel", &"".to_owned())
        .finish()
    }
}