use super::{
    InstanceId, InstanceManager, InvalidInstanceIdError,
    installer::{CurseforgeModInstaller, IntoInstaller, ModrinthModInstaller},
};
use crate::api::keys::instance::INSTANCE_MODS;
use crate::domain::instance::info::{GameVersion, ModLoaderType};
use crate::domain::instance::{self as domain, info};
use crate::managers::AppInner;
use crate::managers::instance::InstanceType;
use crate::managers::metadata::cache::eligibility::{
    Installed, InstalledId, UpdateContext, choose_update, installed_channel,
};
use crate::{domain::vtask::VisualTaskId, managers::ManagerRef};
use anyhow::Context;
use anyhow::{anyhow, bail};
use carbon_platforms::curseforge::FileReleaseType;
use carbon_platforms::curseforge::filters::{
    ModFileParameters, ModFilesParameters, ModFilesParametersQuery, ModParameters,
};
use carbon_platforms::modrinth::project::ProjectVersionsFilters;
use carbon_platforms::modrinth::search::ProjectID;
use carbon_platforms::modrinth::version::VersionType;
use carbon_platforms::{
    ModChannel, ModChannelWithUsage, ModPlatform, ModSources, RemoteVersion, curseforge, modrinth,
};
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::mod_file_cache as mfcdb;
use chrono::{DateTime, FixedOffset, Utc};
use futures::Future;
use std::borrow::Cow;
use std::str::FromStr;
use thiserror::Error;

/// A world has only its exact spelling; file addons have two mutually exclusive
/// spellings. Probe strictly before removal and never choose between two copies.
pub(super) async fn remove_addon_from_disk(
    enabled_path: &std::path::Path,
    disabled_path: &std::path::Path,
    is_directory: bool,
) -> anyhow::Result<bool> {
    let enabled = addon_metadata(enabled_path).await?;
    let disabled = if is_directory {
        None
    } else {
        addon_metadata(disabled_path).await?
    };
    if enabled.is_some() && disabled.is_some() {
        bail!("both enabled and disabled addon paths exist; resolve the conflict before deletion");
    }
    let selected = enabled
        .map(|m| (enabled_path, m))
        .or_else(|| disabled.map(|m| (disabled_path, m)));
    match selected {
        Some((path, meta)) => remove_addon_entry(path, &meta, is_directory).await,
        None => Ok(false),
    }
}

/// Removes one addon entry `meta` (lstat) describes, reporting whether it did.
/// A file addon may be a link to a jar or pack kept elsewhere: only the link
/// is removed, never what it points to, as 2.0.40 did. A save that is a link
/// is refused: the scanner never lists one, and the save it points to is not
/// this instance's to delete.
pub(super) async fn remove_addon_entry(
    path: &std::path::Path,
    meta: &std::fs::Metadata,
    is_directory: bool,
) -> anyhow::Result<bool> {
    if meta.file_type().is_symlink() {
        if is_directory {
            bail!(
                "{} is a link to a save stored elsewhere. Remove the link in your file \
                 manager; the save it points to is kept.",
                path.display()
            );
        }
        // Windows removes a link to a folder (or a junction) as a folder.
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            if meta.file_type().is_symlink_dir() {
                tokio::fs::remove_dir(path).await?;
                return Ok(true);
            }
        }
        tokio::fs::remove_file(path).await?;
        return Ok(true);
    }
    if is_directory && meta.is_dir() {
        tokio::fs::remove_dir_all(path).await?;
    } else if !is_directory && meta.is_file() {
        tokio::fs::remove_file(path).await?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

/// What a Delete left when the folder held an addon twice, as `name` and
/// `name.disabled` (one row): it removed the copy the row showed, and the row
/// now shows the other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAddonCopy {
    /// The file name of the copy removed.
    pub deleted: String,
    /// The file name of the copy still in the folder.
    pub kept: String,
}

/// Why an action cannot pick a copy: the scanner lists `name` and
/// `name.disabled` as one row, and Delete in the Addons tab removes one of
/// them at a time. `verb` is what the action would do to the copy
/// ("enable", "update", ...).
pub(super) fn twin_copies_refusal(
    folder: &std::path::Path,
    name: &str,
    verb: &str,
) -> anyhow::Error {
    anyhow!(
        "Both {name} and {name}.disabled are in {}, so GDLauncher cannot tell which copy to {verb}. \
         Delete one of them (Delete in the Addons tab removes one copy at a time), then try again.",
        folder.display(),
    )
}

pub(super) async fn addon_metadata(
    path: &std::path::Path,
) -> anyhow::Result<Option<std::fs::Metadata>> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(meta) => Ok(Some(meta)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// One name inside an addon folder: never empty, `.`, `..`, or a path. The
/// rest is Windows-only: there `\` separates folders, `:` names a stream and a
/// trailing `.` or space is dropped (so the name would alias another file).
/// Elsewhere they are ordinary characters (macOS stores a `/` typed in Finder
/// as `:`), and the scanner lists such files, so their actions must work.
pub(super) fn validate_addon_basename(name: &str) -> anyhow::Result<()> {
    let windows_only = cfg!(windows) && (name.contains(['\\', ':']) || name.ends_with(['.', ' ']));
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\0']) || windows_only
    {
        bail!("invalid addon filename");
    }
    Ok(())
}

/// Rejects an enable/disable toggle for an addon type that cannot express one.
///
/// Disabling appends `.disabled` to the name inside the type's folder, which is
/// coherent for a file and incoherent for a world: a save is a directory,
/// Minecraft still reads a renamed one, and our own scanner
/// (`managers/metadata/cache/mod.rs`) re-reports `<name>.disabled` as a *new,
/// enabled* world — leaving a duplicate row the UI has no control to undo.
/// The frontend already declines to offer the control
/// (`pages/Library/shared/addons/addonCapabilities.ts`'s `supportsEnableToggle`);
/// this is the same rule where it cannot be routed around.
pub(super) fn ensure_toggleable(addon_type: domain::AddonType) -> anyhow::Result<()> {
    if addon_type == domain::AddonType::Worlds {
        bail!(
            "worlds cannot be enabled or disabled: a save is a directory, and \
             renaming it to <name>.disabled neither hides it from Minecraft nor \
             from our own scanner, which re-reports it as a new, enabled world"
        );
    }

    Ok(())
}

impl ManagerRef<'_, InstanceManager> {
    pub(super) async fn get_owned_mod_file(
        &self,
        instance_id: InstanceId,
        id: &str,
    ) -> anyhow::Result<mfcdb::ModFileCacheRow> {
        mfcdb::get_mod_file_cache_by_id(&self.app.db, id)
            .await?
            .filter(|row| row.instance_id == *instance_id)
            .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.to_owned()).into())
    }

    /// Whether addons may change now: the pack is not locked, the instance is
    /// not being deleted, and no pending setup can stage or apply content
    /// beside them. A queued or preparing launch, or the first install of a
    /// new instance, does not hold them by itself: Play installs the game,
    /// Java and the loader, never the addon folders, and 2.0.40 allowed edits
    /// meanwhile. A setup that does write them (a pack install, update or
    /// repair) holds them through its `.setup` session. Install, publication
    /// and commit all check here, so an install admitted before Play still
    /// finishes while the game prepares.
    pub(super) async fn ensure_modpack_not_locked(
        &self,
        instance_id: InstanceId,
    ) -> anyhow::Result<()> {
        let (shortpath, config, preparing) = {
            let instances = self.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;
            let data = instance.data()?;
            ensure_pack_unlocked(data.config.modpack.as_ref())?;
            if matches!(data.state, super::run::LaunchState::Deleting) {
                bail!("This instance is being deleted, so its addons can't be changed.");
            }
            let preparing = matches!(
                data.state,
                super::run::LaunchState::Queued(_) | super::run::LaunchState::Preparing(_)
            );
            (instance.shortpath.clone(), data.config.clone(), preparing)
        };
        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath);
        let root = path.get_root();
        if addon_metadata(&root.join(".setup")).await?.is_some() {
            use super::modpack::session::{self, Session};
            let pending = Session::load(&root)
                .await
                .with_context(|| unreadable_setup_refusal(&config, PendingChange::Addons))?;
            let allowed = match &pending {
                // A 2.0.40 setup without a record that only waits for a version
                // install (an empty one, or a clean modpack-complete, like a
                // completed import, whose platform link 2.0.40 and Play never
                // download) never stages addon content, and 2.0.40 allowed
                // addon edits beside it. Read-only: not every caller holds the
                // gate.
                None => matches!(
                    session::classify_legacy(&root).await,
                    Ok(Some(
                        session::LegacyRequest::VersionOnly
                            | session::LegacyRequest::Complete { .. }
                    ))
                ),
                Some(pending) => {
                    setup_allows_addon_edits(&root, instance_id, &config, pending).await?
                }
            };
            if !allowed {
                // Play is already finishing it: "press Play" would not help.
                if preparing {
                    bail!(PREPARING_SETUP_REFUSAL);
                }
                bail!(
                    "{}",
                    pending_setup_refusal(
                        &root,
                        instance_id,
                        &config,
                        pending.as_ref(),
                        PendingChange::Addons
                    )
                    .await?
                );
            }
        }

        Ok(())
    }

    pub async fn list_mods(
        self,
        instance_id: InstanceId,
        addon_type: Option<domain::AddonType>,
    ) -> anyhow::Result<Vec<domain::Mod>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            bail!("instance {} is not valid", *instance_id);
        };

        let config = data.config.clone();
        drop(instances);

        let context = crate::managers::metadata::cache::modrinth::target_compatibility(
            self.app,
            crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
        )
        .await;
        let mod_sources = self.instance_cfg_mod_sources(&config).await?;
        // The installed jar's cached channel sets the update floor, so a
        // Release is only badged for a newer Release under the default policy.
        let has_update_for_paths = |kind: &str, paths: &str, installed: ModChannel| {
            context
                .as_ref()
                .is_some_and(|context| context.has_path(kind, paths, &mod_sources, installed))
        };

        let instance_id_val = *instance_id;
        let mut mods = mfcdb::get_instance_mods_full(&self.app.db, instance_id_val).await?;

        // Legacy artifact paths are not evidence for a context we never queried.
        for row in &mut mods {
            row.mr_update_paths = if let Some(context) = &context {
                carbon_repos::repos::mod_metadata::get_mr_update_context(
                    &self.app.db,
                    &row.meta_id,
                    &context.key(&row.addon_type),
                )
                .await?
                .filter(|v| v.cached_at > chrono::Utc::now() - chrono::Duration::days(1))
                .map(|v| v.update_paths)
            } else {
                None
            };
        }

        // Apply the optional addon-type filter in Rust BEFORE the duplicate
        // detection pass, so the modid counts are computed over exactly the
        // rows matching the requested addon type.
        if let Some(addon_type) = addon_type {
            let want = addon_type.to_db_string().to_string();
            mods.retain(|m| m.addon_type == want);
        }

        // Detect duplicated mods by grouping enabled mods by modid
        let mut modid_counts: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for m in &mods {
            // Only consider enabled mods with a modid
            if m.enabled {
                if let Some(ref modid) = m.modid {
                    *modid_counts.entry(modid.clone()).or_insert(0) += 1;
                }
            }
        }

        // Build set of modids that have duplicates (appear more than once)
        let duplicate_modids: std::collections::HashSet<String> = modid_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(modid, _)| modid)
            .collect();

        let mods = mods.into_iter().map(|m| {
            // Both platforms' badges hold the jar to its most stable label, so
            // a file one platform calls a Release is not badged for the other
            // platform's betas.
            let installed = installed_channel([
                m.cf_project_id
                    .and_then(|_| cached_channel("curseforge", &m.meta_id, m.cf_release_type)),
                m.mr_project_id
                    .as_ref()
                    .and_then(|_| cached_channel("modrinth", &m.meta_id, m.mr_release_type)),
            ]);
            let has_update_on = |platform: ModPlatform, listed: bool, paths: &Option<String>| {
                listed
                    && !mod_sources.platform_blacklist.contains(&platform)
                    && installed.is_some_and(|installed| {
                        has_update_for_paths(
                            &m.addon_type,
                            paths.as_deref().unwrap_or(""),
                            installed,
                        )
                    })
            };
            let has_curseforge_update = has_update_on(
                ModPlatform::Curseforge,
                m.cf_project_id.is_some(),
                &m.cf_update_paths,
            );
            let has_modrinth_update = has_update_on(
                ModPlatform::Modrinth,
                m.mr_project_id.is_some(),
                &m.mr_update_paths,
            );

            let is_duplicate = m.enabled
                && m.modid
                    .as_ref()
                    .map(|modid| duplicate_modids.contains(modid))
                    .unwrap_or(false);

            let curseforge = m
                .cf_project_id
                .map(|project_id| domain::CurseForgeModMetadata {
                    project_id: project_id as u32,
                    file_id: m.cf_file_id.unwrap() as u32,
                    name: m.cf_name.clone().unwrap(),
                    version: m.cf_version.clone().unwrap(),
                    urlslug: m.cf_urlslug.clone().unwrap(),
                    summary: m.cf_summary.clone().unwrap(),
                    authors: m.cf_authors.clone().unwrap(),
                    has_image: m.has_cf_image,
                });

            let modrinth = m
                .mr_project_id
                .clone()
                .map(|project_id| domain::ModrinthModMetadata {
                    project_id,
                    version_id: m.mr_version_id.clone().unwrap(),
                    title: m.mr_title.clone().unwrap(),
                    version: m.mr_version.clone().unwrap(),
                    urlslug: m.mr_urlslug.clone().unwrap(),
                    description: m.mr_description.clone().unwrap(),
                    authors: m.mr_authors.clone().unwrap(),
                    has_image: m.has_mr_image,
                });

            domain::Mod {
                id: m.id,
                filename: m.filename,
                enabled: m.enabled,
                addon_type: domain::AddonType::from_db_string(&m.addon_type)
                    .unwrap_or(domain::AddonType::Mods),
                metadata: Some(domain::ModFileMetadata {
                    id: m.meta_id,
                    modid: m.modid,
                    name: m.meta_name,
                    version: m.meta_version,
                    description: m.meta_description,
                    authors: m.meta_authors,
                    modloaders: m
                        .modloaders
                        .split(',')
                        // ignore unknown modloaders
                        .flat_map(|loader| ModLoaderType::try_from(loader).ok())
                        .collect::<Vec<_>>(),
                    sha_512: m.sha512,
                    sha_1: m.sha1,
                    murmur_2: m.murmur2,
                    has_image: m.has_local_image,
                }),
                curseforge,
                modrinth,
                has_update: has_curseforge_update || has_modrinth_update,
                is_duplicate,
                file_size: m.filesize as u64,
            }
        });

        Ok(mods.collect::<Vec<_>>())
    }

    async fn instance_cfg_mod_sources(
        self,
        config: &info::Instance,
    ) -> anyhow::Result<Cow<ModSources>> {
        match &config.mod_sources {
            Some(sources) => Ok(Cow::Borrowed(sources)),
            None => {
                let settings = self.app.settings_manager().get_settings().await?;

                let mut channels = ModChannelWithUsage::str_to_vec(&settings.mod_channels)?;
                ModChannelWithUsage::fixup_list(&mut channels);

                Ok(Cow::Owned(ModSources {
                    channels,
                    platform_blacklist: settings
                        .mod_platform_blacklist
                        .split(",")
                        .filter(|p| !p.is_empty())
                        .map(FromStr::from_str)
                        .collect::<Result<_, _>>()?,
                }))
            }
        }
    }

    pub async fn get_instance_mod_sources(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<ModSources> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.type_.data()?;
        let config = data.config.clone();
        drop(instances);

        Ok(self.instance_cfg_mod_sources(&config).await?.into_owned())
    }

    pub async fn enable_mod(
        self,
        instance_id: InstanceId,
        id: String,
        enabled: bool,
    ) -> anyhow::Result<()> {
        let _activity = self.app.runtime_lifecycle.admit("addon mutation")?;
        let _operation = self.lock_instance(instance_id).await;
        self.ensure_modpack_not_locked(instance_id).await?;
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let shortpath = instance.shortpath.clone();
        drop(instances);

        if !enabled {
            let instance_path = self
                .app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&shortpath);
            super::addon_retry::execute(
                &self.app.db,
                instance_id,
                &id,
                &instance_path,
                super::addon_retry::Action::Disable,
                || async {},
            )
            .await?;
            self.app
                .invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
            return Ok(());
        }

        let m = self.get_owned_mod_file(instance_id, &id).await?;

        let addon_type = domain::AddonType::from_db_string(&m.addon_type)
            .ok_or_else(|| anyhow!("unknown addon kind"))?;
        validate_addon_basename(&m.filename)?;

        ensure_toggleable(addon_type)?;

        let folder = {
            let instance_path = self
                .app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&shortpath);

            super::installer::publication::checked_folder(&instance_path, addon_type).await?
        };

        let enabled_path = folder.join(&m.filename);
        let disabled_path = folder.join(format!("{}.disabled", m.filename));

        if addon_metadata(&enabled_path).await?.is_some()
            && addon_metadata(&disabled_path).await?.is_some()
        {
            return Err(twin_copies_refusal(&folder, &m.filename, "enable"));
        }
        if enabled_path.exists() {
            bail!("mod is already enabled");
        }
        if !disabled_path.exists() {
            bail!("mod does not exist on disk");
        }
        tokio::fs::rename(disabled_path, enabled_path).await?;

        mfcdb::update_mod_file_enabled(
            &self.app.db,
            &id,
            enabled,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;

        self.app
            .invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
        Ok(())
    }

    /// Deletes the copy of an addon its row shows. `Some` when the folder
    /// held a second copy (`name` beside `name.disabled`), which stays and
    /// which the row now shows.
    pub async fn delete_mod(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<Option<KeptAddonCopy>> {
        let _activity = self.app.runtime_lifecycle.admit("addon mutation")?;
        let _operation = self.lock_instance(instance_id).await;
        self.ensure_modpack_not_locked(instance_id).await?;
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let shortpath = instance.shortpath.clone();
        drop(instances);

        let instance_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath);
        let outcome = super::addon_retry::execute(
            &self.app.db,
            instance_id,
            &id,
            &instance_path,
            super::addon_retry::Action::Delete,
            || async {
                #[cfg(test)]
                self.test_hooks.pause("addon_removed", instance_id).await;
            },
        )
        .await?;

        let kept = match outcome {
            super::addon_retry::Outcome::Done => None,
            super::addon_retry::Outcome::KeptTwin(twin) => {
                // The row now shows the copy that stayed. Read its own name,
                // version and size now, under the gate: the queued scan waits
                // while any game runs.
                let refreshed = self
                    .app
                    .meta_cache_manager()
                    .refresh_published_addon(
                        &_operation,
                        &twin.folder,
                        twin.filename.clone(),
                        twin.enabled,
                        twin.addon_type.clone(),
                    )
                    .await;
                if let Err(error) = refreshed {
                    tracing::warn!(
                        filename = %twin.filename,
                        "Could not read the addon copy a Delete left, rescanning: {error:#}"
                    );
                    self.app
                        .meta_cache_manager()
                        .queue_caching(
                            crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
                            false,
                        )
                        .await;
                }
                let (deleted, kept) = twin.names();
                Some(KeptAddonCopy { deleted, kept })
            }
        };

        // GC orphaned metadata
        self.app.meta_cache_manager().gc_mod_metadata().await;

        self.app
            .invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
        Ok(kept)
    }

    pub async fn disable_mod_retryable(
        self,
        instance_id: InstanceId,
        id: String,
        token: Option<String>,
    ) -> anyhow::Result<()> {
        if let Some(token) = token {
            self.retryable_file_mod(instance_id, id, super::addon_retry::Action::Disable, token)
                .await
        } else {
            self.enable_mod(instance_id, id, false).await
        }
    }

    pub async fn delete_mod_retryable(
        self,
        instance_id: InstanceId,
        id: String,
        token: Option<String>,
    ) -> anyhow::Result<Option<KeptAddonCopy>> {
        if let Some(token) = token {
            // The token path refuses two copies, so none is ever left.
            self.retryable_file_mod(instance_id, id, super::addon_retry::Action::Delete, token)
                .await
                .map(|()| None)
        } else {
            self.delete_mod(instance_id, id).await
        }
    }

    async fn retryable_file_mod(
        self,
        instance_id: InstanceId,
        id: String,
        action: super::addon_retry::Action,
        token: String,
    ) -> anyhow::Result<()> {
        let _activity = self.app.runtime_lifecycle.admit("addon mutation")?;
        let _operation = self.lock_instance(instance_id).await;
        self.ensure_modpack_not_locked(instance_id).await?;
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;
        let shortpath = instance.shortpath.clone();
        drop(instances);
        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath);
        super::addon_retry::execute_with_token(
            &self.app.db,
            &self.addon_retry_receipts,
            instance_id,
            &id,
            &path,
            action,
            &token,
            || async {
                #[cfg(test)]
                if action == super::addon_retry::Action::Delete {
                    self.test_hooks.pause("addon_removed", instance_id).await;
                }
            },
        )
        .await?;
        if action == super::addon_retry::Action::Delete {
            self.app.meta_cache_manager().gc_mod_metadata().await;
        }
        self.app
            .invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
        Ok(())
    }

    pub async fn install_curseforge_mod(
        self,
        instance_id: InstanceId,
        project_id: u32,
        file_id: u32,
        install_deps: bool,
        replaces_mod_id: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let installer = CurseforgeModInstaller::create(self.app, project_id, file_id)
            .await?
            .into_installer();

        let task_id = installer
            .install(self.app, instance_id, install_deps, replaces_mod_id)
            .await?;

        Ok(task_id)
    }

    pub async fn install_latest_curseforge_mod(
        self,
        instance_id: InstanceId,
        project_id: u32,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let version = {
            let instances = self.instances.read().await;
            instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?
                .data()?
                .config
                .game_configuration
                .version
                .as_ref()
                .ok_or(anyhow!("Can't find valid version"))?
                .clone()
        };

        let project = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod(ModParameters {
                mod_id: project_id.try_into()?,
            })
            .await?;

        let (version, modloader) = match version {
            domain::info::GameVersion::Custom(_) => todo!("Unsupported"),
            domain::info::GameVersion::Standard(version) => {
                let first_modloader = version.modloaders.iter().next();

                let modloader = match project.data.class_id {
                    Some(carbon_platforms::curseforge::ClassId::Mods) | None => first_modloader,
                    _ => None,
                };

                (version.release.clone(), modloader.map(|v| v.type_.into()))
            }
        };

        let file_id = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: project_id.try_into()?,
                query: ModFilesParametersQuery {
                    game_version: Some(version.clone()),
                    game_version_type_id: None,
                    mod_loader_type: modloader,
                    index: None,
                    page_size: Some(200),
                },
            })
            .await?
            .data
            .iter()
            .find(|value| value.game_versions.contains(&version))
            .ok_or(anyhow::anyhow!(
                "Can't find a valid version for this instance"
            ))?
            .id
            .try_into()?;

        let installer = CurseforgeModInstaller::create(self.app, project_id, file_id)
            .await?
            .into_installer();

        let task_id = installer.install(self.app, instance_id, true, None).await?;

        Ok(task_id)
    }

    pub async fn install_modrinth_mod(
        &self,
        instance_id: InstanceId,
        project_id: String,
        version_id: String,
        install_deps: bool,
        replaces_mod_id: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let installer = ModrinthModInstaller::create(self.app, project_id, version_id)
            .await?
            .into_installer();

        let task_id = installer
            .install(self.app, instance_id, install_deps, replaces_mod_id)
            .await?;

        Ok(task_id)
    }

    pub async fn install_latest_modrinth_mod(
        &self,
        instance_id: InstanceId,
        project_id: String,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let version = {
            let instances = self.instances.read().await;
            instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?
                .data()?
                .config
                .game_configuration
                .version
                .as_ref()
                .ok_or(anyhow!("Can't find valid version"))?
                .clone()
        };

        let project = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project(ProjectID(project_id.clone()))
            .await?;

        let (version, modloader) = match version {
            domain::info::GameVersion::Custom(_) => todo!("Unsupported"),
            domain::info::GameVersion::Standard(version) => {
                let first_modloader = version.modloaders.iter().next();

                let modloader = match project.project_type {
                    carbon_platforms::modrinth::project::ProjectType::Mod => first_modloader,
                    _ => None,
                };

                (
                    version.release.clone(),
                    modloader.map(|v| vec![v.type_.to_string()]),
                )
            }
        };

        let version_id = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(project_id.clone()),
                game_versions: Some(Vec::from([version.clone()])),
                loaders: modloader,
                limit: None,
                offset: None,
            })
            .await?
            .get(0)
            .ok_or(anyhow!("Can't find a valid version for this instance"))?
            .id
            .clone();

        let installer = ModrinthModInstaller::create(self.app, project_id, version_id)
            .await?
            .into_installer();

        let task_id = installer.install(self.app, instance_id, true, None).await?;

        Ok(task_id)
    }

    /// Attempt to find an update for a mod respecting the instance's (and the global) channel preference.
    pub async fn find_mod_update(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<Option<RemoteVersion>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.type_.data()?;
        let config = data.config.clone();
        drop(instances);

        let owned = self.get_owned_mod_file(instance_id, &id).await?;

        let Some(GameVersion::Standard(version)) = &config.game_configuration.version else {
            bail!(
                "Instance uses a custom game version file. Cannot resolve minecraft version for mod installation"
            );
        };

        let context = crate::managers::metadata::cache::modrinth::target_compatibility(
            self.app,
            crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
        )
        .await
        .context("Unknown update compatibility context")?;
        let mod_sources = self.instance_cfg_mod_sources(&config).await?;

        let lookup_id = id.clone();
        let row = mfcdb::get_instance_mod_update_ids(&self.app.db, &lookup_id)
            .await?
            .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let platforms = self.app.modplatforms_manager();
        resolve_mod_update(
            &platforms.curseforge,
            &platforms.modrinth,
            &row,
            version,
            &owned.addon_type,
            &context,
            &mod_sources,
        )
        .await
    }

    pub async fn update_mod(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let update = self.find_mod_update(instance_id, id.clone()).await?;

        match update {
            Some(RemoteVersion::Curseforge(file)) => {
                self.install_curseforge_mod(
                    instance_id,
                    file.mod_id as u32,
                    file.id as u32,
                    false,
                    Some(id),
                )
                .await
            }
            Some(RemoteVersion::Modrinth(version)) => {
                self.install_modrinth_mod(
                    instance_id,
                    version.project_id,
                    version.id,
                    false,
                    Some(id),
                )
                .await
            }
            None => Err(anyhow!(
                "unable to find newer mod version in available update channels"
            )),
        }
    }

    pub async fn update_curseforge_mod(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            bail!("instance is in an invalid state");
        };

        let Some(GameVersion::Standard(version)) = data.config.game_configuration.version.clone()
        else {
            bail!(
                "Instance uses a custom game version file. Cannot resolve minecraft version for mod installation"
            );
        };

        drop(instances);

        self.get_owned_mod_file(instance_id, &id).await?;
        let lookup_id = id.clone();
        let cf = mfcdb::get_instance_mod_cf_ids(&self.app.db, &lookup_id)
            .await?
            .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let cf_project_id = cf.cf_project_id.ok_or_else(|| {
            anyhow!(
                "Attempted to use update_curseforge_mod to update a mod not available on curseforge"
            )
        })?;
        let cf_file_id = cf
            .cf_file_id
            .expect("cf fileId is present whenever cf projectId is");

        let mod_files = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: cf_project_id,
                query: ModFilesParametersQuery {
                    game_version: Some(version.release),
                    game_version_type_id: None,
                    mod_loader_type: version.modloaders.iter().next().map(|v| v.type_.into()),
                    index: None,
                    page_size: None,
                },
            })
            .await?;

        let version = mod_files.data.into_iter().next();

        let Some(version) = version else {
            bail!("unable to find newer mod version");
        };

        if version.id == cf_file_id {
            bail!("unable to find newer mod version");
        }

        self.install_curseforge_mod(
            instance_id,
            version.mod_id as u32,
            version.id as u32,
            false,
            Some(id),
        )
        .await
    }

    pub async fn update_modrinth_mod(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            bail!("instance is in an invalid state");
        };

        let Some(GameVersion::Standard(version)) = data.config.game_configuration.version.clone()
        else {
            bail!(
                "Instance uses a custom game version file. Cannot resolve minecraft version for mod installation"
            );
        };

        drop(instances);

        self.get_owned_mod_file(instance_id, &id).await?;
        let lookup_id = id.clone();
        let mr = mfcdb::get_instance_mod_mr_ids(&self.app.db, &lookup_id)
            .await?
            .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let mr_project_id = mr.mr_project_id.ok_or_else(|| {
            anyhow!(
                "Attempted to use update_modrinth_mod to update a mod not available on modrinth"
            )
        })?;
        let mr_version_id = mr
            .mr_version_id
            .expect("mr versionId is present whenever mr projectId is");

        let mod_files = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(mr_project_id),
                game_versions: Some(vec![version.release]),
                loaders: Some(
                    version
                        .modloaders
                        .iter()
                        .map(|ml| ml.type_.to_string())
                        .collect(),
                ),
                limit: None,
                offset: None,
            })
            .await?;

        let version = mod_files.0.into_iter().next();

        let Some(version) = version else {
            bail!("unable to find newer mod version");
        };

        if version.id == mr_version_id {
            bail!("unable to find newer mod version");
        }

        self.install_modrinth_mod(instance_id, version.project_id, version.id, false, Some(id))
            .await
    }

    pub async fn get_mod_icon(
        &self,
        instance_id: InstanceId,
        mod_id: String,
        platformid: i32,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let instances = self.instances.read().await;
        let _ = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        self.get_owned_mod_file(instance_id, &mod_id).await?;
        let lookup_id = mod_id.clone();
        let r = mfcdb::get_instance_mod_icon_data(&self.app.db, &lookup_id)
            .await?
            .ok_or(InvalidModIdError(mod_id))?;

        let logo_image = match platformid {
            0 => r.local_data,
            1 => r.cf_data,
            2 => r.mr_data,
            _ => bail!("unsupported platform"),
        };

        Ok(logo_image)
    }
}

#[derive(Error, Debug)]
#[error("invalid mod id '{1}' given for instance '{0}'")]
pub struct InvalidInstanceModIdError(pub(super) InstanceId, pub(super) String);

#[derive(Error, Debug)]
#[error("invalid mod id '{0}'")]
pub struct InvalidModIdError(String);

/// Picks the update `find_mod_update` offers for an installed addon, from what
/// the platforms report for its project. Kept apart from the instance lookup so
/// tests can point the platform clients at a stand-in API.
async fn resolve_mod_update(
    curseforge: &crate::managers::modplatforms::curseforge::CurseForge,
    modrinth: &crate::managers::modplatforms::modrinth::Modrinth,
    row: &mfcdb::ModUpdateIdsRow,
    version: &info::StandardVersion,
    addon_type: &str,
    context: &UpdateContext,
    mod_sources: &ModSources,
) -> anyhow::Result<Option<RemoteVersion>> {
    let cf = row.cf_project_id.zip(row.cf_file_id);
    let mr = row.mr_project_id.clone().zip(row.mr_version_id.clone());

    let mut versions = Vec::new();

    if let Some((project_id, _)) = cf {
        let response = curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: project_id,
                query: ModFilesParametersQuery {
                    game_version: Some(version.release.clone()),
                    game_version_type_id: None,
                    mod_loader_type: (addon_type == "mods")
                        .then(|| version.modloaders.iter().next().map(|v| v.type_.into()))
                        .flatten(),
                    index: None,
                    page_size: None,
                },
            })
            .await?;

        versions.extend(
            response
                .data
                .into_iter()
                .map(|f| RemoteVersion::Curseforge(f)),
        );
    }

    if let Some((project_id, _)) = &mr {
        let response = modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(project_id.clone()),
                game_versions: Some(vec![version.release.clone()]),
                loaders: Some(context.request_loaders(addon_type)),
                limit: None,
                offset: None,
            })
            .await?;

        versions.extend(response.into_iter().map(|v| RemoteVersion::Modrinth(v)));
    }

    versions.retain(|candidate| match candidate {
        RemoteVersion::Modrinth(v) => context.compatible(addon_type, &v.game_versions, &v.loaders),
        RemoteVersion::Curseforge(v) => context.cf_compatible(addon_type, &v.game_versions),
    });

    let mut fetched = Vec::new();
    if let Some((project_id, file_id)) = cf {
        let version = curseforge
            .get_mod_file(ModFileParameters {
                mod_id: project_id,
                file_id,
            })
            .await
            .map(|response| RemoteVersion::Curseforge(response.data));
        fetched.push((
            version,
            InstalledId::Curseforge {
                project_id,
                file_id,
            },
        ));
    }
    if let Some((project_id, version_id)) = mr {
        let version = modrinth
            .get_version(carbon_platforms::modrinth::search::VersionID(
                version_id.clone(),
            ))
            .await
            .map(RemoteVersion::Modrinth);
        fetched.push((
            version,
            InstalledId::Modrinth {
                project_id,
                version_id,
            },
        ));
    }
    let cached = installed_channel([
        row.cf_release_type
            .and_then(|channel| ModChannel::try_from(channel).ok()),
        row.mr_release_type
            .and_then(|channel| ModChannel::try_from(channel).ok()),
    ]);
    let installed = installed_states(fetched, cached)?;

    Ok(choose_update(versions, &installed, mod_sources))
}

/// The installed jar as each platform that listed it reports it now.
///
/// `cached` is the jar's channel from the metadata cache, the same one the
/// update badge reads. If no platform's cached value is readable, the live
/// channel stands in, or Stable (the most cautious floor) when no platform
/// serves the file any more. A file a platform no longer has (404) is `Gone`,
/// so the update that replaces a pulled build still works; any other failure
/// still fails the update.
fn installed_states(
    fetched: Vec<(anyhow::Result<RemoteVersion>, InstalledId)>,
    cached: Option<ModChannel>,
) -> anyhow::Result<Vec<Installed>> {
    let mut listed = Vec::new();
    let mut gone = Vec::new();
    for (version, id) in fetched {
        match version {
            Ok(version) => listed.push(version),
            Err(error) if is_not_found(&error) => {
                tracing::info!(
                    ?id,
                    "the installed file is no longer available on its platform; another file of its project can replace it"
                );
                gone.push(id);
            }
            Err(error) => return Err(error),
        }
    }
    let channel = cached
        .or_else(|| listed.iter().map(RemoteVersion::channel).max())
        .unwrap_or(ModChannel::Stable);
    Ok(listed
        .into_iter()
        .map(|version| Installed::Present { version, channel })
        .chain(gone.into_iter().map(|id| Installed::Gone { id, channel }))
        .collect())
}

/// A platform's cached channel for an installed file, or `None` (logged) if
/// the stored value is not a channel.
fn cached_channel(platform: &str, meta_id: &str, release_type: Option<i32>) -> Option<ModChannel> {
    let release_type = release_type?;
    ModChannel::try_from(release_type)
        .inspect_err(|_| {
            tracing::error!(
                "Invalid ModChannel in database for {platform} entry {meta_id}: {release_type}"
            );
        })
        .ok()
}

/// Whether a platform request failed because the requested file or version
/// does not exist (anymore).
fn is_not_found(error: &anyhow::Error) -> bool {
    use crate::error::request::{RequestError, RequestErrorDetails};
    error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<RequestError>(),
            Some(RequestError {
                error: RequestErrorDetails::UnexpectedStatus { status, .. },
                ..
            }) if *status == reqwest::StatusCode::NOT_FOUND || *status == reqwest::StatusCode::GONE
        )
    })
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::managers::instance::InstanceVersionSource;
    use crate::{api::keys::instance::INSTANCE_MODS, domain::instance::info};

    #[tokio::test]
    #[ignore]
    async fn test_mod_metadata() -> anyhow::Result<()> {
        dbg!();
        let app = crate::setup_managers_for_test().await;
        let group = app.instance_manager().get_default_group().await?;
        let instance_id = app
            .instance_manager()
            .create_instance(
                group,
                String::from("test"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.16.5"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        app.meta_cache_manager()
            .cache_with_priority(crate::managers::metadata::cache::CacheEntityId::Instance(
                instance_id,
            ))
            .await;

        app.instance_manager()
            .install_curseforge_mod(instance_id, 331723, 4022327, true, None)
            .await?;

        // first invalidation will happen when the mod is scanned locally
        app.wait_for_invalidation(INSTANCE_MODS).await?;

        let mods = app.instance_manager().list_mods(instance_id, None).await?;
        dbg!(&mods);
        assert_ne!(mods.get(0), None);

        // second invalidation will happen when the curseforge metadata is fetched
        app.wait_for_invalidation(INSTANCE_MODS).await?;

        let mods = app.instance_manager().list_mods(instance_id, None).await?;
        dbg!(&mods);
        assert_ne!(mods[0].curseforge, None);

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{InvalidInstanceModIdError, ensure_toggleable, remove_addon_from_disk};
    use crate::domain::instance::AddonType;
    use crate::domain::instance::info;
    use crate::managers::AppInner;
    use crate::managers::instance::installer::{DependencyIterator, Installer, ResourceInstaller};
    use crate::managers::instance::{InstanceData, InstanceId, InstanceVersionSource};
    use carbon_net::Downloadable;
    use carbon_platforms::ModChannel;
    use carbon_repos::dbtypes::DbDateTime;
    use carbon_repos::repos::mod_file_cache as mfcdb;
    use carbon_repos::repos::mod_metadata as metarepo;
    use carbon_rt_path::InstancePath;
    use chrono::Utc;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::Arc;

    pub(crate) async fn addon_fixture(
        app: &Arc<AppInner>,
        name: &str,
        addon_type: AddonType,
        filename: &str,
        contents: &[u8],
    ) -> (InstanceId, mfcdb::ModFileCacheRow, PathBuf) {
        let group = app.instance_manager().get_default_group().await.unwrap();
        let instance_id = app
            .instance_manager()
            .create_instance(
                group,
                name.to_owned(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.20.1"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await
            .unwrap();
        let shortpath = app.instance_manager().instances.read().await[&instance_id]
            .shortpath
            .clone();
        let instance_path = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath);
        let folder = addon_type.get_folder_path(&instance_path);
        let path = folder.join(filename);
        tokio::fs::create_dir_all(&folder).await.unwrap();
        if addon_type == AddonType::Worlds {
            tokio::fs::create_dir_all(&path).await.unwrap();
            tokio::fs::write(path.join("level.dat"), contents)
                .await
                .unwrap();
        } else {
            tokio::fs::write(&path, contents).await.unwrap();
        }

        let metadata_id = format!("{name}-metadata");
        let now = DbDateTime(Utc::now().fixed_offset());
        metarepo::insert_metadata(
            &app.db,
            &metadata_id,
            0,
            name.as_bytes(),
            name.as_bytes(),
            "vanilla",
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await
        .unwrap();
        mfcdb::upsert_mod_file_cache(
            &app.db,
            *instance_id,
            filename.to_owned(),
            0,
            true,
            addon_type.to_db_string().to_owned(),
            metadata_id,
            now,
        )
        .await
        .unwrap();
        let row = mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *instance_id, filename)
            .await
            .unwrap()
            .unwrap();
        (instance_id, row, path)
    }

    /// A failed or cancelled version-only install keeps its setup until that
    /// version installs. Addons can still be added, toggled and removed
    /// beside it, in either phase, and while it installs, as in 2.0.40; pack
    /// setups, apply markers and a deletion still hold them.
    #[tokio::test]
    async fn versiononly_pending_install_keeps_addon_edits() {
        use crate::domain::vtask::VisualTaskId;
        use crate::managers::instance::modpack::session::{Kind, Phase, Producer, RECORD, Session};
        use crate::managers::instance::run::LaunchState;
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) =
            addon_fixture(&app, "versiononly-addons", AddonType::Mods, "a.jar", b"a").await;
        let shortpath = app.instance_manager().instances.read().await[&id]
            .shortpath
            .clone();
        let root = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_root();
        let record = root.join(".setup").join(RECORD);
        let created = Session::load(&root).await.unwrap().unwrap();
        assert_eq!(
            (created.producer, created.kind, created.phase),
            (Producer::Create, Kind::VersionOnly, Phase::Reserved)
        );
        let write = |session: &Session| {
            std::fs::write(&record, serde_json::to_vec(session).unwrap()).unwrap()
        };
        let disabled = path.with_file_name("a.jar.disabled");
        let set_state = |state| {
            let app = app.clone();
            async move {
                app.instance_manager()
                    .instances
                    .write()
                    .await
                    .get_mut(&id)
                    .unwrap()
                    .data_mut()
                    .unwrap()
                    .state = state;
            }
        };

        // A failed first install, and a version change that failed before or
        // after staging.
        for (producer, phase) in [
            (Producer::Create, Phase::Committed),
            (Producer::Settings, Phase::Reserved),
            (Producer::Settings, Phase::Committed),
        ] {
            let mut pending = created.clone();
            pending.producer = producer;
            pending.phase = phase;
            write(&pending);
            // The gate an install passes before it downloads anything.
            app.instance_manager()
                .ensure_modpack_not_locked(id)
                .await
                .unwrap_or_else(|error| panic!("{producer:?} {phase:?}: {error:#}"));
            app.instance_manager()
                .enable_mod(id, row.id.clone(), false)
                .await
                .unwrap_or_else(|error| panic!("{producer:?} {phase:?}: {error:#}"));
            assert!(
                disabled.exists() && !path.exists(),
                "{producer:?} {phase:?}"
            );
            app.instance_manager()
                .enable_mod(id, row.id.clone(), true)
                .await
                .unwrap_or_else(|error| panic!("{producer:?} {phase:?}: {error:#}"));
            assert!(
                path.exists() && !disabled.exists(),
                "{producer:?} {phase:?}"
            );
        }

        // A completed import that keeps its platform link, imported locked and
        // unlocked since, whose first install failed or was cancelled: Play
        // never downloads its pack request, so addon edits stay allowed, as in
        // 2.0.40.
        let link = info::Modpack::Modrinth(info::ModrinthModpack {
            project_id: "linked".into(),
            version_id: "A".into(),
        });
        let set_modpack = |modpack: Option<info::ModpackInfo>| {
            let app = app.clone();
            async move {
                app.instance_manager()
                    .instances
                    .write()
                    .await
                    .get_mut(&id)
                    .unwrap()
                    .data_mut()
                    .unwrap()
                    .config
                    .modpack = modpack;
            }
        };
        set_modpack(Some(info::ModpackInfo {
            modpack: link.clone(),
            locked: false,
        }))
        .await;
        let mut imported = created.clone();
        imported.producer = Producer::Import;
        imported.kind = Kind::ImportedComplete;
        imported.phase = Phase::Committed;
        let mut installed =
            crate::managers::instance::schema::parse_instance_config(&created.installed_config)
                .unwrap();
        installed.modpack = Some(info::ModpackInfo {
            modpack: link.clone(),
            locked: true,
        });
        imported.installed_config =
            crate::managers::instance::schema::make_instance_config(installed).unwrap();
        imported.requested_pack = Some("linked A".into());
        write(&imported);
        let request = root.join(".setup/change-pack-version.json");
        std::fs::write(&request, b"linked A").unwrap();
        app.instance_manager()
            .enable_mod(id, row.id.clone(), false)
            .await
            .unwrap();
        assert!(disabled.exists() && !path.exists());
        app.instance_manager()
            .enable_mod(id, row.id.clone(), true)
            .await
            .unwrap();
        assert!(path.exists() && !disabled.exists());
        // A request that is not the record's own is one Play refuses.
        std::fs::write(&request, b"linked B").unwrap();
        let error = app
            .instance_manager()
            .enable_mod(id, row.id.clone(), false)
            .await
            .expect_err("a changed pack request");
        assert!(error.to_string().contains("Press Play"), "{error}");
        std::fs::remove_file(&request).unwrap();
        app.instance_manager()
            .enable_mod(id, row.id.clone(), false)
            .await
            .expect_err("a missing pack request");
        assert!(path.exists() && !disabled.exists());
        // So is a setup for another pack, and a locked pack keeps its lock.
        std::fs::write(&request, b"linked A").unwrap();
        set_modpack(Some(info::ModpackInfo {
            modpack: info::Modpack::Modrinth(info::ModrinthModpack {
                project_id: "linked".into(),
                version_id: "B".into(),
            }),
            locked: false,
        }))
        .await;
        app.instance_manager()
            .enable_mod(id, row.id.clone(), false)
            .await
            .expect_err("another pack");
        set_modpack(Some(info::ModpackInfo {
            modpack: link,
            locked: true,
        }))
        .await;
        let error = app
            .instance_manager()
            .enable_mod(id, row.id.clone(), false)
            .await
            .expect_err("a locked pack");
        assert_eq!(error.to_string(), "Modpack is locked");
        assert!(path.exists() && !disabled.exists());
        set_modpack(None).await;
        std::fs::remove_file(&request).unwrap();

        let mut committed = created.clone();
        committed.producer = Producer::Settings;
        committed.phase = Phase::Committed;
        let refused = |what: String| {
            let app = app.clone();
            let id_ = row.id.clone();
            let path = path.clone();
            async move {
                let error = app
                    .instance_manager()
                    .enable_mod(id, id_, false)
                    .await
                    .expect_err(&what);
                assert!(path.exists(), "{what}");
                error.to_string()
            }
        };
        // A pack setup.
        let mut pack = committed.clone();
        pack.kind = Kind::Pack;
        write(&pack);
        let message = refused("pack setup".into()).await;
        assert!(message.contains("Press Play"), "{message}");
        // Apply or pack markers beside a version-only setup.
        write(&committed);
        for marker in [
            "change-pack-version.json",
            "repair",
            "apply-started",
            "representation-transitions",
            "staging-packinfo.json",
        ] {
            let marker_path = root.join(".setup").join(marker);
            std::fs::write(&marker_path, b"").unwrap();
            refused(marker.into()).await;
            std::fs::remove_file(marker_path).unwrap();
        }
        // A queued or running preparation of this version-only setup (the
        // first install of a new instance, or Play) never touches the addon
        // folders: edits go ahead meanwhile, as in 2.0.40. While it finishes a
        // pack setup they wait, and the reason says so rather than "press
        // Play".
        for state in [
            LaunchState::Queued(VisualTaskId(900)),
            LaunchState::Preparing(VisualTaskId(900)),
        ] {
            let name = format!("{state:?}");
            set_state(state).await;
            app.instance_manager()
                .enable_mod(id, row.id.clone(), false)
                .await
                .unwrap_or_else(|error| panic!("{name}: {error:#}"));
            assert!(disabled.exists() && !path.exists(), "{name}");
            app.instance_manager()
                .enable_mod(id, row.id.clone(), true)
                .await
                .unwrap_or_else(|error| panic!("{name}: {error:#}"));
            assert!(path.exists() && !disabled.exists(), "{name}");
            write(&pack);
            let message = refused(format!("{name} with a pack setup")).await;
            assert_eq!(message, super::PREPARING_SETUP_REFUSAL, "{name}");
            write(&committed);
        }
        // An instance being deleted.
        set_state(LaunchState::Deleting).await;
        let message = refused("Deleting".into()).await;
        assert!(message.contains("being deleted"), "{message}");
        set_state(LaunchState::Inactive { failed_task: None }).await;

        app.instance_manager()
            .delete_mod(id, row.id.clone())
            .await
            .unwrap();
        assert!(!path.exists() && !disabled.exists());
    }

    #[tokio::test]
    async fn task05_delete_missing_world_preserves_literal_disabled_save() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) =
            addon_fixture(&app, "stale-world", AddonType::Worlds, "World", b"old").await;
        tokio::fs::remove_dir_all(&path).await.unwrap();
        let independent = path.with_file_name("World.disabled");
        tokio::fs::create_dir(&independent).await.unwrap();
        tokio::fs::write(independent.join("level.dat"), b"independent")
            .await
            .unwrap();
        app.instance_manager()
            .delete_mod(id, row.id.clone())
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read(independent.join("level.dat"))
                .await
                .unwrap(),
            b"independent"
        );
        assert!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn task05_delete_exact_world_preserves_other_spelling() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) =
            addon_fixture(&app, "exact", AddonType::Worlds, "World", b"world").await;
        let other = path.with_file_name("World.disabled");
        std::fs::create_dir(&other).unwrap();
        std::fs::write(other.join("level.dat"), b"other").unwrap();
        app.instance_manager().delete_mod(id, row.id).await.unwrap();
        assert!(!path.exists());
        assert_eq!(std::fs::read(other.join("level.dat")).unwrap(), b"other");
    }

    #[tokio::test]
    async fn task05_delete_literal_disabled_world_is_exact() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) = addon_fixture(
            &app,
            "literal",
            AddonType::Worlds,
            "World.disabled",
            b"world",
        )
        .await;
        let other = path.with_file_name("World.disabled.disabled");
        std::fs::create_dir(&other).unwrap();
        app.instance_manager().delete_mod(id, row.id).await.unwrap();
        assert!(!path.exists());
        assert!(other.is_dir());
    }

    #[tokio::test]
    async fn task05_delete_invalid_kind_and_name_keep_row_and_bytes() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) =
            addon_fixture(&app, "invalid", AddonType::Mods, "safe.jar", b"safe").await;
        // The row itself must carry the kind this build does not know (as one
        // written by a newer version would). An upsert cannot do that any
        // more: the kind is part of the row's key, so it adds a second row.
        use carbon_repos::db_exec::WriteAccess;
        let row_id = row.id.clone();
        app.db
            .write(move |conn| {
                conn.execute(
                    "UPDATE ModFileCache SET addonType = 'unknown' WHERE id = ?1",
                    [row_id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(
            app.instance_manager()
                .delete_mod(id, row.id.clone())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"safe");
        assert!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                .await
                .unwrap()
                .is_some()
        );
        for name in ["../safe.jar", "x/y", ".", "..", "", "nul\0.jar"] {
            assert!(super::validate_addon_basename(name).is_err());
        }
        // Only Windows reads these as a separator, a stream or an alias.
        for name in ["x\\y", "C:foo", ".. ", "World."] {
            assert_eq!(
                super::validate_addon_basename(name).is_err(),
                cfg!(windows),
                "{name}"
            );
        }
    }

    /// A name Linux and macOS allow (Finder stores a typed `/` as `:`) is
    /// listed by the scanner, so its toggle and Delete work there as well.
    #[cfg(not(windows))]
    #[tokio::test]
    async fn names_only_windows_refuses_toggle_and_delete_elsewhere() {
        let app = crate::setup_managers_for_test().await;
        for (fixture, kind, name) in [
            ("posix-colon", AddonType::ResourcePacks, "Faithful 1:2.zip"),
            ("posix-backslash", AddonType::Mods, "back\\slash.jar"),
            ("posix-dot", AddonType::Worlds, "Trailing dot."),
        ] {
            let (id, row, path) = addon_fixture(&app, fixture, kind, name, b"bytes").await;
            if kind != AddonType::Worlds {
                app.instance_manager()
                    .enable_mod(id, row.id.clone(), false)
                    .await
                    .unwrap();
                assert!(path.with_file_name(format!("{name}.disabled")).is_file());
                app.instance_manager()
                    .enable_mod(id, row.id.clone(), true)
                    .await
                    .unwrap();
                assert!(path.is_file());
            }
            app.instance_manager()
                .delete_mod(id, row.id.clone())
                .await
                .unwrap();
            assert!(std::fs::symlink_metadata(&path).is_err(), "{name}");
            assert!(
                mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn task05_delete_links_preserve_target_and_row() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) =
            addon_fixture(&app, "link", AddonType::Worlds, "World", b"safe").await;
        let target = path.with_file_name("Target");
        std::fs::rename(&path, &target).unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert!(
            app.instance_manager()
                .delete_mod(id, row.id.clone())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(target.join("level.dat")).unwrap(), b"safe");
        assert!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                .await
                .unwrap()
                .is_some()
        );
    }

    /// A mod or pack that is a link to a jar or zip kept elsewhere (shared
    /// between instances) toggles by renaming the link, and Delete removes
    /// the link only, as 2.0.40 did; what it points to is kept.
    #[cfg(unix)]
    #[tokio::test]
    async fn linked_file_addons_toggle_and_delete_only_the_link() {
        let app = crate::setup_managers_for_test().await;
        let shared = tempfile::tempdir().unwrap();
        for (fixture, kind, name, disable_first) in [
            ("link-mod", AddonType::Mods, "shared.jar", false),
            ("link-pack", AddonType::ResourcePacks, "shared.zip", false),
            ("link-disabled", AddonType::Mods, "other.jar", true),
        ] {
            let (id, row, path) = addon_fixture(&app, fixture, kind, name, b"local").await;
            let target = shared.path().join(fixture);
            std::fs::write(&target, b"shared bytes").unwrap();
            std::fs::remove_file(&path).unwrap();
            std::os::unix::fs::symlink(&target, &path).unwrap();
            let disabled = path.with_file_name(format!("{name}.disabled"));
            if disable_first {
                app.instance_manager()
                    .enable_mod(id, row.id.clone(), false)
                    .await
                    .unwrap();
                assert!(
                    std::fs::symlink_metadata(&disabled)
                        .unwrap()
                        .file_type()
                        .is_symlink()
                );
            }
            app.instance_manager()
                .delete_mod(id, row.id.clone())
                .await
                .unwrap();
            assert!(std::fs::symlink_metadata(&path).is_err(), "{fixture}");
            assert!(std::fs::symlink_metadata(&disabled).is_err(), "{fixture}");
            assert_eq!(std::fs::read(&target).unwrap(), b"shared bytes");
            assert!(
                mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }

    /// On Windows a mod or pack can be a junction (`mklink /J`, which needs
    /// no extra rights) named like the file. Windows removes a junction as a
    /// folder; Delete removes only the junction, never what it points to.
    #[cfg(windows)]
    #[tokio::test]
    async fn a_junction_named_like_a_file_addon_deletes_only_the_junction() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) = addon_fixture(
            &app,
            "junction-pack",
            AddonType::ResourcePacks,
            "shared.zip",
            b"local",
        )
        .await;
        let shared = tempfile::tempdir().unwrap();
        let target = shared.path().join("pack");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("pack.mcmeta"), b"{}").unwrap();
        std::fs::remove_file(&path).unwrap();
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(&path)
            .arg(&target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed for {}", path.display());
        app.instance_manager()
            .delete_mod(id, row.id.clone())
            .await
            .unwrap();
        assert!(std::fs::symlink_metadata(&path).is_err());
        assert_eq!(std::fs::read(target.join("pack.mcmeta")).unwrap(), b"{}");
        assert!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    /// `twin.jar` beside `twin.jar.disabled` is one row. Toggles explain the
    /// two copies; Delete removes the copy the row shows, says which copy is
    /// left, and the row then shows that copy's own details at once (the
    /// queued scan waits while any game runs). A second Delete removes it,
    /// as in 2.0.40.
    #[tokio::test]
    async fn delete_with_both_spellings_removes_one_copy_per_click() {
        let app = crate::setup_managers_for_test().await;
        for (fixture, shows_enabled) in [("twins-enabled", true), ("twins-disabled", false)] {
            let (id, row, enabled) =
                addon_fixture(&app, fixture, AddonType::Mods, "twin.jar", b"enabled copy").await;
            let disabled = enabled.with_file_name("twin.jar.disabled");
            std::fs::write(&disabled, b"disabled copy!").unwrap();
            // Settle the scan create_instance queued before setting the row.
            app.meta_cache_manager()
                .override_caching_and_wait(
                    crate::managers::metadata::cache::CacheEntityId::Instance(id),
                    false,
                    false,
                )
                .await
                .unwrap();
            // The row describes the copy it shows.
            let (shown_bytes, kept_bytes) = if shows_enabled {
                (&b"enabled copy"[..], &b"disabled copy!"[..])
            } else {
                (&b"disabled copy!"[..], &b"enabled copy"[..])
            };
            mfcdb::upsert_mod_file_cache(
                &app.db,
                *id,
                "twin.jar".into(),
                shown_bytes.len() as i32,
                shows_enabled,
                "mods".into(),
                row.metadata_id.clone(),
                DbDateTime(Utc::now().fixed_offset()),
            )
            .await
            .unwrap();
            for enable in [true, false] {
                let error = app
                    .instance_manager()
                    .enable_mod(id, row.id.clone(), enable)
                    .await
                    .unwrap_err();
                assert!(
                    format!("{error:#}").contains("Both twin.jar and twin.jar.disabled"),
                    "{error:#}"
                );
            }
            let left = app
                .instance_manager()
                .delete_mod(id, row.id.clone())
                .await
                .unwrap();
            let (gone, kept) = if shows_enabled {
                (&enabled, &disabled)
            } else {
                (&disabled, &enabled)
            };
            assert_eq!(
                left,
                Some(super::KeptAddonCopy {
                    deleted: gone.file_name().unwrap().to_str().unwrap().to_owned(),
                    kept: kept.file_name().unwrap().to_str().unwrap().to_owned(),
                })
            );
            assert!(!gone.exists());
            assert_eq!(std::fs::read(kept).unwrap(), kept_bytes);
            let current = mfcdb::get_mod_file_cache_by_instance_kind_filename(
                &app.db, *id, "mods", "twin.jar",
            )
            .await
            .unwrap()
            .expect("the row stays for the copy that is left");
            assert_eq!(current.id, row.id);
            assert_eq!(current.enabled, !shows_enabled);
            assert_eq!(current.filesize, kept_bytes.len() as i32);
            assert_ne!(current.metadata_id, row.metadata_id);
            let metadata = metarepo::get_metadata_by_id(&app.db, &current.metadata_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                metadata.sha512,
                <sha2::Sha512 as sha2::Digest>::digest(kept_bytes).to_vec()
            );
            assert_eq!(
                app.instance_manager()
                    .delete_mod(id, current.id.clone())
                    .await
                    .unwrap(),
                None
            );
            assert!(!enabled.exists() && !disabled.exists());
            assert!(
                mfcdb::get_mod_file_cache_by_instance_kind_filename(
                    &app.db, *id, "mods", "twin.jar",
                )
                .await
                .unwrap()
                .is_none()
            );
        }
    }

    #[tokio::test]
    async fn task05_delete_late_survivor_keeps_cache_row_and_reports_error() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) =
            addon_fixture(&app, "survivor", AddonType::Mods, "mod.jar", b"selected").await;
        let pause = app.instance_manager().test_hooks.arm("addon_removed", id);
        let actor = Arc::clone(&app);
        let row_id = row.id.clone();
        let deleting =
            tokio::spawn(async move { actor.instance_manager().delete_mod(id, row_id).await });
        pause.entered.notified().await;
        let survivor = path.with_file_name("mod.jar.disabled");
        std::fs::write(&survivor, b"later content").unwrap();
        pause.release.notify_one();
        assert!(deleting.await.unwrap().is_err());
        assert_eq!(std::fs::read(survivor).unwrap(), b"later content");
        assert!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                .await
                .unwrap()
                .is_some()
        );
    }

    fn assert_foreign_addon_error(error: anyhow::Error) {
        assert!(
            error.downcast_ref::<InvalidInstanceModIdError>().is_some(),
            "must reject the instance/addon pair before any addon work, got: {error}"
        );
    }

    #[tokio::test]
    async fn foreign_addon_delete_preserves_both_worlds_and_cache_rows() {
        let app = crate::setup_managers_for_test().await;
        let (a, row_a, path_a) =
            addon_fixture(&app, "A", AddonType::Worlds, "MyWorld", b"world A").await;
        let (b, row_b, path_b) =
            addon_fixture(&app, "B", AddonType::Worlds, "MyWorld", b"world B").await;

        assert_foreign_addon_error(
            app.instance_manager()
                .delete_mod(b, row_a.id.clone())
                .await
                .expect_err("a foreign addon ID must not delete a same-named world"),
        );
        assert_eq!(
            tokio::fs::read(path_a.join("level.dat")).await.unwrap(),
            b"world A"
        );
        assert_eq!(
            tokio::fs::read(path_b.join("level.dat")).await.unwrap(),
            b"world B"
        );
        for row in [&row_a, &row_b] {
            assert_eq!(
                mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                    .await
                    .unwrap(),
                Some(row.clone())
            );
        }

        app.instance_manager()
            .delete_mod(a, row_a.id.clone())
            .await
            .unwrap();
        assert!(!path_a.exists());
        assert!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row_a.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            tokio::fs::read(path_b.join("level.dat")).await.unwrap(),
            b"world B"
        );
        assert_eq!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row_b.id)
                .await
                .unwrap(),
            Some(row_b)
        );
    }

    #[tokio::test]
    async fn foreign_addon_toggle_preserves_both_files_and_cache_rows() {
        let app = crate::setup_managers_for_test().await;
        let (a, row_a, path_a) =
            addon_fixture(&app, "A", AddonType::Mods, "mod.jar", b"mod A").await;
        let (b, row_b, path_b) =
            addon_fixture(&app, "B", AddonType::Mods, "mod.jar", b"mod B").await;

        assert_foreign_addon_error(
            app.instance_manager()
                .enable_mod(b, row_a.id.clone(), false)
                .await
                .expect_err("a foreign addon ID must not toggle a same-named file"),
        );
        assert_eq!(tokio::fs::read(&path_a).await.unwrap(), b"mod A");
        assert_eq!(tokio::fs::read(&path_b).await.unwrap(), b"mod B");
        assert!(!path_a.with_file_name("mod.jar.disabled").exists());
        assert!(!path_b.with_file_name("mod.jar.disabled").exists());
        for row in [&row_a, &row_b] {
            assert_eq!(
                mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                    .await
                    .unwrap(),
                Some(row.clone())
            );
        }

        app.instance_manager()
            .enable_mod(a, row_a.id.clone(), false)
            .await
            .unwrap();
        assert!(!path_a.exists());
        assert_eq!(
            tokio::fs::read(path_a.with_file_name("mod.jar.disabled"))
                .await
                .unwrap(),
            b"mod A"
        );
        assert!(
            !mfcdb::get_mod_file_cache_by_id(&app.db, &row_a.id)
                .await
                .unwrap()
                .unwrap()
                .enabled
        );
        app.instance_manager()
            .enable_mod(a, row_a.id.clone(), true)
            .await
            .unwrap();
        assert_eq!(tokio::fs::read(&path_a).await.unwrap(), b"mod A");
        assert!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row_a.id)
                .await
                .unwrap()
                .unwrap()
                .enabled
        );
        assert_eq!(tokio::fs::read(&path_b).await.unwrap(), b"mod B");
        assert_eq!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row_b.id)
                .await
                .unwrap(),
            Some(row_b)
        );
    }

    #[tokio::test]
    async fn foreign_addon_update_and_icon_reject_the_pair_before_platform_work() {
        let app = crate::setup_managers_for_test().await;
        let (a, row, path) = addon_fixture(&app, "A", AddonType::Mods, "mod.jar", b"mod A").await;
        let (b, _, _) = addon_fixture(&app, "B", AddonType::Mods, "mod.jar", b"mod B").await;
        let manager = app.instance_manager();
        let results = [
            manager.find_mod_update(b, row.id.clone()).await.map(|_| ()),
            manager.update_mod(b, row.id.clone()).await.map(|_| ()),
            manager
                .update_curseforge_mod(b, row.id.clone())
                .await
                .map(|_| ()),
            manager
                .update_modrinth_mod(b, row.id.clone())
                .await
                .map(|_| ()),
            manager.get_mod_icon(b, row.id.clone(), 0).await.map(|_| ()),
        ];
        for result in results {
            assert_foreign_addon_error(result.expect_err("a foreign addon ID must be rejected"));
        }
        assert!(
            manager
                .find_mod_update(a, row.id.clone())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            manager.get_mod_icon(a, row.id.clone(), 0).await.unwrap(),
            None
        );
        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"mod A");
        assert_eq!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                .await
                .unwrap(),
            Some(row)
        );
    }

    struct PreflightOnlyInstaller;

    #[async_trait::async_trait]
    impl ResourceInstaller for PreflightOnlyInstaller {
        fn id(&self) -> String {
            panic!("must stop at installed-state check")
        }

        async fn downloadable(&self, _: &InstancePath) -> Option<Downloadable> {
            panic!("must stop before download")
        }

        fn get_install_path(&self, _: &InstancePath) -> PathBuf {
            panic!("must stop before publication")
        }

        async fn post_process(&self, _: &InstancePath, _: &std::path::Path) -> anyhow::Result<()> {
            panic!("must stop before post-processing")
        }

        fn dependencies(
            &self,
            _: &Arc<AppInner>,
            _: InstanceId,
            _: &InstanceData,
            _: ModChannel,
        ) -> DependencyIterator {
            panic!("must stop before dependencies")
        }

        async fn is_already_installed(
            &self,
            _: &Arc<AppInner>,
            _: InstanceId,
        ) -> anyhow::Result<bool> {
            anyhow::bail!("test installer stopped at preflight")
        }

        fn display_name(&self) -> String {
            panic!("must stop before task creation")
        }

        async fn rollback(&self, _: &mut InstanceData) -> anyhow::Result<()> {
            panic!("must stop before rollback")
        }
    }

    #[tokio::test]
    async fn foreign_addon_replacement_is_rejected_before_installation() {
        let app = crate::setup_managers_for_test().await;
        let (a, row, path) = addon_fixture(&app, "A", AddonType::Mods, "mod.jar", b"mod A").await;
        let (b, row_b, path_b) =
            addon_fixture(&app, "B", AddonType::Mods, "mod.jar", b"mod B").await;
        let installer = Installer::new(Box::new(PreflightOnlyInstaller));
        assert_foreign_addon_error(
            installer
                .install(&app, b, false, Some(row.id.clone()))
                .await
                .expect_err("a foreign replacement must fail before preparing an install"),
        );

        let own_error = installer
            .install(&app, a, false, Some(row.id.clone()))
            .await
            .expect_err("the controlled installer stops before download");
        assert_eq!(own_error.to_string(), "test installer stopped at preflight");
        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"mod A");
        assert_eq!(tokio::fs::read(&path_b).await.unwrap(), b"mod B");
        for row in [&row, &row_b] {
            assert_eq!(
                mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                    .await
                    .unwrap(),
                Some(row.clone())
            );
        }
    }

    #[test]
    fn rejects_toggling_a_world() {
        let err = ensure_toggleable(AddonType::Worlds)
            .expect_err("a world must not be enableable/disableable");

        assert!(
            err.to_string().contains("worlds cannot be enabled"),
            "the rejection must say why, got: {err}"
        );
    }

    // Pins the guard at its call site rather than only as a standalone predicate:
    // deleting the `ensure_toggleable` call in `enable_mod` must make this fail.
    // Without it, enabling/disabling a `worlds` row renames `saves/MyWorld` to
    // `saves/MyWorld.disabled`, which the scanner then re-reports as a new,
    // enabled world with no control to undo it.
    #[tokio::test]
    async fn enable_mod_rejects_a_world() {
        let app = crate::setup_managers_for_test().await;

        let group = app.instance_manager().get_default_group().await.unwrap();
        let instance_id = app
            .instance_manager()
            .create_instance(
                group,
                String::from("test"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.20.1"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await
            .unwrap();

        let now = DbDateTime(Utc::now().fixed_offset());
        metarepo::insert_metadata(
            &app.db,
            "world-metadata",
            0,
            b"sha512",
            b"sha1",
            "vanilla",
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await
        .unwrap();

        mfcdb::upsert_mod_file_cache(
            &app.db,
            *instance_id,
            String::from("MyWorld"),
            0,
            true,
            AddonType::Worlds.to_db_string().to_string(),
            String::from("world-metadata"),
            now,
        )
        .await
        .unwrap();

        let row = mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *instance_id, "MyWorld")
            .await
            .unwrap()
            .expect("the seeded ModFileCache row for the world must be found");

        let err = app
            .instance_manager()
            .enable_mod(instance_id, row.id, false)
            .await
            .expect_err("enable_mod must reject toggling a world");

        assert!(
            err.to_string().contains("worlds cannot be enabled"),
            "must fail via the toggle guard, got: {err}"
        );
    }

    #[test]
    fn allows_toggling_every_file_backed_addon() {
        for addon_type in AddonType::all()
            .into_iter()
            .filter(|t| *t != AddonType::Worlds)
        {
            ensure_toggleable(addon_type)
                .unwrap_or_else(|e| panic!("{addon_type:?} must stay toggleable, got: {e}"));
        }
    }

    #[tokio::test]
    async fn removes_a_world_directory() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("MyWorld");
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), b"x").unwrap();

        let removed = remove_addon_from_disk(&world, &dir.path().join("MyWorld.disabled"), true)
            .await
            .unwrap();

        assert!(removed, "a world directory must be reported as removed");
        assert!(
            !world.exists(),
            "the world directory must be gone from disk"
        );
    }

    #[tokio::test]
    async fn removes_an_enabled_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pack.zip");
        std::fs::write(&file, b"x").unwrap();

        let removed = remove_addon_from_disk(&file, &dir.path().join("pack.zip.disabled"), false)
            .await
            .unwrap();

        assert!(removed);
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn removes_a_disabled_file() {
        let dir = tempfile::tempdir().unwrap();
        let disabled = dir.path().join("pack.zip.disabled");
        std::fs::write(&disabled, b"x").unwrap();

        let removed = remove_addon_from_disk(&dir.path().join("pack.zip"), &disabled, false)
            .await
            .unwrap();

        assert!(removed);
        assert!(!disabled.exists());
    }

    #[tokio::test]
    async fn reports_nothing_removed_when_absent() {
        let dir = tempfile::tempdir().unwrap();

        let removed = remove_addon_from_disk(
            &dir.path().join("gone.zip"),
            &dir.path().join("gone.zip.disabled"),
            false,
        )
        .await
        .unwrap();

        assert!(!removed, "an already-absent addon reports nothing removed");
    }

    // Guards the flag itself: a non-world addon whose path happens to be a
    // directory must not be blown away with remove_dir_all.
    #[tokio::test]
    async fn leaves_a_directory_alone_for_file_addons() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("weird");
        std::fs::create_dir(&path).unwrap();

        let removed = remove_addon_from_disk(&path, &dir.path().join("weird.disabled"), false)
            .await
            .unwrap();

        assert!(!removed);
        assert!(
            path.exists(),
            "a directory must survive a file-addon delete"
        );
    }
}

/// `find_mod_update`'s platform half against a stand-in API.
#[cfg(test)]
mod update_resolution_tests {
    use super::*;
    use crate::domain::instance::info::{ModLoader, StandardVersion};
    use crate::managers::modplatforms::{curseforge::CurseForge, modrinth::Modrinth};
    use mockito::Matcher;
    use serde_json::json;

    struct Api {
        server: mockito::ServerGuard,
        curseforge: CurseForge,
        modrinth: Modrinth,
    }

    async fn api() -> Api {
        let server = mockito::Server::new_async().await;
        let base = format!("{}/", server.url());
        // No retry middleware: a 500 must surface at once.
        let client = || reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build();
        Api {
            curseforge: CurseForge::with_base_url(client(), &base),
            modrinth: Modrinth::with_base_url(client(), &base),
            server,
        }
    }

    impl Api {
        async fn resolve(
            &self,
            row: &mfcdb::ModUpdateIdsRow,
        ) -> anyhow::Result<Option<RemoteVersion>> {
            let version = StandardVersion {
                release: "1.20.1".into(),
                modloaders: [ModLoader {
                    type_: ModLoaderType::Fabric,
                    version: "fixture".into(),
                }]
                .into_iter()
                .collect(),
            };
            let context = UpdateContext {
                game: "1.20.1".into(),
                loaders: vec!["fabric".into()],
                shader_engines: vec![],
            };
            let sources = ModSources {
                channels: ModChannelWithUsage::str_to_vec("stable:true,beta:true,alpha:true")?,
                platform_blacklist: vec![],
            };
            resolve_mod_update(
                &self.curseforge,
                &self.modrinth,
                row,
                &version,
                "mods",
                &context,
                &sources,
            )
            .await
        }

        async fn get(
            &mut self,
            path: &str,
            status: usize,
            body: serde_json::Value,
        ) -> mockito::Mock {
            self.server
                .mock("GET", path)
                .match_query(Matcher::Any)
                .with_status(status)
                .with_body(body.to_string())
                .create_async()
                .await
        }
    }

    const CF_PROJECT: i32 = 4242;
    const CF_INSTALLED: i32 = 100;

    /// `release_type`: 1 release, 2 beta, 3 alpha.
    fn cf_file(id: i32, day: u32, release_type: u8) -> serde_json::Value {
        json!({
            "id": id, "gameId": 432, "modId": CF_PROJECT, "isAvailable": true,
            "displayName": format!("file-{id}"), "fileName": format!("file-{id}.jar"),
            "releaseType": release_type, "fileStatus": 4, "hashes": [],
            "fileDate": format!("2026-01-{day:02}T00:00:00Z"), "fileLength": 1,
            "downloadCount": 0, "downloadUrl": null, "gameVersions": ["1.20.1", "Fabric"],
            "sortableGameVersions": [], "dependencies": [], "fileFingerprint": 0
        })
    }

    fn mr_version(id: &str, day: u32, version_type: &str) -> serde_json::Value {
        json!({
            "id": id, "project_id": "project", "name": id, "version_number": id,
            "dependencies": [], "game_versions": ["1.20.1"], "version_type": version_type,
            "loaders": ["fabric"], "featured": false, "author_id": "author",
            "date_published": format!("2026-01-{day:02}T00:00:00Z"), "downloads": 0, "files": []
        })
    }

    fn cf_row() -> mfcdb::ModUpdateIdsRow {
        mfcdb::ModUpdateIdsRow {
            cf_project_id: Some(CF_PROJECT),
            cf_file_id: Some(CF_INSTALLED),
            cf_release_type: Some(ModChannel::Stable as i32),
            mr_project_id: None,
            mr_version_id: None,
            mr_release_type: None,
        }
    }

    fn mr_row() -> mfcdb::ModUpdateIdsRow {
        mfcdb::ModUpdateIdsRow {
            cf_project_id: None,
            cf_file_id: None,
            cf_release_type: None,
            mr_project_id: Some("project".into()),
            mr_version_id: Some("installed".into()),
            mr_release_type: Some(ModChannel::Stable as i32),
        }
    }

    fn chosen(update: Option<RemoteVersion>) -> Option<String> {
        update.map(|update| match update {
            RemoteVersion::Curseforge(file) => file.id.to_string(),
            RemoteVersion::Modrinth(version) => version.id,
        })
    }

    /// A jar both platforms list, with each platform's cached channel.
    fn dual_row(cf: ModChannel, mr: ModChannel) -> mfcdb::ModUpdateIdsRow {
        mfcdb::ModUpdateIdsRow {
            cf_release_type: Some(cf as i32),
            mr_project_id: Some("project".into()),
            mr_version_id: Some("installed".into()),
            mr_release_type: Some(mr as i32),
            ..cf_row()
        }
    }

    /// A stand-in API whose CurseForge project lists `cf_files` and whose
    /// Modrinth project lists `mr_versions`. The installed file (id
    /// `CF_INSTALLED`) and version (id "installed") are served from those
    /// lists, and answer 404 when the lists do not hold them.
    async fn serving(cf_files: Vec<serde_json::Value>, mr_versions: Vec<serde_json::Value>) -> Api {
        let mut api = api().await;
        let installed_file = cf_files
            .iter()
            .find(|file| file["id"] == CF_INSTALLED)
            .cloned();
        let installed_version = mr_versions
            .iter()
            .find(|version| version["id"] == "installed")
            .cloned();
        api.get(
            &format!("/mods/{CF_PROJECT}/files"),
            200,
            json!({ "data": cf_files }),
        )
        .await;
        let (status, body) = match installed_file {
            Some(file) => (200, json!({ "data": file })),
            None => (404, json!({ "error": "not found" })),
        };
        api.get(
            &format!("/mods/{CF_PROJECT}/files/{CF_INSTALLED}"),
            status,
            body,
        )
        .await;
        api.get("/project/project/version", 200, json!(mr_versions))
            .await;
        let (status, body) = match installed_version {
            Some(version) => (200, version),
            None => (404, json!({ "error": "not_found" })),
        };
        api.get("/version/installed", status, body).await;
        api
    }

    fn cached(row: mfcdb::ModUpdateIdsRow, channel: ModChannel) -> mfcdb::ModUpdateIdsRow {
        mfcdb::ModUpdateIdsRow {
            cf_release_type: row.cf_release_type.map(|_| channel as i32),
            mr_release_type: row.mr_release_type.map(|_| channel as i32),
            ..row
        }
    }

    #[tokio::test]
    async fn a_release_install_is_not_moved_to_a_newer_beta() {
        let mut api = api().await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files"),
            200,
            json!({ "data": [cf_file(CF_INSTALLED, 10, 1), cf_file(200, 20, 2), cf_file(300, 25, 3)] }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/{CF_INSTALLED}"),
            200,
            json!({ "data": cf_file(CF_INSTALLED, 10, 1) }),
        )
        .await;
        api.get(
            "/project/project/version",
            200,
            json!([
                mr_version("installed", 10, "release"),
                mr_version("beta", 20, "beta")
            ]),
        )
        .await;
        api.get(
            "/version/installed",
            200,
            mr_version("installed", 10, "release"),
        )
        .await;

        assert_eq!(chosen(api.resolve(&cf_row()).await.unwrap()), None);
        assert_eq!(chosen(api.resolve(&mr_row()).await.unwrap()), None);
    }

    #[tokio::test]
    async fn a_removed_installed_file_is_replaced_by_a_compatible_release() {
        let mut api = api().await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files"),
            200,
            json!({ "data": [cf_file(150, 5, 1), cf_file(200, 20, 2)] }),
        )
        .await;
        let removed = api
            .get(
                &format!("/mods/{CF_PROJECT}/files/{CF_INSTALLED}"),
                404,
                json!({ "error": "not found" }),
            )
            .await;
        api.get(
            "/project/project/version",
            200,
            json!([
                mr_version("stable", 5, "release"),
                mr_version("beta", 20, "beta")
            ]),
        )
        .await;
        api.get("/version/installed", 404, json!({ "error": "not_found" }))
            .await;

        assert_eq!(
            chosen(api.resolve(&cf_row()).await.unwrap()).as_deref(),
            Some("150"),
            "the Release uploaded after the removed file replaces it"
        );
        removed.assert_async().await;
        assert_eq!(
            chosen(api.resolve(&mr_row()).await.unwrap()).as_deref(),
            Some("stable")
        );
    }

    #[tokio::test]
    async fn any_other_failure_to_read_the_installed_file_still_fails() {
        let mut api = api().await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files"),
            200,
            json!({ "data": [cf_file(150, 5, 1)] }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/{CF_INSTALLED}"),
            500,
            json!({ "error": "internal" }),
        )
        .await;

        let Err(error) = api.resolve(&cf_row()).await else {
            panic!("a 500 on the installed file must fail the update");
        };
        assert!(!is_not_found(&error), "{error:?}");
        assert!(format!("{error:#}").contains("500"), "{error:#}");
    }

    /// The badge reads the cached channel, so the resolver must too, even
    /// when the platform has relabelled the file since.
    #[tokio::test]
    async fn the_cached_channel_sets_the_floor_not_the_live_one() {
        // Cached as a beta, live as a Release.
        let api = serving(
            vec![cf_file(CF_INSTALLED, 10, 1), cf_file(200, 20, 2)],
            vec![
                mr_version("installed", 10, "release"),
                mr_version("beta", 20, "beta"),
            ],
        )
        .await;
        assert_eq!(
            chosen(
                api.resolve(&cached(cf_row(), ModChannel::Beta))
                    .await
                    .unwrap()
            )
            .as_deref(),
            Some("200")
        );
        assert_eq!(
            chosen(
                api.resolve(&cached(mr_row(), ModChannel::Beta))
                    .await
                    .unwrap()
            )
            .as_deref(),
            Some("beta")
        );
        assert_eq!(
            chosen(api.resolve(&cf_row()).await.unwrap()),
            None,
            "cached as a Release, the same file has no update"
        );
        assert_eq!(chosen(api.resolve(&mr_row()).await.unwrap()), None);
    }

    #[tokio::test]
    async fn a_removed_beta_moves_to_the_newer_beta_not_an_older_release() {
        // CurseForge numbers files in upload order: the older Release has a
        // lower id than the removed file, the newer beta a higher one.
        let api = serving(
            vec![cf_file(50, 5, 1), cf_file(250, 25, 2)],
            vec![
                mr_version("release", 5, "release"),
                mr_version("beta", 25, "beta"),
            ],
        )
        .await;
        assert_eq!(
            chosen(
                api.resolve(&cached(cf_row(), ModChannel::Beta))
                    .await
                    .unwrap()
            )
            .as_deref(),
            Some("250")
        );
        assert_eq!(
            chosen(
                api.resolve(&cached(mr_row(), ModChannel::Beta))
                    .await
                    .unwrap()
            )
            .as_deref(),
            Some("beta")
        );
    }

    #[tokio::test]
    async fn a_jar_both_platforms_list_is_held_to_its_most_stable_label() {
        // CurseForge and Modrinth label the installed jar differently, and
        // each has a newer beta.
        for (cf, mr) in [
            (ModChannel::Stable, ModChannel::Beta),
            (ModChannel::Beta, ModChannel::Stable),
        ] {
            let api = serving(
                vec![
                    cf_file(
                        CF_INSTALLED,
                        10,
                        if cf == ModChannel::Stable { 1 } else { 2 },
                    ),
                    cf_file(200, 20, 2),
                ],
                vec![
                    mr_version(
                        "installed",
                        10,
                        if mr == ModChannel::Stable {
                            "release"
                        } else {
                            "beta"
                        },
                    ),
                    mr_version("newer-beta", 21, "beta"),
                ],
            )
            .await;
            assert_eq!(
                chosen(api.resolve(&dual_row(cf, mr)).await.unwrap()),
                None,
                "CurseForge {cf:?}, Modrinth {mr:?}"
            );
            assert_eq!(
                chosen(
                    api.resolve(&dual_row(ModChannel::Beta, ModChannel::Beta))
                        .await
                        .unwrap()
                )
                .as_deref(),
                Some("newer-beta"),
                "a beta on both platforms still moves to the newest beta"
            );
        }
    }

    #[tokio::test]
    async fn a_file_removed_from_one_platform_follows_its_listing_on_the_other() {
        // CurseForge removed the installed beta; Modrinth still lists the
        // same jar, published on day 20. File 50 was uploaded before it.
        let api = serving(
            vec![cf_file(50, 5, 1), cf_file(250, 25, 2)],
            vec![mr_version("installed", 20, "beta")],
        )
        .await;
        assert_eq!(
            chosen(
                api.resolve(&dual_row(ModChannel::Beta, ModChannel::Beta))
                    .await
                    .unwrap()
            )
            .as_deref(),
            Some("250")
        );
    }
}

pub(super) fn same_addon_game_version(
    a: &Option<crate::domain::instance::info::GameVersion>,
    b: &Option<crate::domain::instance::info::GameVersion>,
) -> bool {
    use crate::domain::instance::info::GameVersion;
    match (a, b) {
        (Some(GameVersion::Standard(a)), Some(GameVersion::Standard(b))) => {
            a.release == b.release && a.modloaders == b.modloaders
        }
        (Some(GameVersion::Custom(a)), Some(GameVersion::Custom(b))) => a == b,
        (None, None) => true,
        _ => false,
    }
}

pub(super) fn ensure_pack_unlocked(
    pack: Option<&crate::domain::instance::info::ModpackInfo>,
) -> anyhow::Result<()> {
    if pack.is_some_and(|pack| pack.locked) {
        bail!("Modpack is locked");
    }
    Ok(())
}

/// Whether addon edits may go ahead beside this instance's pending setup: one
/// that never stages or applies pack content (see
/// [`Session::is_content_free`]) and still targets the configured game version
/// and pack. A version-only install keeps allowing them after it fails or is
/// cancelled, in any phase, as 2.0.40 did. So does a completed import that
/// keeps its platform link: its pack request is never downloaded or applied,
/// Play only checks that it still matches the record. The same holds for such
/// a record that is not this instance's own (copied by a duplicate, written
/// before the instance had an id, or kept from an earlier registration of the
/// folder) that Play continues as this instance's own. The lock is checked
/// before this, so only the pack's identity has to match, and a user who
/// unpaired the pack (no configured pack) may edit too, as after Unlock: Play
/// neither applies the link nor attaches it again. Read-only.
///
/// [`Session::is_content_free`]: super::modpack::session::Session::is_content_free
async fn setup_allows_addon_edits(
    root: &std::path::Path,
    instance_id: InstanceId,
    config: &info::Instance,
    session: &super::modpack::session::Session,
) -> anyhow::Result<bool> {
    if !session.is_content_free() {
        return Ok(false);
    }
    if !session.owned_by(instance_id)
        && session.copy_qualification(root, instance_id).await? != Some(true)
    {
        return Ok(false);
    }
    let installed = super::schema::parse_instance_config(&session.installed_config)?;
    if !same_addon_game_version(
        &installed.game_configuration.version,
        &config.game_configuration.version,
    ) || config.modpack.as_ref().is_some_and(|pack| {
        installed.modpack.as_ref().map(|pack| &pack.modpack) != Some(&pack.modpack)
    }) {
        return Ok(false);
    }
    let setup = root.join(".setup");
    // A pack request that is not the record's own is one Play refuses.
    let request = setup.join("change-pack-version.json");
    let on_disk = match addon_metadata(&request).await? {
        None => None,
        Some(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            match tokio::fs::read_to_string(&request).await {
                Ok(text) => Some(text),
                Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                    return Ok(false);
                }
                Err(error) => return Err(error.into()),
            }
        }
        Some(_) => return Ok(false),
    };
    if on_disk != session.requested_pack {
        return Ok(false);
    }
    for marker in [
        "repair",
        "apply-started",
        "representation-transitions",
        "staging-packinfo.json",
    ] {
        if addon_metadata(&setup.join(marker)).await?.is_some() {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Why addons wait while Play (or a Repair or update) is installing a setup
/// that writes them, such as a modpack's files: the way out is to wait.
pub(super) const PREPARING_SETUP_REFUSAL: &str = "This instance is still installing or updating its files. Wait for it to finish, then change its addons.";

/// What waits for a pending setup, for [`pending_setup_refusal`] and
/// [`unreadable_setup_refusal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PendingChange {
    Addons,
    GameVersion,
    /// Play itself, which finishes the pending setup.
    Play,
}

impl PendingChange {
    fn before(self) -> &'static str {
        match self {
            Self::Addons => "changing addons",
            Self::GameVersion => "changing the game version or loader",
            Self::Play => "playing",
        }
    }
    fn not_safe(self) -> &'static str {
        match self {
            Self::Addons => "addons can't be changed safely",
            Self::GameVersion => "the game version or loader can't be changed safely",
            Self::Play => "it can't be finished safely",
        }
    }
}

/// Why `change` waits for a pending setup whose record can't be read (an
/// unknown or damaged `setup-session.json`, or a `.setup` that is not a plain
/// folder), naming the way out: Repair for a modpack, otherwise removing the
/// hidden folder, as for the 2.0.40 leftovers Play refuses. The technical
/// cause stays in the error chain and the logs.
pub(super) fn unreadable_setup_refusal(config: &info::Instance, change: PendingChange) -> String {
    let unreadable = format!(
        "This instance has an unfinished install or update that can't be checked, so {}.",
        change.not_safe()
    );
    if config.modpack.is_some() {
        format!(
            "{unreadable} Use Repair on this instance to reinstall the modpack; your worlds are kept."
        )
    } else if change == PendingChange::GameVersion {
        format!(
            "{unreadable} To change it anyway, delete the hidden .setup folder inside the instance folder, then choose the version again."
        )
    } else {
        format!(
            "{unreadable} To install it again, delete the hidden .setup folder inside the instance folder, then press Play."
        )
    }
}

/// Why `change` waits for a pending setup that is not this instance's own
/// (see [`Session::binds_to`]) and that Play can't continue: it may have
/// changed files before the instance folder was copied, moved or registered
/// again, and there is no original archive to rebuild it from. `copied`: a
/// duplicate made in the launcher copied it, so the original may still finish
/// it. Names the way out the instance itself has: Repair for a modpack,
/// otherwise removing the hidden folder, as for the setups Play refuses to
/// resume.
///
/// [`Session::binds_to`]: super::modpack::session::Session::binds_to
pub(super) fn uncontinued_setup_refusal(
    config: &info::Instance,
    change: PendingChange,
    copied: bool,
) -> String {
    let refused = if copied {
        format!(
            "This instance was copied while its setup was part-way through, so {} on the copy.",
            change.not_safe()
        )
    } else {
        format!(
            "This instance's unfinished install or update began before the instance folder was copied or moved, or before the launcher's database was reset, so {}.",
            change.not_safe()
        )
    };
    let way_out = if config.modpack.is_some() {
        "Use Repair on this instance to reinstall the modpack; your worlds are kept."
    } else if change == PendingChange::GameVersion {
        "To change it anyway, delete the hidden .setup folder inside the instance folder, then choose the version again."
    } else {
        "To install it again, delete the hidden .setup folder inside the instance folder, then press Play."
    };
    if copied {
        format!(
            "{refused} {way_out} You can also finish the setup on the original instance, then copy it again."
        )
    } else {
        format!("{refused} {way_out}")
    }
}

/// Why `change` waits for the pending setup in `root`, naming the way out
/// that Play, Edit and the Repair dialog actually offer for it. `session` is its
/// record, if it has one. Read-only: not every caller holds the gate.
pub(super) async fn pending_setup_refusal(
    root: &std::path::Path,
    instance_id: InstanceId,
    config: &info::Instance,
    session: Option<&super::modpack::session::Session>,
    change: PendingChange,
) -> anyhow::Result<String> {
    use super::modpack::session::{self, Kind};
    let unfinished = format!(
        "This instance has an unfinished install or update. Press Play to finish it before {}.",
        change.before()
    );
    let cancellable = format!(
        "This instance has an unfinished modpack update or repair. Press Play to finish it, or cancel it from Repair, before {}.",
        change.before()
    );
    // A setup that only waits for its game version install, which a version
    // or loader change replaces. Not advertised for a modpack, whose files
    // target its own version.
    let replaceable = format!(
        "This instance's game version or loader hasn't finished installing. Press Play to finish installing it, or choose another version with Edit, before {}.",
        change.before()
    );
    let advertise_edit = change == PendingChange::Addons && config.modpack.is_none();
    // Repair, and so its Cancel, is only offered for an instance with a
    // modpack: not after the user unpaired it.
    let repair_offered = config.modpack.is_some();
    let setup = root.join(".setup");
    let Some(session) = session else {
        let legacy = session::classify_legacy(root).await;
        // The Repair dialog adopts an untouched update of an installed pack
        // and offers Cancel for it, as it does once adopted.
        if repair_offered
            && matches!(&legacy, Ok(Some(request)) if request.over_installed_pack(config))
        {
            return Ok(cancellable);
        }
        if advertise_edit && session::legacy_pack_applied(root).await.unwrap_or(false) {
            return Ok(replaceable);
        }
        if matches!(legacy, Ok(Some(_))) || session::legacy_resumed_by_play(root).unwrap_or(false) {
            return Ok(unfinished);
        }
        // The leftovers Play refuses to resume, in the same words.
        return Ok(if config.modpack.is_some() {
            format!(
                "An earlier launcher version was interrupted while applying this modpack's files, so {}. Use Repair on this instance to reinstall the modpack; your worlds are kept.",
                change.not_safe()
            )
        } else {
            format!(
                "An earlier launcher version was interrupted while applying this instance's files, so {}. To install it again, delete the hidden .setup folder inside the instance folder, then press Play.",
                change.not_safe()
            )
        });
    };
    // Play and the Repair dialog both continue a record that is not this
    // instance's own (a copied one, or one kept from an earlier registration
    // of the folder) when it qualifies as this instance's own, and the dialog
    // offers Cancel for such a pack request.
    let owned = session.owned_by(instance_id);
    let continued_copy = !owned
        && matches!(
            session.copy_qualification(root, instance_id).await,
            Ok(Some(true))
        );
    // The dialog withholds Cancel once apply may have started.
    let apply_marked = addon_metadata(&setup.join("apply-started"))
        .await?
        .is_some()
        || addon_metadata(&setup.join("representation-transitions"))
            .await?
            .is_some();
    let edit_replaces = advertise_edit
        && (session
            .supersedable(root, instance_id)
            .await
            .unwrap_or(false)
            || session
                .supersedable_once_continued(root, instance_id)
                .await
                .unwrap_or(false)
            || session
                .copied_pack_installed(root, instance_id)
                .await
                .unwrap_or(false));
    Ok(
        if repair_offered
            && (owned || continued_copy && session.kind == Kind::Pack)
            && !apply_marked
            && session.cancel_reason(config).is_none()
        {
            cancellable
        } else if edit_replaces {
            replaceable
        } else if owned
            || continued_copy
            || session.binds_to(instance_id) && session::has_original_archive(&setup)
        {
            unfinished
        } else {
            uncontinued_setup_refusal(config, change, session.copied_to == Some(*instance_id))
        },
    )
}
