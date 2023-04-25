use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Translation {
    pub key: &'static str,
    pub params: HashMap<&'static str, String>,
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
}
