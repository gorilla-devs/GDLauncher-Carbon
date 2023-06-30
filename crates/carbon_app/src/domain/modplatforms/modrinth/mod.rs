//! Models specified in the Modrinth documentation

pub mod misc;
pub mod project;
pub mod search;
pub mod tag;
pub mod user;
pub mod version;

/// ISO 8601 UTC datetime
pub type UtcDateTime = chrono::DateTime<chrono::Utc>;

use serde::{Deserialize, Serialize};
use url::Url;

fn deserialise_optional_url<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Option<Url>, D::Error> {
    use serde::de::{Error, Unexpected};
    use std::borrow::Cow;

    let intermediate = <Option<Cow<'de, str>>>::deserialize(de)?;
    match intermediate.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => Url::parse(s).map_or_else(
            |err| {
                Err(Error::invalid_value(
                    Unexpected::Str(s),
                    &err.to_string().as_str(),
                ))
            },
            |ok| Ok(Some(ok)),
        ),
    }
}
