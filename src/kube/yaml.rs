use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddressRepr {
    pub ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRepr {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
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
    #[serde(skip_serializing)]
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
    pub yaml: String,
}

impl FromStr for ServiceRepr {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match serde_yaml::from_str::<Self>(s) {
            Ok(mut repr) => {
                repr.yaml = s.to_owned();
                Ok(repr)
            }
            Err(e) => Err(e),
        }
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
                name: "test".to_owned(),
                resource_version: "1".to_owned(),
            },
            subsets: vec![SubsetRepr {
                addresses: vec![AddressRepr {
                    ip: "1.1.1.1".to_owned(),
                }],
                ports: vec![
                    PortRepr {
                        name: Some("23".to_owned()),
                        port: 23,
                        protocol: "UDP".to_owned(),
                    },
                    PortRepr {
                        name: None,
                        port: 80,
                        protocol: "TCP".to_owned(),
                    },
                ],
            }],
            yaml: "s".to_owned(),
        };
        println!("{:?}", s.to_yaml().unwrap());
    }
}
