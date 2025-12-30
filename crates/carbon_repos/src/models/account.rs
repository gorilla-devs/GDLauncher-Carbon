//! Account and Skin models.

use carbon_macro::FromRow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Microsoft account for Minecraft authentication.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// Unique identifier (UUID).
    pub uuid: String,
    /// Display username.
    pub username: String,
    /// OAuth access token for Minecraft API.
    pub access_token: Option<String>,
    /// Token expiration time.
    pub token_expires: Option<DateTime<Utc>>,
    /// Microsoft refresh token for token renewal.
    pub ms_refresh_token: Option<String>,
    /// ID token from authentication.
    pub id_token: Option<String>,
    /// Last time this account was used.
    pub last_used: DateTime<Utc>,
    /// Associated skin ID.
    pub skin_id: Option<String>,
}

/// Player skin data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
    /// Skin identifier.
    pub id: String,
    /// Raw skin image data.
    pub skin: Vec<u8>,
}
