use std::collections::HashMap;

use carbon_domain::translation as domain;

#[derive(Debug, Clone, PartialEq)]
pub struct Translation {
    pub key: &'static str,
    pub params: HashMap<&'static str, String>,
}

impl From<Translation> for domain::Translation {
    fn from(value: Translation) -> Self {
        Self {
            key: value.key,
            params: value.params,
        }
    }
}

macro_rules! translate {
    ($key:literal) => {
        $crate::translation::Translation {
            key: $key,
            params: ::std::collections::HashMap::new(),
        }
    };
    ($key:literal { $($param:literal : $value:expr),+ $(,)? }) => {
        $crate::translation::Translation {
            key: $key,
            params: ::std::collections::HashMap::from([
                $(($param, $value)),+
            ]),
        }
    };
}

pub(crate) use translate;
