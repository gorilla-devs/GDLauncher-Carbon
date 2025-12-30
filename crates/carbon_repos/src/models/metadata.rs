//! Mod file and metadata models.

use carbon_macro::FromRow;
use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};

/// Cached mod file within an instance.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ModFileCache {
    /// Unique identifier (UUID).
    pub id: String,
    /// Last update timestamp.
    pub last_updated_at: DateTime<Utc>,
    /// Associated instance ID.
    pub instance_id: i32,
    /// Mod filename.
    pub filename: String,
    /// File size in bytes.
    pub filesize: i32,
    /// Whether the mod is enabled.
    pub enabled: bool,
    /// Type of addon (mods, resourcepacks, shaders, datapacks, worlds).
    pub addon_type: String,
    /// Associated metadata ID.
    pub metadata_id: String,
}

/// Base mod metadata.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ModMetadata {
    /// Unique identifier (hash-based).
    pub id: String,
    /// Last update timestamp.
    pub last_updated_at: DateTime<Utc>,
    /// MurmurHash2 of the file.
    pub murmur2: i32,
    /// SHA-512 hash.
    pub sha512: Vec<u8>,
    /// SHA-1 hash.
    pub sha1: Vec<u8>,
    /// Mod display name.
    pub name: Option<String>,
    /// Mod ID (e.g., "modid" from mcmod.info).
    pub modid: Option<String>,
    /// Mod version.
    pub version: Option<String>,
    /// Mod description.
    pub description: Option<String>,
    /// Author names (comma-separated).
    pub authors: Option<String>,
    /// Supported modloaders (comma-separated).
    pub modloaders: String,
}

/// CurseForge-specific mod cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModCache {
    /// Metadata ID (foreign key, primary key).
    pub metadata_id: String,
    /// MurmurHash2 fingerprint.
    pub murmur2: i32,
    /// CurseForge project ID.
    pub project_id: i32,
    /// CurseForge file ID.
    pub file_id: i32,
    /// Project name.
    pub name: String,
    /// File version.
    pub version: String,
    /// URL slug.
    pub urlslug: String,
    /// Project summary.
    pub summary: String,
    /// Authors (comma-separated).
    pub authors: String,
    /// Release type (0=alpha, 1=beta, 2=stable).
    pub release_type: i32,
    /// Update paths in format "gamever,modloader,channel;...".
    pub update_paths: String,
    /// Cache timestamp.
    pub cached_at: DateTime<Utc>,
}

/// Modrinth-specific mod cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthModCache {
    /// Metadata ID (foreign key, primary key).
    pub metadata_id: String,
    /// SHA-512 hash (string).
    pub sha512: String,
    /// Modrinth project ID.
    pub project_id: String,
    /// Modrinth version ID.
    pub version_id: String,
    /// Project title.
    pub title: String,
    /// File version.
    pub version: String,
    /// URL slug.
    pub urlslug: String,
    /// Project description.
    pub description: String,
    /// Authors (comma-separated).
    pub authors: String,
    /// Release type (0=alpha, 1=beta, 2=stable).
    pub release_type: i32,
    /// Update paths in format "gamever,modloader,channel;...".
    pub update_paths: String,
    /// Filename.
    pub filename: String,
    /// Direct download URL.
    pub file_url: String,
    /// Cache timestamp.
    pub cached_at: DateTime<Utc>,
}

/// Local mod image cache.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LocalModImageCache {
    /// Metadata ID (primary key).
    pub metadata_id: String,
    /// Image data.
    pub data: Vec<u8>,
}

/// CurseForge mod image cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeModImageCache {
    /// Metadata ID (primary key).
    pub metadata_id: String,
    /// Image URL.
    pub url: String,
    /// Image data (optional).
    pub data: Option<Vec<u8>>,
    /// Whether the cache is up to date.
    pub up_to_date: i32,
}

impl CurseForgeModImageCache {
    /// Creates a CurseForgeModImageCache from a database row.
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            metadata_id: row.get("metadataId")?,
            url: row.get("url")?,
            data: row.get("data")?,
            up_to_date: row.get("upToDate")?,
        })
    }
}

/// Modrinth mod image cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthModImageCache {
    /// Metadata ID (primary key).
    pub metadata_id: String,
    /// Image URL.
    pub url: String,
    /// Image data (optional).
    pub data: Option<Vec<u8>>,
    /// Whether the cache is up to date.
    pub up_to_date: i32,
}

impl ModrinthModImageCache {
    /// Creates a ModrinthModImageCache from a database row.
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            metadata_id: row.get("metadataId")?,
            url: row.get("url")?,
            data: row.get("data")?,
            up_to_date: row.get("upToDate")?,
        })
    }
}

/// Mod file with all associated metadata (for complex join queries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFileCacheWithMetadata {
    // ModFileCache fields
    pub id: String,
    pub filename: String,
    pub filesize: i32,
    pub enabled: bool,
    pub addon_type: String,
    // ModMetadata fields
    pub metadata_id: String,
    pub murmur2: i32,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub name: Option<String>,
    pub modid: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: String,
    // CurseForge fields (optional)
    pub cf_project_id: Option<i32>,
    pub cf_file_id: Option<i32>,
    pub cf_name: Option<String>,
    pub cf_version: Option<String>,
    pub cf_urlslug: Option<String>,
    pub cf_summary: Option<String>,
    pub cf_authors: Option<String>,
    pub cf_release_type: Option<i32>,
    pub cf_update_paths: Option<String>,
    // Modrinth fields (optional)
    pub mr_project_id: Option<String>,
    pub mr_version_id: Option<String>,
    pub mr_title: Option<String>,
    pub mr_version: Option<String>,
    pub mr_urlslug: Option<String>,
    pub mr_description: Option<String>,
    pub mr_authors: Option<String>,
    pub mr_release_type: Option<i32>,
    pub mr_update_paths: Option<String>,
    pub mr_file_url: Option<String>,
}

impl ModFileCacheWithMetadata {
    /// Creates a ModFileCacheWithMetadata from a join query row.
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            filename: row.get("filename")?,
            filesize: row.get("filesize")?,
            enabled: row.get("enabled")?,
            addon_type: row.get("addonType")?,
            metadata_id: row.get("metadataId")?,
            murmur2: row.get("mm_murmur2")?,
            sha512: row.get("mm_sha512")?,
            sha1: row.get("mm_sha1")?,
            name: row.get("mm_name")?,
            modid: row.get("mm_modid")?,
            version: row.get("mm_version")?,
            description: row.get("mm_description")?,
            authors: row.get("mm_authors")?,
            modloaders: row.get("mm_modloaders")?,
            cf_project_id: row.get("cf_projectId")?,
            cf_file_id: row.get("cf_fileId")?,
            cf_name: row.get("cf_name")?,
            cf_version: row.get("cf_version")?,
            cf_urlslug: row.get("cf_urlslug")?,
            cf_summary: row.get("cf_summary")?,
            cf_authors: row.get("cf_authors")?,
            cf_release_type: row.get("cf_releaseType")?,
            cf_update_paths: row.get("cf_updatePaths")?,
            mr_project_id: row.get("mr_projectId")?,
            mr_version_id: row.get("mr_versionId")?,
            mr_title: row.get("mr_title")?,
            mr_version: row.get("mr_version")?,
            mr_urlslug: row.get("mr_urlslug")?,
            mr_description: row.get("mr_description")?,
            mr_authors: row.get("mr_authors")?,
            mr_release_type: row.get("mr_releaseType")?,
            mr_update_paths: row.get("mr_updatePaths")?,
            mr_file_url: row.get("mr_fileUrl")?,
        })
    }
}

/// Mod file with metadata and image availability flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFileCacheWithMetadataAndImages {
    // ModFileCache fields
    pub id: String,
    pub filename: String,
    pub filesize: i32,
    pub enabled: bool,
    pub addon_type: String,
    // ModMetadata fields
    pub metadata_id: String,
    pub murmur2: i32,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub name: Option<String>,
    pub modid: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: String,
    // CurseForge fields (optional)
    pub cf_project_id: Option<i32>,
    pub cf_file_id: Option<i32>,
    pub cf_name: Option<String>,
    pub cf_version: Option<String>,
    pub cf_urlslug: Option<String>,
    pub cf_summary: Option<String>,
    pub cf_authors: Option<String>,
    pub cf_release_type: Option<i32>,
    pub cf_update_paths: Option<String>,
    // Modrinth fields (optional)
    pub mr_project_id: Option<String>,
    pub mr_version_id: Option<String>,
    pub mr_title: Option<String>,
    pub mr_version: Option<String>,
    pub mr_urlslug: Option<String>,
    pub mr_description: Option<String>,
    pub mr_authors: Option<String>,
    pub mr_release_type: Option<i32>,
    pub mr_update_paths: Option<String>,
    pub mr_file_url: Option<String>,
    // Image availability flags
    pub has_local_image: bool,
    pub has_cf_image: bool,
    pub has_mr_image: bool,
}

impl ModFileCacheWithMetadataAndImages {
    /// Creates a ModFileCacheWithMetadataAndImages from a join query row.
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            filename: row.get("filename")?,
            filesize: row.get("filesize")?,
            enabled: row.get("enabled")?,
            addon_type: row.get("addonType")?,
            metadata_id: row.get("metadataId")?,
            murmur2: row.get("mm_murmur2")?,
            sha512: row.get("mm_sha512")?,
            sha1: row.get("mm_sha1")?,
            name: row.get("mm_name")?,
            modid: row.get("mm_modid")?,
            version: row.get("mm_version")?,
            description: row.get("mm_description")?,
            authors: row.get("mm_authors")?,
            modloaders: row.get("mm_modloaders")?,
            cf_project_id: row.get("cf_projectId")?,
            cf_file_id: row.get("cf_fileId")?,
            cf_name: row.get("cf_name")?,
            cf_version: row.get("cf_version")?,
            cf_urlslug: row.get("cf_urlslug")?,
            cf_summary: row.get("cf_summary")?,
            cf_authors: row.get("cf_authors")?,
            cf_release_type: row.get("cf_releaseType")?,
            cf_update_paths: row.get("cf_updatePaths")?,
            mr_project_id: row.get("mr_projectId")?,
            mr_version_id: row.get("mr_versionId")?,
            mr_title: row.get("mr_title")?,
            mr_version: row.get("mr_version")?,
            mr_urlslug: row.get("mr_urlslug")?,
            mr_description: row.get("mr_description")?,
            mr_authors: row.get("mr_authors")?,
            mr_release_type: row.get("mr_releaseType")?,
            mr_update_paths: row.get("mr_updatePaths")?,
            mr_file_url: row.get("mr_fileUrl")?,
            has_local_image: row.get::<_, i32>("has_local_image")? != 0,
            has_cf_image: row.get::<_, i32>("has_cf_image")? != 0,
            has_mr_image: row.get::<_, i32>("has_mr_image")? != 0,
        })
    }
}

/// Lightweight struct for export with CurseForge metadata.
#[derive(Debug, Clone)]
pub struct ModFileCacheWithCurseforge {
    pub id: String,
    pub filename: String,
    pub filesize: i32,
    pub cf_project_id: i32,
    pub cf_file_id: i32,
}

impl ModFileCacheWithCurseforge {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            filename: row.get("filename")?,
            filesize: row.get("filesize")?,
            cf_project_id: row.get("cf_projectId")?,
            cf_file_id: row.get("cf_fileId")?,
        })
    }
}

/// Lightweight struct for export with Modrinth metadata.
#[derive(Debug, Clone)]
pub struct ModFileCacheWithModrinth {
    pub id: String,
    pub filename: String,
    pub filesize: i32,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub mr_project_id: String,
    pub mr_version_id: String,
    pub mr_file_url: String,
}

impl ModFileCacheWithModrinth {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            filename: row.get("filename")?,
            filesize: row.get("filesize")?,
            sha512: row.get("mm_sha512")?,
            sha1: row.get("mm_sha1")?,
            mr_project_id: row.get("mr_projectId")?,
            mr_version_id: row.get("mr_versionId")?,
            mr_file_url: row.get("mr_fileUrl")?,
        })
    }
}
