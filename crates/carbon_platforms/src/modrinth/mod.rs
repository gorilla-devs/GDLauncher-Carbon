//! Models specified in the Modrinth documentation

pub mod project;
pub mod responses;
pub mod search;
pub mod tag;
pub mod user;
pub mod version;

/// ISO 8601 UTC datetime
pub type UtcDateTime = chrono::DateTime<chrono::Utc>;

use serde::{Deserialize, Serialize};
