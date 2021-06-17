use async_trait::async_trait;

mod wecom;

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

pub struct Alert<T: AlertChannel> {
    channel: T,
}

impl<T: AlertChannel> Alert<T> {
    pub fn new(channel: T) -> Self {
        Self {
            channel
        }
    }

    pub async fn alert(&self, msg: Msg) {
        self.channel.send(msg).await
    }
}