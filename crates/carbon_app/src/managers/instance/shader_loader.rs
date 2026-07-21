use super::{InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};
use crate::domain::instance::InstanceSettingsUpdate;
use crate::domain::instance::info::{GameVersion, ModLoaderType};
use crate::domain::vtask::VisualTaskId;
use crate::managers::ManagerRef;
use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use specta::Type;

pub const IRIS_MODRINTH_ID: &str = "YL57xq9U";
pub const SODIUM_MODRINTH_ID: &str = "AANobbMI";
pub const OCULUS_MODRINTH_ID: &str = "GchcoXML";
pub const EMBEDDIUM_MODRINTH_ID: &str = "sk9rgfiA";
pub const LITHIUM_MODRINTH_ID: &str = "gvQqBUqZ";
pub const PHOSPHOR_MODRINTH_ID: &str = "hEOCdOgW";

pub const IRIS_CF_ID: u32 = 455508;
pub const OCULUS_CF_ID: u32 = 581495;
pub const EMBEDDIUM_CF_ID: u32 = 908741;

#[derive(Type, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShaderLoaderKind {
    Iris,
    Oculus,
    OptiFine,
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ShaderRecommendation {
    LoaderPresent {
        loader: ShaderLoaderKind,
    },
    RecommendLoader {
        recommended: ShaderLoaderKind,
        modloader_type: ApiModLoaderType,
        mc_version: String,
        /// Modrinth project id of the loader the wizard will install. Carried
        /// in the recommendation so the frontend doesn't need a parallel
        /// copy of these constants.
        loader_modrinth_id: String,
    },
    RequiresModloader {
        mc_version: String,
        /// Modrinth project id of the shader loader to install once the
        /// modloader is in place. Always Iris for this branch (we install
        /// Fabric, and Iris is the Fabric-native option).
        loader_modrinth_id: String,
    },
}

/// Modrinth project id for the shader loader, when one exists. OptiFine
/// returns `None` because it ships outside Modrinth and the wizard never
/// installs it automatically.
fn shader_loader_modrinth_id(kind: ShaderLoaderKind) -> Option<&'static str> {
    match kind {
        ShaderLoaderKind::Iris => Some(IRIS_MODRINTH_ID),
        ShaderLoaderKind::Oculus => Some(OCULUS_MODRINTH_ID),
        ShaderLoaderKind::OptiFine => None,
    }
}

#[derive(Type, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ApiModLoaderType {
    Neoforge,
    Forge,
    Fabric,
    Quilt,
}

impl From<ModLoaderType> for ApiModLoaderType {
    fn from(value: ModLoaderType) -> Self {
        match value {
            ModLoaderType::Neoforge => Self::Neoforge,
            ModLoaderType::Forge => Self::Forge,
            ModLoaderType::Fabric => Self::Fabric,
            ModLoaderType::Quilt => Self::Quilt,
        }
    }
}

impl ManagerRef<'_, InstanceManager> {
    pub async fn install_fabric_loader_default(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<VisualTaskId> {
        let mc_version = {
            let instances = self.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;
            let InstanceType::Valid(data) = &instance.type_ else {
                bail!("instance {} is not valid", *instance_id);
            };
            match &data.config.game_configuration.version {
                Some(GameVersion::Standard(v)) => v.release.clone(),
                _ => bail!("custom versions are not supported"),
            }
        };

        let manifest = self.app.minecraft_manager().get_fabric_manifest().await?;

        // Daedalus convention: each MC version entry just signals support
        // (loaders are empty); the actual loader pool lives under a single
        // dummy entry id="${gdlauncher.gameVersion}".
        const DUMMY_META_VERSION: &str = "${gdlauncher.gameVersion}";

        let supported = manifest.game_versions.iter().any(|v| v.id == mc_version);
        if !supported {
            bail!("Fabric does not support Minecraft {}", mc_version);
        }

        let loader_version = manifest
            .game_versions
            .iter()
            .find(|v| v.id == DUMMY_META_VERSION)
            .and_then(|v| {
                v.loaders
                    .iter()
                    .find(|l| l.stable)
                    .or_else(|| v.loaders.first())
            })
            .map(|l| l.id.clone())
            .ok_or_else(|| {
                anyhow!("Fabric loader manifest is empty (no loaders for any version)")
            })?;

        let modloader = crate::domain::instance::info::ModLoader {
            type_: ModLoaderType::Fabric,
            version: loader_version,
        };

        let update = InstanceSettingsUpdate {
            instance_id,
            name: None,
            use_loaded_icon: None,
            notes: None,
            version: None,
            modloader: Some(Some(modloader)),
            java_override: None,
            global_java_args: None,
            extra_java_args: None,
            memory: None,
            pre_launch_hook: None,
            post_exit_hook: None,
            wrapper_command: None,
            game_resolution: None,
            mod_sources: None,
            modpack_locked: None,
        };

        self.update_instance(update)
            .await?
            .ok_or_else(|| anyhow!("update_instance did not return a setup task"))
    }

    pub async fn check_shader_requirements(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<ShaderRecommendation> {
        let (mc_version, modloader_type) = {
            let instances = self.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;

            let InstanceType::Valid(data) = &instance.type_ else {
                bail!("instance {} is not valid", *instance_id);
            };

            match &data.config.game_configuration.version {
                Some(GameVersion::Standard(version)) => {
                    let modloader = version.modloaders.iter().next().map(|m| m.type_);
                    (version.release.clone(), modloader)
                }
                _ => bail!("custom versions are not supported for shader detection"),
            }
        };

        if let Some(loader) = self.detect_shader_loader(instance_id).await? {
            return Ok(ShaderRecommendation::LoaderPresent { loader });
        }

        let Some(modloader_type) = modloader_type else {
            return Ok(ShaderRecommendation::RequiresModloader {
                mc_version,
                loader_modrinth_id: IRIS_MODRINTH_ID.to_string(),
            });
        };

        let recommended = recommend_loader(modloader_type);
        // `recommend_loader` only ever returns Iris or Oculus, both of
        // which have Modrinth IDs — the OptiFine branch of
        // `shader_loader_modrinth_id` is unreachable here.
        let loader_modrinth_id = shader_loader_modrinth_id(recommended)
            .expect("recommend_loader returned a non-Modrinth loader")
            .to_string();

        Ok(ShaderRecommendation::RecommendLoader {
            recommended,
            modloader_type: modloader_type.into(),
            mc_version,
            loader_modrinth_id,
        })
    }

    async fn detect_shader_loader(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Option<ShaderLoaderKind>> {
        let instance_id_val = *instance_id;
        let mods = carbon_repos::repos::mod_file_cache::get_enabled_instance_mod_modids(
            &self.app.db,
            instance_id_val,
        )
        .await?;

        for entry in mods {
            let Some(modid) = entry.modid else {
                continue;
            };

            if let Some(loader) = match_shader_loader_modid(&modid) {
                return Ok(Some(loader));
            }
        }

        Ok(None)
    }
}

fn match_shader_loader_modid(modid: &str) -> Option<ShaderLoaderKind> {
    match modid.to_ascii_lowercase().as_str() {
        "iris" => Some(ShaderLoaderKind::Iris),
        "oculus" => Some(ShaderLoaderKind::Oculus),
        "optifine" | "optifabric" => Some(ShaderLoaderKind::OptiFine),
        _ => None,
    }
}

fn recommend_loader(modloader: ModLoaderType) -> ShaderLoaderKind {
    match modloader {
        ModLoaderType::Fabric | ModLoaderType::Quilt | ModLoaderType::Neoforge => {
            ShaderLoaderKind::Iris
        }
        ModLoaderType::Forge => ShaderLoaderKind::Oculus,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn recommendation_matrix() {
        assert_eq!(
            recommend_loader(ModLoaderType::Fabric),
            ShaderLoaderKind::Iris
        );
        assert_eq!(
            recommend_loader(ModLoaderType::Quilt),
            ShaderLoaderKind::Iris
        );
        assert_eq!(
            recommend_loader(ModLoaderType::Neoforge),
            ShaderLoaderKind::Iris
        );
        assert_eq!(
            recommend_loader(ModLoaderType::Forge),
            ShaderLoaderKind::Oculus
        );
    }

    #[test]
    fn modid_detection() {
        assert_eq!(
            match_shader_loader_modid("iris"),
            Some(ShaderLoaderKind::Iris)
        );
        assert_eq!(
            match_shader_loader_modid("Iris"),
            Some(ShaderLoaderKind::Iris)
        );
        assert_eq!(
            match_shader_loader_modid("oculus"),
            Some(ShaderLoaderKind::Oculus)
        );
        assert_eq!(
            match_shader_loader_modid("optifine"),
            Some(ShaderLoaderKind::OptiFine)
        );
        assert_eq!(
            match_shader_loader_modid("optifabric"),
            Some(ShaderLoaderKind::OptiFine)
        );
        assert_eq!(match_shader_loader_modid("sodium"), None);
        assert_eq!(match_shader_loader_modid("embeddium"), None);
    }
}
