//! Modpack cache models.

use carbon_macro::FromRow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// CurseForge modpack cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModpackCache {
    /// CurseForge project ID.
    pub project_id: i32,
    /// CurseForge file ID.
    pub file_id: i32,
    /// Modpack name.
    pub modpack_name: String,
    /// Version name.
    pub version_name: String,
    /// URL slug.
    pub url_slug: String,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Modrinth modpack cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthModpackCache {
    /// Modrinth project ID.
    pub project_id: String,
    /// Modrinth version ID.
    pub version_id: String,
    /// Modpack name.
    pub modpack_name: String,
    /// Version name.
    pub version_name: String,
    /// URL slug.
    pub url_slug: String,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// CurseForge modpack image cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModpackImageCache {
    /// CurseForge project ID.
    pub project_id: i32,
    /// CurseForge file ID.
    pub file_id: i32,
    /// Image URL.
    pub url: String,
    /// Image data (optional).
    pub data: Option<Vec<u8>>,
}

/// Modrinth modpack image cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthModpackImageCache {
    /// Modrinth project ID.
    pub project_id: String,
    /// Modrinth version ID.
    pub version_id: String,
    /// Image URL.
    pub url: String,
    /// Image data (optional).
    pub data: Option<Vec<u8>>,
}

/// CurseForge modpack with image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeModpackCacheWithImage {
    pub modpack: CurseForgeModpackCache,
    pub image: Option<CurseForgeModpackImageCache>,
}

/// Modrinth modpack with image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthModpackCacheWithImage {
    pub modpack: ModrinthModpackCache,
    pub image: Option<ModrinthModpackImageCache>,
}
