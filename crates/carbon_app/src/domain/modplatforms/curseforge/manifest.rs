use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::domain::instance::info::{ModLoader, ModLoaderType, StandardVersion};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub minecraft: Minecraft,
    pub manifest_type: String,
    pub name: String,
    pub version: Option<String>,
    pub author: String,
    pub overrides: String,
    pub files: Vec<ManifestFileReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Minecraft {
    pub version: String,
    pub mod_loaders: Vec<ModLoaders>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModLoaders {
    pub id: String,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFileReference {
    #[serde(rename = "projectID")]
    pub project_id: u32,
    #[serde(rename = "fileID")]
    pub file_id: u32,
    pub required: bool,
}
impl TryFrom<Minecraft> for StandardVersion {
    type Error = anyhow::Error;

    fn try_from(value: Minecraft) -> Result<Self, Self::Error> {
        Ok(StandardVersion {
            release: value.version.clone(),
            modloaders: value
                .mod_loaders
                .into_iter()
                .map(|mod_loader| {
                    let (loader, version) = mod_loader.id.split_once('-').ok_or_else(|| {
                        anyhow::anyhow!(
                            "modloader id '{}' could not be split into a name-version pair",
                            mod_loader.id
                        )
                    })?;

                    Ok(ModLoader {
                        type_: match loader {
                            "forge" => ModLoaderType::Forge,
                            "fabric" => ModLoaderType::Fabric,
                            "quilt" => ModLoaderType::Quilt,
                            _ => bail!("unsupported modloader '{loader}'"),
                        },
                        version: format!("{}-{}", value.version, version),
                    })
                })
                .collect::<Result<_, _>>()?,
        })
    }
}
