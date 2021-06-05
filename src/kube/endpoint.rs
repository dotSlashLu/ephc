use std::{net::SocketAddr, str::FromStr};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Protocol {
    TCP,
    UDP,
}

impl FromStr for Protocol {
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

#[derive(Debug, Clone, Default)]
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
    pub protocol: Protocol,
    pub status: EndpointStatus,
    pub counter: Counter,
    pub threshold: Threshold,
}

impl Endpoint {
    pub fn up(&mut self) -> bool {
        match self.status {
            EndpointStatus::Removed => {
                self.counter.up += 1;
                self.counter.up >= self.threshold.restore
            }
            _ => {
                // clear previous down counter
                self.counter.down = 0;
                false
            }
        }
    }

    pub fn down(&mut self) -> bool {
        match self.status {
            EndpointStatus::Healthy => {
                self.counter.down += 1;
                self.counter.down >= self.threshold.remove
            }
            _ => {
                // clear previous up counter
                self.counter.up = 0;
                false
            }
        }
    }

    pub fn reset_counter(&mut self) -> &Self {
        self.counter.up = 0;
        self.counter.down = 0;
        self
    }
}

impl std::cmp::PartialEq for Endpoint {
    fn eq(&self, other: &Self) -> bool {
        if self.addr == other.addr && self.protocol == other.protocol {
            return true;
        }
        false
    }
}
