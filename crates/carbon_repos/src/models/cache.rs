//! Cache models for HTTP and version metadata.

use carbon_macro::FromRow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// HTTP response cache entry.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HTTPCache {
    /// URL (primary key).
    pub url: String,
    /// HTTP status code (snake_case in DB).
    #[serde(rename = "status_code")]
    pub status_code: i32,
    /// Response body data.
    pub data: Vec<u8>,
    /// Cache expiration time.
    pub expires_at: Option<DateTime<Utc>>,
    /// Last-Modified header value.
    pub last_modified: Option<String>,
    /// ETag header value.
    pub etag: Option<String>,
}

/// Active download tracking.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "snake_case")]
pub struct ActiveDownload {
    /// Download URL (primary key).
    pub url: String,
    /// Unique file identifier.
    pub file_id: String,
}

/// Minecraft version info cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfoCache {
    /// Minecraft version ID (primary key).
    pub id: String,
    /// Last update timestamp.
    pub last_updated_at: DateTime<Utc>,
    /// Serialized version info.
    pub version_info: Vec<u8>,
}

/// Partial version info cache (modloader versions).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PartialVersionInfoCache {
    /// ID in format "modloaderName-modloaderVersion" (primary key).
    pub id: String,
    /// Last update timestamp.
    pub last_updated_at: DateTime<Utc>,
    /// Serialized partial version info.
    pub partial_version_info: Vec<u8>,
}

/// LWJGL library metadata cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LwjglMetaCache {
    /// Cache ID (primary key).
    pub id: String,
    /// Last update timestamp.
    pub last_updated_at: DateTime<Utc>,
    /// Serialized LWJGL data.
    pub lwjgl: Vec<u8>,
}

/// Minecraft assets metadata cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AssetsMetaCache {
    /// Cache ID (primary key).
    pub id: String,
    /// Last update timestamp.
    pub last_updated_at: DateTime<Utc>,
    /// Serialized assets index.
    pub assets_index: Vec<u8>,
}
