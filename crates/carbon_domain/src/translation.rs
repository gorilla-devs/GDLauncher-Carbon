use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Clone, PartialEq)]
pub struct Translation {
    pub key: &'static str,
    pub params: HashMap<&'static str, String>,
}

impl Display for Translation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(\"{}\" {{ ", self.key)?;

        for (param, val) in &self.params {
            write!(f, "\"{param}\": \"{val}\", ")?;
        }

        write!(f, "}})")
    }
}

#[macro_export]
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
    ([$key:expr] { $($param:literal : $value:expr),+ $(,)? }) => {
        $crate::translation::Translation {
            key: $key,
            params: ::std::collections::HashMap::from([
                $(($param, $value)),+
            ]),
        }
    };
}
