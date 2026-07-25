use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GDLPack Manifest - format version 1
/// A minimal, hash-based modpack distribution format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    //=== FORMAT METADATA ===
    /// Schema version, always 1 for this format
    pub format_version: u32,

    //=== PACK IDENTITY ===
    /// Human-readable pack name
    pub name: String,
    /// Semantic version (e.g., "1.0.0", "2.1.0-beta.1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Pack summary/tagline (short, one line)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Pack author or team name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    //=== TIMESTAMPS ===
    /// When this pack was created/exported
    pub created_at: DateTime<Utc>,

    //=== VISUAL ASSETS (embedded in archive) ===
    /// Path to icon within archive (e.g., ".gdl/icon.png")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    //=== GAME DEPENDENCIES ===
    pub dependencies: GameDependencies,

    //=== ENTRIES ===
    /// Platform files and optional override entries
    pub entries: Vec<PackFile>,

    //=== OVERRIDES ===
    /// Directory containing raw files to copy (default: "overrides")
    #[serde(default = "default_overrides")]
    pub overrides: String,
    /// Optional server-specific overrides directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_overrides: Option<String>,
    /// Optional client-specific overrides directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_overrides: Option<String>,

    //=== SOURCE REFERENCE ===
    /// Reference to original modpack (if this is a derivative)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ModpackSource>,
}

fn default_overrides() -> String {
    "overrides".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDependencies {
    /// Minecraft version (required)
    pub minecraft: String,

    /// Modloader requirements (can have multiple, e.g., Fabric + Quilt compatible)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modloaders: Vec<ModloaderDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModloaderDependency {
    /// Modloader type
    #[serde(rename = "type")]
    pub type_: ModloaderType,
    /// Version requirement (exact version or semver range)
    pub version: String,
    /// Is this the primary/recommended modloader?
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModloaderType {
    Forge,
    Neoforge,
    Fabric,
    Quilt,
}

/// A manifest entry - either a required platform file or an optional feature
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PackFile {
    /// Required platform file resolved via hash from CurseForge/Modrinth APIs
    Platform(PlatformFile),
    /// Optional feature that user can skip - can include platform files and/or override paths
    Optional(OptionalFeature),
}

/// Required platform file that can be resolved via hash from mod platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFile {
    /// Hashes for platform resolution and integrity verification
    pub hashes: FileHashes,
}

/// Optional feature - a group of platform files and/or override paths that user can skip.
/// Use this to bundle related optional content (e.g., shader mod + shader configs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionalFeature {
    /// Description of this optional feature
    pub description: String,
    /// Optional platform files (resolved via hash)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<FileHashes>,
    /// Optional override paths (files/folders in overrides/)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub override_paths: Vec<String>,
}

/// Folds a hex digest to lowercase as it is read.
///
/// A gdlpack is an interchange format, so its hashes are written by whatever
/// exporter produced the pack, and hex case carries no meaning: `AB12` and
/// `ab12` are the same digest. Consumers both compare these strings and key maps
/// by them (the Modrinth resolution map is keyed by the hash sent in the query,
/// then looked up again by this field), so a digest that arrives in a different
/// case than the platform reports it would match one use and miss the other.
/// Folding once here leaves every downstream use working on one casing.
///
/// ASCII-only on purpose: a hex digest is ASCII, and full Unicode folding would
/// let a locale-specific rule alter a character it should leave alone.
fn deserialize_lowercase_hex<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let hex = String::deserialize(deserializer)?;
    Ok(hex.to_ascii_lowercase())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHashes {
    /// SHA-512 hash (Modrinth resolution + primary verification)
    #[serde(deserialize_with = "deserialize_lowercase_hex")]
    pub sha512: String,
    /// SHA-1 hash (Modrinth resolution + Java verification)
    #[serde(deserialize_with = "deserialize_lowercase_hex")]
    pub sha1: String,
    /// Murmur2 fingerprint (CurseForge resolution)
    pub murmur2: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "platform", rename_all = "camelCase")]
pub enum ModpackSource {
    /// Original pack from CurseForge
    Curseforge {
        project_id: u32,
        file_id: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    /// Original pack from Modrinth
    Modrinth {
        project_id: String,
        version_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_manifest() {
        let manifest = Manifest {
            format_version: 1,
            name: "Test Pack".to_string(),
            version: Some("1.0.0".to_string()),
            summary: Some("A test modpack".to_string()),
            author: Some("Test Author".to_string()),
            created_at: chrono::Utc::now(),
            icon: Some(".gdl/icon.png".to_string()),
            dependencies: GameDependencies {
                minecraft: "1.20.1".to_string(),
                modloaders: vec![ModloaderDependency {
                    type_: ModloaderType::Forge,
                    version: "47.2.0".to_string(),
                    primary: true,
                }],
            },
            entries: vec![
                // Required platform file
                PackFile::Platform(PlatformFile {
                    hashes: FileHashes {
                        sha512: "abc123".to_string(),
                        sha1: "def456".to_string(),
                        murmur2: 123456789,
                    },
                }),
                // Optional feature with platforms + overrides
                PackFile::Optional(OptionalFeature {
                    description: "Shader support - skip for low-end GPUs".to_string(),
                    platforms: vec![FileHashes {
                        sha512: "xyz789".to_string(),
                        sha1: "uvw012".to_string(),
                        murmur2: 987654321,
                    }],
                    override_paths: vec![
                        "config/iris".to_string(),
                        "shaderpacks/default".to_string(),
                    ],
                }),
                // Optional feature with just overrides
                PackFile::Optional(OptionalFeature {
                    description: "Hardcore difficulty preset".to_string(),
                    platforms: vec![],
                    override_paths: vec!["config/hardcore".to_string()],
                }),
            ],
            overrides: "overrides".to_string(),
            server_overrides: None,
            client_overrides: None,
            source: Some(ModpackSource::Curseforge {
                project_id: 12345,
                file_id: 67890,
                name: Some("Original Pack".to_string()),
                url: None,
            }),
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        assert!(json.contains("\"formatVersion\": 1"));
        assert!(json.contains("\"name\": \"Test Pack\""));
    }

    #[test]
    fn test_deserialize_manifest() {
        let json = r#"{
            "formatVersion": 1,
            "name": "Test Pack",
            "createdAt": "2024-01-15T10:30:00Z",
            "dependencies": {
                "minecraft": "1.20.1",
                "modloaders": [
                    { "type": "forge", "version": "47.2.0", "primary": true }
                ]
            },
            "entries": [
                {
                    "type": "platform",
                    "hashes": { "sha512": "abc", "sha1": "def", "murmur2": 123456 }
                },
                {
                    "type": "optional",
                    "description": "Shader support",
                    "platforms": [{ "sha512": "xyz", "sha1": "uvw", "murmur2": 789012 }],
                    "overridePaths": ["config/iris"]
                },
                {
                    "type": "optional",
                    "description": "Config presets",
                    "overridePaths": ["config/preset1", "config/preset2"]
                }
            ]
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.format_version, 1);
        assert_eq!(manifest.name, "Test Pack");
        assert_eq!(manifest.dependencies.minecraft, "1.20.1");
        assert_eq!(manifest.overrides, "overrides"); // default
        assert_eq!(manifest.entries.len(), 3);

        // Check required platform file
        match &manifest.entries[0] {
            PackFile::Platform(pf) => {
                assert_eq!(pf.hashes.sha512, "abc");
            }
            _ => panic!("Expected Platform file"),
        }

        // Check optional feature with platforms + overrides
        match &manifest.entries[1] {
            PackFile::Optional(of) => {
                assert_eq!(of.description, "Shader support");
                assert_eq!(of.platforms.len(), 1);
                assert_eq!(of.platforms[0].sha512, "xyz");
                assert_eq!(of.override_paths, vec!["config/iris"]);
            }
            _ => panic!("Expected Optional feature"),
        }

        // Check optional feature with just overrides
        match &manifest.entries[2] {
            PackFile::Optional(of) => {
                assert_eq!(of.description, "Config presets");
                assert!(of.platforms.is_empty());
                assert_eq!(of.override_paths, vec!["config/preset1", "config/preset2"]);
            }
            _ => panic!("Expected Optional feature"),
        }
    }

    /// A pack written by another launcher may spell its digests in upper case.
    /// Both carriers of `FileHashes` -- the required platform entry and an
    /// optional feature's `platforms` list -- must land lowercased, so the
    /// resolution map is keyed and looked up on one casing.
    #[test]
    fn uppercase_manifest_hashes_are_read_as_lowercase() {
        let json = r#"{
            "formatVersion": 1,
            "name": "Foreign Pack",
            "createdAt": "2024-01-15T10:30:00Z",
            "dependencies": {
                "minecraft": "1.20.1",
                "modloaders": [
                    { "type": "forge", "version": "47.2.0", "primary": true }
                ]
            },
            "entries": [
                {
                    "type": "platform",
                    "hashes": { "sha512": "AB12CD", "sha1": "EF34AB", "murmur2": 123456 }
                },
                {
                    "type": "optional",
                    "description": "Shader support",
                    "platforms": [{ "sha512": "DEADBEEF", "sha1": "CAFEBABE", "murmur2": 789012 }],
                    "overridePaths": []
                }
            ]
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();

        match &manifest.entries[0] {
            PackFile::Platform(pf) => {
                assert_eq!(pf.hashes.sha512, "ab12cd");
                assert_eq!(pf.hashes.sha1, "ef34ab");
                assert_eq!(pf.hashes.murmur2, 123456);
            }
            _ => panic!("Expected Platform file"),
        }

        match &manifest.entries[1] {
            PackFile::Optional(of) => {
                assert_eq!(of.platforms[0].sha512, "deadbeef");
                assert_eq!(of.platforms[0].sha1, "cafebabe");
            }
            _ => panic!("Expected Optional feature"),
        }
    }

    /// Mixed case must fold too -- a digest is not required to arrive uniformly
    /// cased, and a lowercase one must pass through untouched.
    #[test]
    fn mixed_and_lower_case_hashes_both_normalize() {
        let hashes: FileHashes =
            serde_json::from_str(r#"{ "sha512": "aB1c", "sha1": "abcd", "murmur2": 1 }"#).unwrap();

        assert_eq!(hashes.sha512, "ab1c");
        assert_eq!(hashes.sha1, "abcd");
    }
}
