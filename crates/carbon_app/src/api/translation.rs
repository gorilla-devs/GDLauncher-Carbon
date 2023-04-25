use std::collections::HashMap;
use carbon_domain::translation as domain;
use rspc::Type;
use serde::Serialize;

#[derive(Type, Serialize, Clone, PartialEq)]
pub struct Translation {
    pub key: &'static str,
    pub params: HashMap<&'static str, String>,
}

impl From<domain::Translation> for Translation {
    fn from(value: domain::Translation) -> Self {
        Self {
            key: value.key,
            params: value.params,
        }
    }
}
