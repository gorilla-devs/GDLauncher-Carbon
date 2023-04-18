use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum EventName {
    AppClosed,
}

impl ToString for EventName {
    fn to_string(&self) -> String {
        match self {
            EventName::AppClosed => "app_closed".to_string(),
        }
    }
}

impl TryFrom<String> for EventName {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match &*value {
            "app_closed" => Ok(EventName::AppClosed),
            _ => Err(format!("Unknown event name: {}", value)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub name: EventName,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pageview {
    pub path: String,
}
