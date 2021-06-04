use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
#[serde(rename_all = "camelCase")]
pub struct ServiceMetadataRepr {
    pub name: String,
    pub resource_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ServiceRepr {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    pub metadata: ServiceMetadataRepr,
    pub subsets: Vec<SubsetRepr>,
    #[serde(skip)]
    yaml: String,
}

impl FromStr for ServiceRepr {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

impl ServiceRepr {
    pub fn to_yaml(&self) -> Result<String> {
        let yaml = serde_yaml::to_string(self)?;
        Ok(yaml)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_yaml() {
        let s = ServiceRepr {
            api_version: "v1".to_owned(),
            kind: "Endpoints".to_owned(),
            metadata: ServiceMetadataRepr {
                name: String::from_str("test").unwrap(),
                resource_version: String::from_str("1").unwrap(),
            },
            subsets: vec![SubsetRepr {
                addresses: vec![AddressRepr {
                    ip: String::from_str("1.1.1.1").unwrap(),
                }],
                ports: vec![
                    PortRepr {
                        port: 23,
                        protocol: String::from_str("UDP").unwrap(),
                    },
                    PortRepr {
                        port: 80,
                        protocol: String::from_str("TCP").unwrap(),
                    },
                ],
            }],
            yaml: String::from_str("s").unwrap(),
        };
        println!("{:?}", s.to_yaml());
    }
}
