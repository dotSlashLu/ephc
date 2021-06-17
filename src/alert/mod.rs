use reqwest;
use async_trait::async_trait;

#[async_trait]
trait AlertChannel {
	async fn send(&self, msg: Msg);
}

enum Msg {
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
            Msg::EpDown(svc, addr) => format!("Service {} endpoint {} is DOWN", svc, addr),
            _ => "".to_owned()
        }
    }
}

struct WeCom {
    url: &'static str,
}

#[async_trait]
impl AlertChannel for WeCom {
    async fn send(&self, msg: Msg) {
            
    }
}