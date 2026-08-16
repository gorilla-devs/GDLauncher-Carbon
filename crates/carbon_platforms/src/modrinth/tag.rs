//! Models related to tags

use super::*;

/// A category that projects of `project_type` specify
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Category {
    /// An SVG icon for the category
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    pub icon: Option<String>,
    pub name: String,
    /// The project type this category is applicable to
    pub project_type: project::ProjectType,
    /// The header under which the category should go
    pub header: String,
}

/// Deserializes an empty string as None
fn deserialize_empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
}

/// A loader that can load projects of `project_type`
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Loader {
    /// An SVG icon for the loader
    pub icon: String,
    pub name: LoaderType,
    /// The project types that this loader can load
    pub supported_project_types: Vec<project::ProjectType>,
}

#[derive(
    serde_enum_str::Deserialize_enum_str,
    serde_enum_str::Serialize_enum_str,
    Debug,
    PartialEq,
    Eq,
    Clone,
    strum_macros::EnumIter,
)]
#[serde(rename_all = "lowercase")]
pub enum LoaderType {
    Bukkit,
    Bungeecord,
    Canvas,
    Datapack,
    Fabric,
    Folia,
    Forge,
    Neoforge,
    Iris,
    Liteloader,
    Minecraft,
    Modloader,
    Optifine,
    Paper,
    Purpur,
    Quilt,
    Rift,
    Spigot,
    Sponge,
    Vanilla,
    Velocity,
    Waterfall,
    Babric,
    #[serde(rename = "bta-babric")]
    BtaBabric,
    Geyser,
    #[serde(rename = "java-agent")]
    JavaAgent,
    #[serde(rename = "legacy-fabric")]
    LegacyFabric,
    Nilloader,
    Ornithe,
    #[serde(other)]
    Other(String),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GameVersion {
    pub version: String,
    /// The type of the game version
    pub version_type: GameVersionType,
    /// When the game version released
    pub date: UtcDateTime,
    /// Whether this game version was considered a major version
    ///
    /// This is set to true if this version introduced many breaking changes to internal APIs
    /// that causes most mods made for previous versions of the game to break on this version.
    pub major: bool,
}

/// The licenses that projects can be searched with
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct License {
    /// The SPDX license ID of a project
    pub short: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DonationPlatform {
    /// A short identifier for the donation platform
    pub short: String,
    pub name: String,
}

/// The type of a game version
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GameVersionType {
    Snapshot,
    Release,
    Beta,
    Alpha,
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserializes_new_minecraft_java_server_category() {
        // Fixture like the live API: icon may be an empty string
        let raw = r#"{"icon":"","name":"adventure-mode","project_type":"minecraft_java_server","header":"minecraft_server_meta"}"#;
        let cat: Category = serde_json::from_str(raw).unwrap();
        assert_eq!(cat.project_type, project::ProjectType::MinecraftJavaServer);
        assert_eq!(cat.header, "minecraft_server_meta");
        // Serialization round-trip: exactly "minecraft_java_server"
        assert_eq!(
            serde_json::to_string(&cat.project_type).unwrap(),
            r#""minecraft_java_server""#
        );
    }

    #[test]
    fn unknown_project_type_falls_back_to_unknown() {
        // The crash-guard: future Modrinth types must never crash
        let raw = r#"{"icon":"","name":"whatever","project_type":"some_future_type","header":"x"}"#;
        let cat: Category = serde_json::from_str(raw).unwrap();
        assert_eq!(cat.project_type, project::ProjectType::Unknown);
    }

    #[test]
    fn deserializes_new_modrinth_loaders() {
        let raw = r#"{"icon":"","name":"geyser","supported_project_types":["mod","project","minecraft_java_server"]}"#;
        let loader: Loader = serde_json::from_str(raw).unwrap();
        assert_eq!(loader.name, LoaderType::Geyser);
        assert!(
            loader
                .supported_project_types
                .contains(&project::ProjectType::MinecraftJavaServer)
        );
        assert!(
            loader
                .supported_project_types
                .contains(&project::ProjectType::Unknown)
        ); // "project" -> Unknown
    }
}
