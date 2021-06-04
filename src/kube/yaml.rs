use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::string::ToString;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressRepr {
    pub ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRepr {
    pub port: u32,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsetRepr {
    pub addresses: Vec<AddressRepr>,
    pub ports: Vec<PortRepr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetadataRepr {
    pub name: String,
    pub resourceVersion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRepr {
    pub metadata: ServiceMetadataRepr,
    pub subsets: Vec<SubsetRepr>,
    yaml: String,
}

impl FromStr for ServiceRepr {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

impl ServiceRepr {
    fn to_yaml(&self) -> Result<String> {
        let yaml = serde_yaml::to_string(self)?;
        Ok(yaml)
    }
}
