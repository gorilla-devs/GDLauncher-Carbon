use super::{InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};
use crate::domain::instance::InstanceSettingsUpdate;
use crate::domain::instance::info::{GameVersion, ModLoaderType};
use crate::domain::vtask::VisualTaskId;
use crate::managers::ManagerRef;
use anyhow::{Context, anyhow, bail};
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

/// Exact provider artifact and instance target selected by the Auto action.
/// Verification is read-only; possession of this value never authorizes enabling.
#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct ShaderLoaderVerification {
    pub instance_id: i32,
    pub mc_version: String,
    pub modloaders: Vec<String>,
    pub filename: String,
    pub sha1: String,
    /// rspc refuses to export `u64`; exported as `number` like the other
    /// file sizes (see `FEUntrackedFile::size`). Loader jars stay far below
    /// 2^53 bytes, so the JSON number round-trips exactly.
    #[specta(type = f64)]
    pub size: u64,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct ShaderLoaderInstall {
    pub task_id: i32,
    pub verification: ShaderLoaderVerification,
}

impl ShaderLoaderVerification {
    pub(super) async fn validate_target(
        &self,
        app: &std::sync::Arc<crate::managers::AppInner>,
        id: InstanceId,
    ) -> anyhow::Result<()> {
        if self.instance_id != *id {
            bail!("shader loader target instance changed");
        }
        let manager = app.instance_manager();
        let instances = manager.instances.read().await;
        let config = &instances
            .get(&id)
            .ok_or(InvalidInstanceIdError(id))?
            .data()?
            .config;
        let Some(GameVersion::Standard(version)) = &config.game_configuration.version else {
            bail!("unsupported shader target");
        };
        if version.release != self.mc_version || target_modloaders(version) != self.modloaders {
            bail!("shader loader target changed during installation");
        }
        Ok(())
    }
}

fn target_modloaders(version: &crate::domain::instance::info::StandardVersion) -> Vec<String> {
    let mut loaders = version
        .modloaders
        .iter()
        .map(|l| format!("{}:{}", l.type_.to_string(), l.version))
        .collect::<Vec<_>>();
    loaders.sort();
    loaders
}

impl ManagerRef<'_, InstanceManager> {
    /// Auto is the only entry point that grants enabling authority. Resolve an
    /// exact compatible provider version; ordinary generic installs keep disabled intent.
    pub async fn ensure_shader_loader(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<ShaderLoaderInstall> {
        use super::installer::{IntoInstaller, ModrinthModInstaller};
        use carbon_platforms::modrinth::{project::ProjectVersionsFilters, search::ProjectID};
        self.ensure_modpack_not_locked(instance_id).await?;
        let target = {
            let instances = self.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;
            let Some(GameVersion::Standard(version)) =
                &instance.data()?.config.game_configuration.version
            else {
                bail!("unsupported shader target");
            };
            version.clone()
        };
        let loader = target
            .modloaders
            .iter()
            .next()
            .ok_or_else(|| anyhow!("install a supported modloader before shader Auto"))?
            .type_;
        let project_id = shader_loader_modrinth_id(recommend_loader(loader))
            .unwrap()
            .to_owned();
        let platform = &self.app.modplatforms_manager().modrinth;
        let project = platform.get_project(ProjectID(project_id.clone())).await?;
        if project.id != project_id
            || project.project_type != carbon_platforms::modrinth::project::ProjectType::Mod
        {
            bail!("shader provider returned an unexpected project or addon kind");
        }
        let versions = platform
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(project_id.clone()),
                game_versions: Some(vec![target.release.clone()]),
                loaders: Some(vec![loader.to_string()]),
                limit: None,
                offset: None,
            })
            .await?;
        let version = versions
            .iter()
            .find(|v| {
                v.project_id == project_id
                    && v.game_versions.contains(&target.release)
                    && v.loaders
                        .iter()
                        .any(|l| l.eq_ignore_ascii_case(&loader.to_string()))
            })
            .ok_or_else(|| anyhow!("no compatible shader loader version"))?
            .clone();
        let resource = ModrinthModInstaller::from_version(version, project)?;
        let path = super::installer::publication::current_path(self.app, instance_id).await?;
        use super::installer::ResourceInstaller;
        let download = resource
            .downloadable(&path)
            .await
            .context("shader loader has no artifact")?;
        let Some(carbon_net::Checksum::Sha1(sha1)) = download.checksum else {
            bail!("shader loader has no supported checksum");
        };
        let verification = ShaderLoaderVerification {
            instance_id: *instance_id,
            mc_version: target.release.clone(),
            modloaders: target_modloaders(&target),
            filename: super::installer::publication::filename(&resource, &path)?,
            sha1,
            size: download.size.context("shader loader size missing")?,
        };
        let mut installer = resource.into_installer();
        installer.required_enabled = Some(verification.clone());
        let task = installer.install(self.app, instance_id, true, None).await?;
        Ok(ShaderLoaderInstall {
            task_id: task.0,
            verification,
        })
    }

    pub async fn verify_shader_loader(
        self,
        verification: ShaderLoaderVerification,
    ) -> anyhow::Result<bool> {
        use super::installer::publication;
        let id = InstanceId(verification.instance_id);
        let _operation = self.lock_instance(id).await;
        verification.validate_target(self.app, id).await?;
        let path = publication::current_path(self.app, id).await?;
        let folder =
            publication::checked_folder(&path, crate::domain::instance::AddonType::Mods).await?;
        let Some((physical, enabled)) =
            publication::selected_file(&folder, &verification.filename).await?
        else {
            return Ok(false);
        };
        if !enabled {
            return Ok(false);
        }
        let rows =
            carbon_repos::repos::mod_file_cache::get_instance_mods_full(&self.app.db, *id).await?;
        if !rows.iter().any(|row| {
            row.enabled && row.filename == verification.filename && row.addon_type == "mods"
        }) {
            return Ok(false);
        }
        let downloadable = carbon_net::Downloadable::new("", physical.clone())
            .with_size(verification.size)
            .with_checksum(Some(carbon_net::Checksum::Sha1(verification.sha1)));
        publication::matches_download(&physical, &downloadable).await
    }

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
        use super::installer::publication;
        let (path, modloader) = {
            let instances = self.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;
            let modloader = match &instance.data()?.config.game_configuration.version {
                Some(GameVersion::Standard(v)) => v.modloaders.iter().next().map(|l| l.type_),
                _ => None,
            };
            (
                self.app
                    .settings_manager()
                    .runtime_path
                    .get_instances()
                    .get_instance_path(&instance.shortpath),
                modloader,
            )
        };
        let folder =
            publication::checked_folder(&path, crate::domain::instance::AddonType::Mods).await?;
        let mods =
            carbon_repos::repos::mod_file_cache::get_instance_mods_full(&self.app.db, *instance_id)
                .await?;
        for entry in mods {
            if !entry.enabled || entry.addon_type != "mods" {
                continue;
            }
            let Some(loader) = entry.modid.as_deref().and_then(match_shader_loader_modid) else {
                continue;
            };
            if loader != ShaderLoaderKind::OptiFine
                && modloader.map(recommend_loader) != Some(loader)
            {
                continue;
            }
            if let Some(target) = modloader {
                let compatible = entry.modloaders.split(',').any(|value| {
                    ModLoaderType::try_from(value).ok().is_some_and(|actual| {
                        actual == target
                            || (target == ModLoaderType::Quilt && actual == ModLoaderType::Fabric)
                    })
                });
                if loader != ShaderLoaderKind::OptiFine && !compatible {
                    continue;
                }
            }
            let found = match publication::selected_file(&folder, &entry.filename).await {
                Ok(found) => found,
                Err(error) => {
                    // A linked jar, or an enabled and a disabled copy of one
                    // name: this only reads, and the scanner read the copy,
                    // so trust its row, as 2.0.40 did and dependency
                    // planning does.
                    tracing::debug!(
                        filename = %entry.filename,
                        "Shader loader copy cannot be inspected, using its recorded state: {error:#}"
                    );
                    return Ok(Some(loader));
                }
            };
            let Some((physical, enabled)) = found else {
                continue;
            };
            if !enabled {
                continue;
            }
            let download = carbon_net::Downloadable::new("", physical.clone())
                .with_size(entry.filesize as u64)
                .with_checksum(Some(carbon_net::Checksum::Sha512(hex::encode(
                    &entry.sha512,
                ))));
            if publication::matches_download(&physical, &download).await? {
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

    #[tokio::test]
    async fn task05_shader_preflight_requires_real_enabled_loader_of_current_kind() {
        use crate::domain::instance::{AddonType, info};
        use carbon_repos::{
            dbtypes::DbDateTime,
            repos::{mod_file_cache, mod_metadata},
        };
        use sha1::Digest;
        let app = crate::setup_managers_for_test().await;
        let (id, _, path) = super::super::mods::tests::addon_fixture(
            &app,
            "iris-preflight",
            AddonType::Mods,
            "iris.jar",
            b"iris",
        )
        .await;
        let now = DbDateTime(chrono::Utc::now().fixed_offset());
        mod_metadata::insert_metadata(
            &app.db,
            "iris-actual",
            0,
            &sha2::Sha512::digest(b"iris"),
            &sha1::Sha1::digest(b"iris"),
            "fabric",
            Some("Iris"),
            Some("iris"),
            None,
            None,
            None,
            now,
        )
        .await
        .unwrap();
        mod_file_cache::upsert_mod_file_cache(
            &app.db,
            *id,
            "iris.jar".into(),
            4,
            true,
            "mods".into(),
            "iris-actual".into(),
            now,
        )
        .await
        .unwrap();
        {
            let manager = app.instance_manager();
            let mut instances = manager.instances.write().await;
            instances
                .get_mut(&id)
                .unwrap()
                .data_mut()
                .unwrap()
                .config
                .game_configuration
                .version = Some(GameVersion::Standard(info::StandardVersion {
                release: "1.20.1".into(),
                modloaders: [info::ModLoader {
                    type_: ModLoaderType::Fabric,
                    version: "fixture".into(),
                }]
                .into(),
            }));
        }
        assert!(matches!(
            app.instance_manager()
                .check_shader_requirements(id)
                .await
                .unwrap(),
            ShaderRecommendation::LoaderPresent {
                loader: ShaderLoaderKind::Iris
            }
        ));
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(
            app.instance_manager()
                .check_shader_requirements(id)
                .await
                .unwrap(),
            ShaderRecommendation::RecommendLoader { .. }
        ));
        std::fs::write(&path, b"changed").unwrap();
        assert!(matches!(
            app.instance_manager()
                .check_shader_requirements(id)
                .await
                .unwrap(),
            ShaderRecommendation::RecommendLoader { .. }
        ));
        // An enabled and a disabled copy of one name, or a jar linked from
        // elsewhere, cannot be inspected as one file. This only reads, so the
        // row is trusted, as 2.0.40 did, rather than failing the preflight.
        let present = || async {
            matches!(
                app.instance_manager()
                    .check_shader_requirements(id)
                    .await
                    .unwrap(),
                ShaderRecommendation::LoaderPresent {
                    loader: ShaderLoaderKind::Iris
                }
            )
        };
        std::fs::write(&path, b"iris").unwrap();
        std::fs::write(path.with_file_name("iris.jar.disabled"), b"iris").unwrap();
        assert!(present().await);
        #[cfg(unix)]
        {
            std::fs::remove_file(path.with_file_name("iris.jar.disabled")).unwrap();
            let shared = tempfile::tempdir().unwrap();
            let target = shared.path().join("iris.jar");
            std::fs::rename(&path, &target).unwrap();
            std::os::unix::fs::symlink(&target, &path).unwrap();
            assert!(present().await);
        }
    }

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
