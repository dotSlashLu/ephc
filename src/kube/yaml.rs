use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressRepr {
    pub ip: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortRepr {
    pub port: u32,
    pub protocol: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubsetRepr {
    pub addresses: Vec<AddressRepr>,
    pub ports: Vec<PortRepr>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceMetadataRepr {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceRepr {
    pub metadata: ServiceMetadataRepr,
    pub subsets: Vec<SubsetRepr>,
}

impl FromStr for ServiceRepr {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}