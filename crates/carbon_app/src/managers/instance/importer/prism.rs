use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};

use anyhow::{anyhow, Context};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::{api::translation::Translation, managers::{instance::installer, AppInner}};
use crate::domain::vtask::VisualTaskId;

use super::{ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter, InternalImportEntry, InvalidImportEntry, GET_IMPORT_SCAN_STATUS};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    instance_cfg: InstanceCfg,
    mmc_pack: MmcPack,
}

#[derive(Debug, Clone)]
struct InstanceCfg {
    name: String,
    icon_key: String,
    pack: Option<ManagedPack>,
}

#[derive(Debug, Clone)]
struct ManagedPack {
    id: String,
    version: String,
    type_: String,
}

fn parse_instance_conf(text: &str) -> anyhow::Result<InstanceCfg> {
    // appears to be an INI that only has one category
    let values = text.lines().filter_map(|line| line.split_once('=')).collect::<HashMap<&str, &str>>();

    let get = |name| values.get("name").with_context(|| format!("missing key '{name}'")).map(|v| v.to_string());

    Ok(InstanceCfg {
        name: get("name")?,
        icon_key: get("iconKey")?,
        pack: values.get("ManagedPack")
            .map(|mp| bool::from_str(mp))
            .unwrap_or(Ok(false))
            .context("ManagedPack is not a boolean")?
            .then(|| Ok::<_, anyhow::Error>(ManagedPack {
                id: get("ManagedPackID")?,
                version: get("ManagedPackVersionID")?,
                type_: get("ManagedPackType")?,
            })).transpose()?,
    })
}

#[derive(Debug, Clone, Deserialize)]
struct MmcPack {
    components: Vec<MmcPackItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct MmcPackItem {
    uid: String,
    version: String,
}

#[derive(Debug, Clone, Deserialize)]
enum MmcPackUid {
    #[serde(rename = "net.minecraft")]
    Minecraft,
    #[serde(rename = "net.minecraftforge")]
    Forge,
    #[serde(rename = "net.fabricmc.fabric-loader")]
    Fabric,
    #[serde(rename = "org.quiltmc.quilt-loader")]
    Quilt,
    #[serde(rename = "net.neoforged")]
    Neoforge,
    Unknown(String),
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.instance_cfg.name,
        }
    }
}

#[derive(Debug)]
pub struct PrismImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl PrismImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path() -> anyhow::Result<PathBuf> {
        let basedirs = directories::BaseDirs::new().ok_or_else(|| anyhow!("Cannot build basedirs"))?;
        let p = basedirs.data_dir().join("PrismLauncher");
        Ok(p)
    }

    async fn scan_instance(
    &self,
    path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        let cfg_path = path.join("instance.cfg");
        let mmc_pack_path = path.join("mmc-pack.json");

        let cfg_text = match tokio::fs::read_to_string(&cfg_path).await {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        let mmc_pack_text = match tokio::fs::read_to_string(&mmc_pack_path).await {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        let cfg = parse_instance_conf(&cfg_text);
        let mmc_pack = serde_json::from_str::<MmcPack>(&mmc_pack_text);

        let filename = path.file_name().unwrap().to_string_lossy().to_string();

        match (cfg, mmc_pack) {
            (Ok(instance_cfg), Ok(mmc_pack)) => Ok(Some(InternalImportEntry::Valid(Importable {
                filename,
                path,
                instance_cfg,
                mmc_pack,
            }))),
            _ => Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportPrismMalformedManifest,
            }))),
        }
    }
}

#[async_trait::async_trait]
impl InstanceImporter for PrismImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        if scan_path.is_dir() {
            let Ok(mut dir) = tokio::fs::read_dir(&scan_path).await else {
                return Ok(());
            };

            while let Some(path) = dir.next_entry().await? {
                if path.metadata().await?.is_dir() {
                    if let Ok(Some(entry)) = self.scan_instance(path.path()).await {
                        self.state.write().await.push_multi(entry).await;
                        app.invalidate(GET_IMPORT_SCAN_STATUS, None);
                    }
                }
            }
        }

        Ok(())
    }

    async fn get_status(&self) -> ImportScanStatus {
        self.state.read().await.clone().into()
    }

    async fn begin_import(&self, app: &Arc<AppInner>, index: u32, name: Option<String>) -> anyhow::Result<VisualTaskId> {
        Err(anyhow!("fuck"))
    }
}
