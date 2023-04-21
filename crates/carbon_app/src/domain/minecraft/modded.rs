use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::minecraft::{Libraries, Library, VersionArguments, VersionInfo, VersionType};

pub const DUMMY_REPLACE_STRING: &str = "${gdlauncher.gameVersion}";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModdedManifest {
    pub game_versions: Vec<ModdedManifestVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModdedManifestVersion {
    pub id: String,
    pub stable: bool,
    pub loaders: Vec<ModdedManifestLoaderVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModdedManifestLoaderVersion {
    pub id: String,
    pub url: String,
    pub stable: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// A partial version returned by fabric meta
pub struct PartialVersionInfo {
    pub id: String,
    pub inherits_from: String,
    pub release_time: DateTime<Utc>,
    pub time: DateTime<Utc>,
    pub main_class: Option<String>,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<VersionArguments>,
    pub libraries: Vec<Library>,
    #[serde(rename = "type")]
    pub type_: VersionType,
    /// (Forge-only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<HashMap<String, SidedDataEntry>>,
    /// (Forge-only) The list of processors to run after downloading the files
    pub processors: Option<Vec<Processor>>,
}

pub fn merge_partial_version(partial: PartialVersionInfo, merge: VersionInfo) -> VersionInfo {
    let merge_id = merge.id.clone();

    VersionInfo {
        arguments: if let Some(partial_args) = partial.arguments {
            if let Some(merge_args) = merge.arguments {
                let mut new_map = VersionArguments::new();

                fn add_keys(new_map: &mut VersionArguments, args: VersionArguments) {
                    for game in args.game {
                        if !new_map.game.contains(&game) {
                            new_map.game.push(game);
                        }
                    }

                    for jvm in args.jvm {
                        if !new_map.jvm.contains(&jvm) {
                            new_map.jvm.push(jvm);
                        }
                    }
                }

                add_keys(&mut new_map, merge_args);
                add_keys(&mut new_map, partial_args);

                Some(new_map)
            } else {
                Some(partial_args)
            }
        } else {
            merge.arguments
        },
        asset_index: merge.asset_index,
        inherits_from: merge.inherits_from,
        assets: merge.assets,
        downloads: merge.downloads,
        id: partial.id.replace(DUMMY_REPLACE_STRING, &merge_id),
        java_version: merge.java_version,
        libraries: Libraries {
            libraries: partial
                .libraries
                .into_iter()
                .chain(merge.libraries.libraries)
                .map(|x| Library {
                    downloads: x.downloads,
                    extract: x.extract,
                    name: x.name.replace(DUMMY_REPLACE_STRING, &merge_id),
                    url: x.url,
                    natives: x.natives,
                    rules: x.rules,
                })
                .collect::<Vec<_>>(),
        },
        main_class: if let Some(main_class) = partial.main_class {
            main_class
        } else {
            merge.main_class
        },
        minecraft_arguments: partial.minecraft_arguments,
        minimum_launcher_version: merge.minimum_launcher_version,
        release_time: partial.release_time,
        time: partial.time,
        type_: partial.type_,
        data: partial.data,
        processors: partial.processors,
        logging: merge.logging,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Processor {
    /// Maven coordinates for the JAR library of this processor.
    pub jar: String,
    /// Maven coordinates for all the libraries that must be included in classpath when running this processor.
    pub classpath: Vec<String>,
    /// Arguments for this processor.
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Represents a map of outputs. Keys and values can be data values
    pub outputs: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Which sides this processor shall be ran on.
    /// Valid values: client, server, extract
    pub sides: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SidedDataEntry {
    /// The value on the client
    pub client: String,
    /// The value on the server
    pub server: String,
}
