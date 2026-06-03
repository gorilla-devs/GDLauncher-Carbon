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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHashes {
    /// SHA-512 hash (Modrinth resolution + primary verification)
    pub sha512: String,
    /// SHA-1 hash (Modrinth resolution + Java verification)
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
}
