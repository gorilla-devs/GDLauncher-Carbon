//
// Modloader Prep

use std::sync::Arc;

use daedalus::minecraft::VersionInfo;

use crate::{
    domain::instance::info::{ModLoader, ModLoaderType, StandardVersion},
    managers::AppInner,
};

#[async_trait::async_trait]
pub trait PrepareModLoader {
    async fn prepare_modloader(
        &self,
        app: Arc<AppInner>,
        mc_version: &StandardVersion,
        version_info: VersionInfo,
    ) -> anyhow::Result<VersionInfo>;
}

pub struct ForgeModLoader {
    pub version: String,
}

#[async_trait::async_trait]
impl PrepareModLoader for ForgeModLoader {
    async fn prepare_modloader(
        &self,
        app: Arc<AppInner>,
        mc_version: &StandardVersion,
        version_info: VersionInfo,
    ) -> anyhow::Result<VersionInfo> {
        let forge_manifest = app.minecraft_manager().get_forge_manifest().await?;

        let forge_version = match self
            .version
            .strip_prefix(&format!("{}-", mc_version.release))
        {
            None => self.version.as_str(),
            Some(sub) => sub,
        };

        let forge_manifest_version = forge_manifest
            .game_versions
            .into_iter()
            .find(|v| v.id == mc_version.release)
            .ok_or_else(|| {
                anyhow::anyhow!("Could not find forge versions for {}", mc_version.release)
            })?
            .loaders
            .into_iter()
            .find(|v| v.id == format!("{}-{}", mc_version.release, forge_version))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not find forge version {}-{} for minecraft version {}",
                    mc_version.release,
                    forge_version,
                    mc_version.release,
                )
            })?;

        let forge_version = crate::managers::minecraft::forge::get_version(
            &app.reqwest_client,
            forge_manifest_version,
        )
        .await?;

        Ok(daedalus::modded::merge_partial_version(
            forge_version,
            version_info,
        ))
    }
}

pub struct FabricModLoader {
    pub version: String,
}

#[async_trait::async_trait]
impl PrepareModLoader for FabricModLoader {
    async fn prepare_modloader(
        &self,
        app: Arc<AppInner>,
        mc_version: &StandardVersion,
        version_info: VersionInfo,
    ) -> anyhow::Result<VersionInfo> {
        let fabric_manifest = app.minecraft_manager().get_fabric_manifest().await?;

        let fabric_version = match self
            .version
            .strip_prefix(&format!("{}-", mc_version.release))
        {
            None => self.version.as_str(),
            Some(sub) => sub,
        };

        let dummy_string = daedalus::BRANDING
            .get_or_init(daedalus::Branding::default)
            .dummy_replace_string
            .clone();

        let supported = fabric_manifest
            .game_versions
            .iter()
            .any(|v| v.id == mc_version.release);

        if !supported {
            return Err(anyhow::anyhow!(
                "Fabric does not support version {}",
                mc_version.release
            ));
        }

        let fabric_manifest_version = fabric_manifest
            .game_versions
            .into_iter()
            .find(|v| v.id == dummy_string)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not find fabric metadata template using {}",
                    dummy_string
                )
            })?
            .loaders
            .into_iter()
            .find(|v| v.id == fabric_version)
            .ok_or_else(|| anyhow::anyhow!("Could not find fabric version {}", fabric_version))?;

        let fabric_version = crate::managers::minecraft::fabric::replace_template(
            &crate::managers::minecraft::fabric::get_version(
                &app.reqwest_client,
                fabric_manifest_version,
            )
            .await?,
            &mc_version.release,
            &dummy_string,
        );

        Ok(daedalus::modded::merge_partial_version(
            fabric_version,
            version_info,
        ))
    }
}

pub struct QuiltModLoader {
    version: String,
}

#[async_trait::async_trait]
impl PrepareModLoader for QuiltModLoader {
    async fn prepare_modloader(
        &self,
        app: Arc<AppInner>,
        mc_version: &StandardVersion,
        version_info: VersionInfo,
    ) -> anyhow::Result<VersionInfo> {
        let quilt_manifest = app.minecraft_manager().get_quilt_manifest().await?;

        let quilt_version = match self
            .version
            .strip_prefix(&format!("{}-", mc_version.release))
        {
            None => self.version.as_str(),
            Some(sub) => sub,
        };

        let dummy_string = daedalus::BRANDING
            .get_or_init(daedalus::Branding::default)
            .dummy_replace_string
            .clone();

        let supported = quilt_manifest
            .game_versions
            .iter()
            .any(|v| v.id == mc_version.release);

        if !supported {
            return Err(anyhow::anyhow!(
                "Quilt does not support version {}",
                mc_version.release
            ));
        }

        let quilt_manifest_version = quilt_manifest
            .game_versions
            .into_iter()
            .find(|v| v.id == dummy_string)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not find quilt metadata template using {}",
                    dummy_string
                )
            })?
            .loaders
            .into_iter()
            .find(|v| v.id == quilt_version)
            .ok_or_else(|| anyhow::anyhow!("Could not find quilt version {}", quilt_version))?;

        let quilt_version = crate::managers::minecraft::quilt::replace_template(
            &crate::managers::minecraft::quilt::get_version(
                &app.reqwest_client,
                quilt_manifest_version,
            )
            .await?,
            &mc_version.release,
            &dummy_string,
        );

        Ok(daedalus::modded::merge_partial_version(
            quilt_version,
            version_info,
        ))
    }
}

#[async_trait::async_trait]
impl PrepareModLoader for ModLoader {
    async fn prepare_modloader(
        &self,
        app: Arc<AppInner>,
        mc_version: &StandardVersion,
        version_info: VersionInfo,
    ) -> anyhow::Result<VersionInfo> {
        match self.type_ {
            ModLoaderType::Forge => {
                ForgeModLoader {
                    version: self.version.clone(),
                }
                .prepare_modloader(app, mc_version, version_info)
                .await
            }
            ModLoaderType::Fabric => {
                FabricModLoader {
                    version: self.version.clone(),
                }
                .prepare_modloader(app, mc_version, version_info)
                .await
            }
            ModLoaderType::Quilt => {
                QuiltModLoader {
                    version: self.version.clone(),
                }
                .prepare_modloader(app, mc_version, version_info)
                .await
            }
        }
    }
}
