//! Java and JavaProfile models.

use carbon_macro::FromRow;
use rusqlite::Row;
use serde::{Deserialize, Serialize};

/// Java installation information.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Java {
    /// Unique identifier (UUID).
    pub id: String,
    /// Path to the Java executable.
    pub path: String,
    /// Major version number (e.g., 8, 11, 17, 21).
    pub major: i32,
    /// Full version string (e.g., "17.0.2").
    pub full_version: String,
    /// Java type (e.g., "jre", "jdk").
    #[serde(rename = "type")]
    pub java_type: String,
    /// Operating system.
    pub os: String,
    /// Architecture (e.g., "x64", "arm64").
    pub arch: String,
    /// Vendor name (e.g., "Adoptium", "Oracle").
    pub vendor: String,
    /// Whether this Java installation is valid/usable.
    pub is_valid: bool,
}

/// Named Java profile for associating Java versions with instances.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct JavaProfile {
    /// Profile name (primary key).
    pub name: String,
    /// Whether this is a system-managed profile.
    pub is_system_profile: bool,
    /// Associated Java installation ID (optional).
    pub java_id: Option<String>,
}

/// Java profile with its associated Java installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaProfileWithJava {
    pub profile: JavaProfile,
    pub java: Option<Java>,
}

/// Result type for JavaProfile with optional Java path.
///
/// This is a typed JOIN result used by `ListJavaProfilesWithJavaPath` query,
/// avoiding fragile column index access (like `row.get(4)`).
#[derive(Debug, Clone)]
pub struct JavaProfileWithPath {
    /// Profile name.
    pub name: String,
    /// Whether this is a system-managed profile.
    pub is_system_profile: bool,
    /// Associated Java installation ID (optional).
    pub java_id: Option<String>,
    /// Path to the Java executable (from joined Java table).
    pub java_path: Option<String>,
}

impl JavaProfileWithPath {
    /// Creates a JavaProfileWithPath from a database row.
    ///
    /// Expects the query to use column alias `java_path` for the joined path.
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            name: row.get("name")?,
            is_system_profile: row.get("isSystemProfile")?,
            java_id: row.get("javaId")?,
            // Use .ok() for optional LEFT JOIN column - returns None if NULL
            java_path: row.get("java_path").ok(),
        })
    }
}
