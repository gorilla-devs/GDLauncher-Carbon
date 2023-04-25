use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Translation {
    pub key: &'static str,
    pub params: HashMap<&'static str, String>,
}
