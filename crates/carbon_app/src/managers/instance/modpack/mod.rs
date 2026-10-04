use super::{InstanceData, InstanceManager, InstanceType};
use crate::{
    domain::{
        instance::{
            AddonType, InstanceId,
            info::{self, CurseforgeModpack, Modpack, ModpackInfo, ModrinthModpack},
        },
        vtask::VisualTaskId,
    },
    managers::{ManagerRef, instance::InvalidInstanceIdError},
    util::NormalizedWalkdir,
};
use anyhow::{Context, bail};
use carbon_platforms::{
    curseforge::{
        self,
        filters::{
            ModFilesParameters, ModFilesParametersQuery, ModParameters, ModsParameters,
            ModsParametersBody,
        },
    },
    modrinth::{project::ProjectVersionsFilters, search::ProjectID},
};
use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_rt_path::InstancePath;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

pub mod apply_plan;
pub mod disk_scan;
mod imported_baseline;
pub mod origin_check;
pub mod packinfo;
pub(crate) mod session;

/// The version changes each instance is checking: usually one, but a request
/// can wait for the gate behind another whose modal was closed.
pub(super) type VersionChangeChecks =
    dashmap::DashMap<InstanceId, Vec<std::sync::Arc<tokio_util::sync::CancellationToken>>>;

/// Makes a version change's check stoppable through
/// `cancel_version_change_check` while it lives.
struct VersionChangeCheck<'a> {
    checks: &'a VersionChangeChecks,
    instance_id: InstanceId,
    cancel: std::sync::Arc<tokio_util::sync::CancellationToken>,
}

impl<'a> VersionChangeCheck<'a> {
    fn begin(
        checks: &'a VersionChangeChecks,
        instance_id: InstanceId,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Self {
        let cancel = std::sync::Arc::new(cancel.clone());
        checks.entry(instance_id).or_default().push(cancel.clone());
        Self {
            checks,
            instance_id,
            cancel,
        }
    }
}

impl Drop for VersionChangeCheck<'_> {
    fn drop(&mut self) {
        self.checks.remove_if_mut(&self.instance_id, |_, running| {
            running.retain(|cancel| !std::sync::Arc::ptr_eq(cancel, &self.cancel));
            running.is_empty()
        });
    }
}

impl ManagerRef<'_, InstanceManager> {
    pub async fn check_curseforge_modpack_updates(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data()?;

        let Some(ModpackInfo {
            modpack: Modpack::Curseforge(modpack),
            ..
        }) = data.config.modpack.clone()
        else {
            bail!("Instance is not a curseforge modpack");
        };

        drop(instances);

        let response = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: modpack.project_id as i32,
                query: ModFilesParametersQuery {
                    game_version: None,
                    mod_loader_type: None,
                    game_version_type_id: None,
                    index: None,
                    page_size: None,
                },
            })
            .await?;

        let has_update = !response
            .data
            .first()
            .map(|file| file.id as u32 == modpack.file_id)
            .unwrap_or(false);

        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data_mut()?;
        data.modpack_update_curseforge = Some(has_update);

        Ok(())
    }

    pub async fn check_modrinth_modpack_updates(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data()?;

        let Some(ModpackInfo {
            modpack: Modpack::Modrinth(modpack),
            ..
        }) = data.config.modpack.clone()
        else {
            bail!("Instance is not a modrinth modpack");
        };

        drop(instances);

        let response = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(modpack.project_id),
                game_versions: Some(Vec::new()),
                loaders: Some(Vec::new()),
                offset: None,
                limit: None,
            })
            .await?;

        let has_update = response
            .0
            .first()
            .map(|v| v.id != modpack.version_id)
            .unwrap_or(false);

        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data_mut()?;
        data.modpack_update_modrinth = Some(has_update);

        Ok(())
    }

    /// Change the modpack (hence modpack version) of an instance.
    /// While this could also change between different modpack (and modplatforms), the usual use case is to change
    /// between modpack versions.
    pub async fn change_modpack(
        self,
        instance_id: InstanceId,
        modpack: Modpack,
    ) -> anyhow::Result<VisualTaskId> {
        let app = self.app.clone();
        tokio::spawn(async move {
            app.instance_manager()
                .change_modpack_owned(instance_id, modpack)
                .await
        })
        .await?
    }

    /// Stops every installed-version check a version change of this instance
    /// is running, if any: each change then ends with
    /// [`ChangeVersionRefusal::Cancelled`] and its setup never starts. Once
    /// the setup has started this does nothing; the task runs on as usual.
    pub fn cancel_version_change_check(self, instance_id: InstanceId) {
        if let Some(checks) = self.version_change_checks.get(&instance_id) {
            for check in checks.iter() {
                check.cancel();
            }
        }
    }

    async fn change_modpack_owned(
        self,
        instance_id: InstanceId,
        modpack: Modpack,
    ) -> anyhow::Result<VisualTaskId> {
        use super::run::LaunchState;

        let runtime_activity = self
            .app
            .runtime_lifecycle
            .admit(format!("instance {instance_id} setup producer"))?;
        let operation = self.lock_instance(instance_id).await;
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data()?;
        if data.config.modpack.is_none() {
            anyhow::bail!("Instance does not have an associated modpack");
        }

        match data.state {
            LaunchState::Inactive { .. } => {}
            _ => {
                anyhow::bail!(
                    "Cannot change the modpack version while the instance is launching, queued, running, or being deleted"
                );
            }
        }

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        let installed_pack = data.config.modpack.as_ref().unwrap().modpack.clone();
        drop(instances);

        let requests_installed = modpack == installed_pack;
        let pack_version_text = serde_json::to_string(&PackVersionFile::from(modpack))?;

        let setup_path = instance_path.get_root().join(".setup");

        if setup_path.exists() {
            anyhow::bail!(
                "Instance has not completed the setup phase, attempting to change the modpack may irreparably damage it."
            );
        }

        let platform = imported_baseline::platform(&installed_pack);
        let baseline = imported_baseline::ensure_cancellable(
            &instance_path.get_root(),
            platform,
            &cancel,
            imported_baseline::resolve(self.app, &installed_pack, &cancel),
        )
        .await?;
        #[cfg(test)]
        self.test_hooks.pause("change_checked", instance_id).await;
        // The check only looks between files, so a cancel can land after the
        // last look. Asked once the check can no longer be stopped, so any
        // cancel that reached it counts. A record it wrote stays for the next
        // attempt to read.
        drop(check);
        if cancel.is_cancelled() {
            return Err(ChangeVersionRefusal::Cancelled.into());
        }
        // Before a setup exists: one for this version could only fail, and it
        // would keep the instance from starting until it is cancelled.
        if requests_installed
            && matches!(
                baseline,
                imported_baseline::Baseline::Unavailable { gone: true }
            )
        {
            return Err(ChangeVersionRefusal::VersionGone.into());
        }

        self.run_prepare_with_setup_cleanup(
            instance_id,
            operation,
            runtime_activity,
            session::Request {
                producer: session::Producer::Change,
                kind: session::Kind::Pack,
                files: vec![(
                    "change-pack-version.json".into(),
                    pack_version_text.into_bytes(),
                )],
                options: None,
                reset: false,
                replaces: None,
                warnings: baseline.warning(platform).into_iter().collect(),
            },
        )
        .await
    }

    /// Repair the instance's current modpack. Equivalent to wiping the
    /// `.setup` folder by hand and relaunching, but without leaving the user
    /// to figure that out. Re-runs the modpack download/extract pipeline in
    /// repair mode: every pack-tracked path is reconciled against what is
    /// actually on disk right now (via the `.setup/repair` marker this
    /// writes), so a missing *or* damaged file is restored, not just a
    /// missing one.
    ///
    /// A pending setup, whatever it is (a failed install, update or repair,
    /// one a duplicate copied, one 2.0.40 left, or one whose record can't be
    /// read), is retired to `.setup-cancelled-<operation|legacy>-<uuid>` and
    /// replaced, as long as it is still the one `expected_pending` names (see
    /// [`PendingSetupStatus::discard_token`]): a dialog that went stale never
    /// discards a setup accepted after it opened. The repair installs the
    /// configured pack, or the pack an interrupted apply was installing (see
    /// [`session::Session::repair_target`]), from a fresh download with a
    /// deep check, planned against the root `packinfo.json`, which stays.
    ///
    /// Refuses to run if the instance is currently launching or running —
    /// wiping `.setup` mid-install would corrupt the in-flight task.
    pub async fn repair_modpack(
        self,
        instance_id: InstanceId,
        mut options: RepairMarkerFile,
    ) -> anyhow::Result<VisualTaskId> {
        use super::run::LaunchState;

        let runtime_activity = self
            .app
            .runtime_lifecycle
            .admit(format!("instance {instance_id} setup producer"))?;
        let operation = self.lock_instance(instance_id).await;
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data()?;
        let modpack = data
            .config
            .modpack
            .as_ref()
            .map(|m| m.modpack.clone())
            .ok_or_else(|| {
                anyhow::anyhow!("Instance does not have an associated modpack to repair")
            })?;

        match data.state {
            LaunchState::Inactive { .. } => {}
            _ => {
                anyhow::bail!(
                    "Cannot repair while the instance is launching, queued, running, or being deleted"
                );
            }
        }

        // Fail fast on a malformed cleanup path before anything on disk is
        // touched — a bad path here must surface to the user immediately,
        // not after `.setup` has already been wiped and rewritten (see the
        // apply-time cleanup pass in `run/modpack.rs::apply_user_cleanup`,
        // which does the rest of the validation — tracked-path checks need
        // packinfo, not available yet here — and never bails once staging
        // has actually started).
        for path in &options.cleanup_paths {
            if normalize_cleanup_path(path).is_none() {
                anyhow::bail!("repair cleanup path {path:?} is not a valid packinfo-style path");
            }
        }

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        drop(instances);

        #[cfg(test)]
        self.test_hooks
            .pause("repair_prechecked", instance_id)
            .await;

        // The setup this repair replaces must still be the one the user chose
        // to discard: never one accepted after the dialog opened.
        let (status, pending) = self.inspect_pending_setup(instance_id).await?;
        if status.discard_token != expected_pending {
            bail!(
                "This instance's unfinished install or update changed after Repair was opened, so nothing was changed. Open Repair again to see what it will do."
            );
        }
        let target = pending
            .as_ref()
            .and_then(|pending| pending.target.clone())
            .unwrap_or_else(|| modpack.clone());
        // Continuing an interrupted change finishes it, so this task says what
        // that change would have said.
        let warnings = pending
            .as_ref()
            .filter(|pending| pending.interrupted && pending.target.is_some())
            .map(|pending| pending.warnings.clone())
            .unwrap_or_default();
        let replaces = pending.map(|pending| session::Replaced {
            label: pending.label,
            interrupted: pending.interrupted,
            keep_payload: pending.keep_payload,
        });
        // Cleanup selections come from a preview of the installed record,
        // which is withheld when that record may not describe the files.
        if status.preview_withheld && !options.cleanup_selections.is_empty() {
            bail!(
                "Files can't be selected for removal while an unfinished install or update is replaced. Repair without selecting files, then open Repair again to remove them."
            );
        }

        let context = PackProofContext::from(&modpack);
        let origin = self.origin_checks.read().await.get(&instance_id).cloned();
        validate_cleanup_selections(&options.cleanup_selections, &context, origin.as_ref())?;
        if !options.cleanup_paths.is_empty() {
            tracing::warn!("legacy path-only repair cleanup is unbound and will be preserved");
        }
        // Persist this storage identifier in the repair marker for retry.
        // It never grants admission to setup or ownership of the instance.
        options.cleanup_batch = if options.cleanup_selections.is_empty() {
            None
        } else {
            match create_cleanup_batch(&instance_path.get_root()) {
                Ok(batch) => Some(batch),
                Err(error) => {
                    tracing::warn!("cleanup will be preserved: {error:#}");
                    None
                }
            }
        };

        let repair_marker_text = serde_json::to_string(&options)?;
        let pack_version_text = serde_json::to_string(&PackVersionFile::from(target))?;

        self.run_prepare_with_setup_cleanup(
            instance_id,
            operation,
            runtime_activity,
            session::Request {
                producer: session::Producer::Repair,
                kind: session::Kind::Pack,
                files: vec![
                    ("repair".into(), repair_marker_text.into_bytes()),
                    (
                        "change-pack-version.json".into(),
                        pack_version_text.into_bytes(),
                    ),
                ],
                options: Some(options),
                reset: true,
                replaces,
                warnings,
            },
        )
        .await
    }

    async fn run_prepare_with_setup_cleanup(
        self,
        instance_id: InstanceId,
        operation: super::InstanceOperationGuard,
        runtime_activity: crate::managers::runtime_lifecycle::RuntimeActivity,
        request: session::Request,
    ) -> anyhow::Result<VisualTaskId> {
        self.prepare_game_admitted(
            instance_id,
            None,
            None,
            true,
            operation,
            runtime_activity,
            Some(request),
        )
        .await
        .map(|result| result.1)
    }

    pub async fn pending_setup(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<PendingSetupStatus> {
        let _operation = self.lock_instance(instance_id).await;
        self.pending_setup_owned(instance_id).await
    }

    async fn pending_setup_owned(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<PendingSetupStatus> {
        Ok(self.inspect_pending_setup(instance_id).await?.0)
    }

    /// The pending setup's status, and what a Repair does with it (`None`
    /// when nothing is pending). Requires the instance gate.
    async fn inspect_pending_setup(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<(PendingSetupStatus, Option<PendingRepair>)> {
        let (shortpath, current) = self.current_config(instance_id).await?;
        let root = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_root();
        let configured = current.modpack.as_ref().map(|p| &p.modpack);
        let inactive = {
            let instances = self.instances.read().await;
            matches!(
                instances
                    .get(&instance_id)
                    .ok_or(InvalidInstanceIdError(instance_id))?
                    .data()?
                    .state,
                super::run::LaunchState::Inactive { .. }
            )
        };
        // A replacement a crash left half-published is the pending setup.
        // Only while Inactive: a running preparation owns its own setup.
        if inactive {
            if let Err(error) = session::settle_replacement(&root).await {
                tracing::warn!(?error, "unfinished setup replacement left as is");
            }
        }
        let mut status = PendingSetupStatus {
            pending: root.join(".setup").try_exists()?,
            operation: None,
            installed_version: configured.map(|p| PackProofContext::from(p).version_id),
            pending_version: None,
            can_cancel: false,
            reason: None,
            repair_version: configured.map(|p| PackProofContext::from(p).version_id),
            preview_withheld: false,
            can_resume: false,
            busy: !inactive,
            discard_token: None,
        };
        if !status.pending {
            return Ok((status, None));
        }
        let mut loaded = match session::Session::load(&root).await {
            // A 2.0.40 setup without a record that provably never reached
            // apply gets its session here, so the dialog can offer Cancel. Only
            // while Inactive: a running preparation owns its own adoption.
            Ok(None) if inactive => {
                session::Session::adopt_legacy(&root, instance_id, &current).await
            }
            other => other,
        };
        // A pack request that is not this instance's own (copied by a
        // duplicate, or kept from an earlier registration of the folder) and
        // whose archive hadn't arrived yet becomes this instance's own here,
        // as Play would make it, so it can be cancelled back to the
        // instance's own installed version like the source's can.
        if let Ok(Some(saved)) = &loaded {
            if inactive
                && saved.kind == session::Kind::Pack
                && matches!(
                    saved.copy_qualification(&root, instance_id).await,
                    Ok(Some(true))
                )
            {
                loaded = saved.qualify_copy(&root, instance_id).await.map(Some);
            }
        }
        // What Repair does with it: retire it, whatever it is, and install the
        // configured pack, or the pack an interrupted apply was installing.
        let mut repair = match &loaded {
            Ok(Some(session)) => {
                let mut interrupted = session.interrupted(&root);
                // Only a damaged request of a repair that continues an
                // interrupted setup fails here; Play refuses it as well.
                let target = session.repair_target().unwrap_or_else(|error| {
                    tracing::debug!(?error, "repairing the configured pack instead");
                    interrupted = true;
                    None
                });
                PendingRepair {
                    label: session.operation.clone(),
                    target,
                    interrupted,
                    keep_payload: session.changed_files(&root)
                        && session.producer != session::Producer::Repair,
                    warnings: session.warnings.clone(),
                }
            }
            // Without a readable record, the configured pack: 2.0.40 wrote an
            // update's pack there before applying it.
            _ => {
                let changed_files = session::unrecorded_interrupted(&root, &current);
                PendingRepair {
                    label: "legacy".into(),
                    target: None,
                    interrupted: changed_files,
                    keep_payload: changed_files,
                    warnings: Vec::new(),
                }
            }
        };
        // The preview plans the configured pack over its installed record.
        repair.interrupted |= repair
            .target
            .as_ref()
            .is_some_and(|target| Some(target) != configured);
        status.repair_version = repair
            .target
            .as_ref()
            .or(configured)
            .map(|p| PackProofContext::from(p).version_id);
        status.preview_withheld = repair.interrupted;
        match loaded {
            Ok(Some(session)) if session.owned_by(instance_id) => {
                status.operation = Some(session.operation.clone());
                status.can_resume = inactive;
                // Display only; `Session::load` already proved both parse.
                let version = |config: &str| {
                    super::schema::parse_instance_config(config)
                        .ok()
                        .and_then(|config| config.modpack)
                        .map(|p| PackProofContext::from(&p.modpack).version_id)
                };
                status.installed_version = version(&session.installed_config);
                if let Some(pending) = &session.pending_config {
                    status.pending_version = version(pending);
                } else if let Ok(text) = tokio::fs::read_to_string(root.join(".setup/change-pack-version.json")).await {
                    if let Ok(pack) = serde_json::from_str::<PackVersionFile>(&text) { status.pending_version = Some(PackProofContext::from(&Modpack::from(pack)).version_id); }
                }
                status.reason = session.cancel_reason(&current);
                if !inactive { status.reason = Some("Wait for the current operation to finish before cancelling.".into()); }
                if root.join(".setup/apply-started").try_exists()? || root.join(".setup/representation-transitions").try_exists()? {
                    status.reason = Some("Apply may have started. Resume the pending setup to finish safely.".into());
                }
                status.can_cancel = status.reason.is_none();
            }
            Ok(None) if !inactive => status.reason = Some("Wait for the current operation to finish before cancelling.".into()),
            // Play resumes these from the original archive or as a completed install.
            Ok(None) if session::legacy_resumed_by_play(&root).unwrap_or(false) => {
                status.can_resume = true;
                status.reason = Some("An earlier launcher version didn't finish this modpack's install or update. Press Play to finish it.".into());
            }
            Ok(None) => status.reason = Some("An earlier launcher version was interrupted while applying this setup's files, so it can't be cancelled safely. Its files have been kept.".into()),
            Ok(Some(session)) => {
                // A record that is not this instance's own: copied by a
                // duplicate, written before the instance had an id, or kept
                // from an earlier registration of the folder (see
                // `Session::binds_to`). Play continues it under this
                // instance's identity when it qualifies or still has its
                // original archive.
                let continues = matches!(session.copy_qualification(&root, instance_id).await, Ok(Some(true)))
                    || session.binds_to(instance_id) && session::has_original_archive(&root.join(".setup"));
                let copied = session.copied_to == Some(instance_id.0);
                status.can_resume = continues && inactive;
                status.reason = Some(match (continues, copied) {
                    (true, true) => "This instance was copied while its setup was unfinished. Press Play to finish it for this copy.".into(),
                    (true, false) => "This instance's setup hasn't finished. Press Play to finish it.".into(),
                    (false, _) => super::mods::uncontinued_setup_refusal(&current, super::mods::PendingChange::Play, copied),
                });
            }
            Err(error) => status.reason = Some(format!("Setup evidence cannot be verified: {error:#}")),
        }
        status.discard_token = Some(status.operation.clone().unwrap_or_else(|| "legacy".into()));
        Ok((status, Some(repair)))
    }

    pub async fn cancel_pending_setup(
        self,
        instance_id: InstanceId,
        expected_operation: String,
    ) -> anyhow::Result<()> {
        let _runtime_activity = self
            .app
            .runtime_lifecycle
            .admit(format!("instance {instance_id} cancel setup"))?;
        let _operation = self.lock_instance(instance_id).await;
        let status = self.pending_setup_owned(instance_id).await?;
        if status.operation.as_deref() != Some(&expected_operation) {
            bail!("pending operation changed; refresh its status before cancelling");
        }
        if !status.can_cancel {
            bail!(
                "{}",
                status
                    .reason
                    .unwrap_or_else(|| "pending setup cannot be safely cancelled".into())
            );
        }
        let (shortpath, _) = self.current_config(instance_id).await?;
        let root = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_root();
        let owner = session::Owner {
            instance_id: instance_id.0,
            operation: expected_operation,
        };
        let session = session::Session::require(&root, &owner).await?;
        let temporary = root.join("tmp-packinfo.json");
        if temporary.try_exists()? {
            let target = session
                .target
                .as_ref()
                .context("unbound temporary record prevents cancellation")?;
            let info = packinfo::parse_packinfo(&tokio::fs::read_to_string(&temporary).await?)?;
            if session::binding(&info, session.original_options.as_ref())?.0 != target.digest {
                bail!("temporary record belongs to a different target; preserve setup");
            }
            tokio::fs::rename(&temporary, root.join(".setup/cancelled-target.json")).await?;
        }
        // Rename retains evidence and atomically restores the ordinary no-setup
        // installed-version launch path, without downloading or rewriting config.
        let retained = session::retire(&root, &owner.operation).await?;
        // A cancellable setup never changed installed content, so its
        // downloads are evidence of nothing; its records stay.
        if let Err(error) = session::trim_retired(&retained).await {
            tracing::warn!(?error, "cancelled setup payload kept");
        }
        self.app.invalidate(
            crate::api::keys::instance::GET_PENDING_SETUP,
            Some(instance_id.0.into()),
        );
        self.app
            .invalidate(crate::api::keys::instance::GET_REPAIR_PREVIEW, None);
        self.app.invalidate(
            crate::api::keys::instance::INSTANCE_DETAILS,
            Some(instance_id.0.into()),
        );
        Ok(())
    }

    /// Computes what [`repair_modpack`](Self::repair_modpack) would do,
    /// read-only: it only reads the recorded `packinfo.json`, hashes what is
    /// already on disk (via [`disk_scan::scan_disk_state`]) once, and runs
    /// [`apply_plan::plan`] — the exact same pure planner the real repair
    /// uses — twice over that one disk scan, once per
    /// `re_enable_disabled` setting, followed by the DB-only duplicate-mod
    /// scan. Nothing is ever written to disk, no staged bytes are produced
    /// or consumed, the network is never touched, and no plan entry is ever
    /// executed.
    ///
    /// Returns both `re_enable_disabled` variants in one
    /// [`RepairPreview`] — [`RepairPreview::with_re_enable`] and
    /// [`RepairPreview::without_re_enable`] — rather than taking the flag as
    /// a parameter: `plan` is pure and cheap relative to the disk scan
    /// (`entries`/`counts` alone versus a real filesystem walk+hash), so the
    /// caller (`RepairModpack/index.tsx`, via `GET_REPAIR_PREVIEW`) can
    /// switch which variant it displays entirely client-side, on a query
    /// keyed only by instance — no re-scan, and no second network
    /// round-trip, just to reflect the re-enable checkbox toggling.
    ///
    /// Preview/execution asymmetry (documented, intentional): a real repair
    /// re-downloads and re-verifies the pack's true current manifest, so a
    /// modpack that changed server-side since the last install can surface
    /// new corruption the preview never saw. The preview instead treats the
    /// **recorded** `packinfo.json` as both `old` and `target` — it never
    /// hits the network — and synthesizes `staged` as the full set of
    /// packinfo-declared paths ("everything the pack declares is
    /// obtainable"), so the plan is never blocked on a real staging
    /// directory that (for a preview) was never populated.
    pub async fn repair_preview(self, instance_id: InstanceId) -> anyhow::Result<RepairPreview> {
        let _operation = self.lock_instance(instance_id).await;
        // A pending setup that may have changed installed files, or whose
        // repair installs another pack, would make the recorded pack's plan
        // describe something Repair won't do. Every other one is replaced by
        // exactly this plan.
        if self
            .pending_setup_owned(instance_id)
            .await?
            .preview_withheld
        {
            bail!(
                "This modpack's last install or update stopped while changing its files, so what Repair will change can't be listed. Repair reinstalls the modpack from a fresh download; your worlds are kept."
            );
        }

        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;
        let shortpath = instance.shortpath.clone();
        let context = instance
            .data()?
            .config
            .modpack
            .as_ref()
            .map(|p| PackProofContext::from(&p.modpack));
        drop(instances);

        // Cloned out (rather than holding the read lock across the rest of
        // this — potentially slow, disk-walking — function) the same way
        // `instances` above is: whatever the last completed
        // `check_pack_origin` run found, if any.
        let origin_results = self.origin_checks.read().await.get(&instance_id).cloned();

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path.get_instances().get_instance_path(&shortpath);
        let instance_root = instance_path.get_root();
        let data_path = instance_path.get_data_path();

        let packinfo_path = instance_root.join("packinfo.json");
        let recorded = match tokio::fs::read_to_string(&packinfo_path).await {
            Ok(text) => {
                Some(packinfo::parse_packinfo(&text).context("while parsing packinfo json")?)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };

        // Computed regardless of packinfo presence — a duplicate mod isn't a
        // packinfo concept, it's purely a fact about what's currently
        // installed, so even an instance with no recorded packinfo at all
        // still gets a meaningful duplicates list.
        let mods = mfcdb::get_instance_mods_full(&self.app.db, instance_id.0).await?;
        let candidates: Vec<DuplicateCandidate> = mods
            .into_iter()
            .map(|m| DuplicateCandidate {
                modid: m.modid,
                enabled: m.enabled,
                addon_type: AddonType::from_db_string(&m.addon_type).unwrap_or(AddonType::Mods),
                filename: m.filename,
            })
            .collect();
        let duplicates = group_duplicates(&candidates, recorded.as_ref(), &instance_path);

        let Some(recorded) = recorded else {
            return Ok(RepairPreview {
                has_packinfo: false,
                with_re_enable: RepairPlanVariant::default(),
                without_re_enable: RepairPlanVariant::default(),
                untracked: Vec::new(),
                duplicates,
            });
        };

        let universe: BTreeSet<String> = recorded.files.keys().cloned().collect();
        let disk_scan::DiskScan {
            states: disk,
            coexisting_disabled_twin_md5,
        } = disk_scan::scan_disk_state(&data_path, &universe).await?;
        // Synthetic: the preview never actually stages anything, so every
        // target path is simply assumed obtainable — see the doc comment
        // above for why this is the documented preview/execution asymmetry.
        let staged: HashSet<String> = universe.iter().cloned().collect();
        // See `disk_scan::probe_case_insensitive`'s own doc for the fallback
        // semantics; threaded into both the planner and the untracked-file
        // walk below for the same reason `process_modpack_staging` threads
        // it (`run/modpack.rs`).
        let fs_case_insensitive = disk_scan::probe_case_insensitive(&data_path).await;

        // `plan` itself never touches the filesystem, so running it twice —
        // once per `re_enable_disabled` setting — over the single disk scan
        // above is negligible next to the scan itself; see this method's own
        // doc for why both variants are computed unconditionally rather than
        // one selected by a parameter.
        let plan_variant = |re_enable_disabled: bool| -> anyhow::Result<RepairPlanVariant> {
            let mut entries = apply_plan::plan(apply_plan::PlanInputs {
                old: Some(&recorded),
                target: &recorded,
                staged: &staged,
                disk: &disk,
                coexisting_disabled_twin_md5: &coexisting_disabled_twin_md5,
                mode: apply_plan::ApplyMode::Repair { re_enable_disabled },
                fs_case_insensitive,
            })?;
            disk_scan::protect_resolved_saves(&mut entries, &data_path, Some(&recorded));
            let counts = tally_counts(&entries);
            Ok(RepairPlanVariant { entries, counts })
        };
        let with_re_enable = plan_variant(true)?;
        let without_re_enable = plan_variant(false)?;

        let untracked = untracked_files_for_preview(
            &data_path,
            &recorded,
            origin_results.as_ref(),
            context.as_ref(),
            fs_case_insensitive,
        )
        .await?;

        Ok(RepairPreview {
            has_packinfo: true,
            with_re_enable,
            without_re_enable,
            untracked,
            duplicates,
        })
    }
}

#[derive(specta::Type, Serialize, Debug, Clone)]
pub struct PendingSetupStatus {
    pub pending: bool,
    pub operation: Option<String>,
    /// The pack version the pending setup started from and the one it
    /// installs, by platform id. The dialog names versions by
    /// `repair_version` instead; these identify the setup for diagnostics
    /// and tests.
    pub installed_version: Option<String>,
    pub pending_version: Option<String>,
    pub can_cancel: bool,
    /// Why it can't be cancelled; the refusal `cancel_pending_setup` gives.
    pub reason: Option<String>,
    /// The pack version Repair installs: the configured one, or the one a
    /// pending setup that stopped while applying was installing.
    pub repair_version: Option<String>,
    /// The pending setup may have changed installed files (or Repair installs
    /// another pack than the configured one), so the repair preview, which
    /// plans the configured pack over its installed record, is withheld.
    pub preview_withheld: bool,
    /// Play continues the pending setup, so the dialog offers to resume it.
    pub can_resume: bool,
    /// The instance is being prepared, launched, run or deleted, so Repair
    /// (and Cancel or Resume) must wait until it stops.
    pub busy: bool,
    /// What a Repair confirmed against this status sends back unchanged as
    /// `expected_pending`: the pending operation this instance owns, `legacy`
    /// for any other pending setup, or nothing when none is pending. The
    /// repair is refused when the pending setup no longer matches it.
    pub discard_token: Option<String>,
}

/// What a Repair does with a pending setup (see
/// [`ManagerRef::repair_modpack`](ManagerRef::repair_modpack)).
#[derive(Debug, Clone)]
struct PendingRepair {
    /// Its retired label: its operation, or `legacy` without a readable record.
    label: String,
    /// The pack Repair installs instead of the configured one.
    target: Option<Modpack>,
    /// It may have changed installed content, or continues a setup that may
    /// have (see [`session::Session::interrupted`]), or Repair installs
    /// another pack than the configured one.
    interrupted: bool,
    /// Its payload stays once it is retired (see
    /// [`session::Replaced::keep_payload`]).
    keep_payload: bool,
    /// What it would have told the user once it succeeded (see
    /// [`session::Request::warnings`]).
    warnings: Vec<crate::domain::vtask::TaskWarning>,
}

/// Identity of the installed pack whose preview was reviewed.
#[derive(specta::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PackProofContext {
    pub platform: String,
    pub project_id: String,
    pub version_id: String,
}
impl From<&Modpack> for PackProofContext {
    fn from(pack: &Modpack) -> Self {
        match pack {
            Modpack::Modrinth(p) => Self {
                platform: "modrinth".into(),
                project_id: p.project_id.clone(),
                version_id: p.version_id.clone(),
            },
            Modpack::Curseforge(p) => Self {
                platform: "curseforge".into(),
                project_id: p.project_id.to_string(),
                version_id: p.file_id.to_string(),
            },
        }
    }
}

/// Frozen at selection, including manual choices. Never refresh its hash
/// at submission or while resuming a persisted repair.
#[derive(specta::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CleanupSelection {
    pub path: String,
    pub sha512: String,
    pub context: PackProofContext,
    pub proven: bool,
}

pub(crate) fn validate_cleanup_selections(
    selections: &[CleanupSelection],
    context: &PackProofContext,
    origin: Option<&origin_check::OriginResults>,
) -> anyhow::Result<()> {
    let mut seen = HashSet::new();
    for selection in selections {
        let mut hash = [0u8; 64];
        if normalize_cleanup_path(&selection.path).as_ref() != Some(&selection.path)
            || !seen.insert(&selection.path)
            || &selection.context != context
            || hex::decode_to_slice(&selection.sha512, &mut hash).is_err()
        {
            bail!(
                "invalid or obsolete repair cleanup selection for {}",
                selection.path
            );
        }
        if selection.proven
            && !matches!(
                origin_verdict_for(origin, &selection.path, &hash, Some(context)),
                Some(OriginVerdict::ShippedIn { .. })
            )
        {
            bail!(
                "repair origin proof no longer applies to {}",
                selection.path
            );
        }
    }
    Ok(())
}

pub(crate) fn create_cleanup_batch(root: &Path) -> anyhow::Result<String> {
    let recovery = root.join(".repair-recovery");
    match std::fs::create_dir(&recovery) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::symlink_metadata(&recovery)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                bail!("repair recovery root is not a plain directory");
            }
        }
        Err(e) => return Err(e.into()),
    }
    let batch = tempfile::Builder::new()
        .prefix("cleanup-")
        .tempdir_in(&recovery)?
        .keep();
    Ok(batch.file_name().unwrap().to_string_lossy().into_owned())
}

/// Body of the `.setup/repair` marker `repair_modpack` writes. Its mere
/// presence (checked by `run/modpack.rs::process_modpack`) is what switches
/// the staging pipeline from an ordinary version-change reconciliation to a
/// disk-scan-driven repair; an absent marker (including a `.setup` left by
/// an older build that never wrote one) falls back to
/// `apply_plan::ApplyMode::VersionChange` unchanged.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct RepairMarkerFile {
    pub re_enable_disabled: bool,
    /// Packinfo-style keys the user ticked for removal in the preview.
    #[serde(default)]
    pub cleanup_paths: Vec<String>,
    #[serde(default)]
    pub cleanup_selections: Vec<CleanupSelection>,
    /// Relative storage batch, allocated by admission; not setup ownership.
    #[serde(default)]
    pub cleanup_batch: Option<String>,
}

/// Syntactically validates and normalizes one `RepairMarkerFile::cleanup_paths`
/// entry. Returns the rebuilt packinfo-style key — always exactly the
/// re-joined validated components, *never* the original string — or `None`
/// if the path fails any check. Two independent callers rely on this being
/// the single source of truth for "is this cleanup path even well-formed":
/// `repair_modpack` (route-time, fails the whole request fast) and
/// `run/modpack.rs::apply_user_cleanup` (apply-time, skips just this one
/// entry — see its own docs for why it can never bail).
///
/// Requires a leading `/`, and every remaining path component must be
/// [`Component::Normal`] and valid UTF-8. This alone is what kills a `.` or
/// `..` segment, a smuggled second root (`//etc/passwd`, whose tail after
/// stripping one leading `/` is itself still absolute), and — on Windows —
/// a smuggled drive prefix: [`Path::join`]'s documented behaviour is that an
/// *absolute* `path` argument replaces the base entirely, so any of those
/// would silently escape the instance the moment a caller joined the result
/// onto the instance data dir, however careful the joining code is. A `..`
/// occurring only as a substring of a longer segment (`foo..bar.jar`) is
/// unaffected — component parsing only treats a segment as `ParentDir` when
/// the *entire* segment is `..`, never as a substring match.
///
/// Also rejects a path that normalizes to nothing (bare `/`) and one that
/// lands under `/saves` — repair must never be able to touch a save file via
/// this path any more than the planner itself can. Callers still owe their
/// own pack-tracked-path check; this only proves the path is syntactically
/// safe to resolve and would never touch `/saves`.
pub fn normalize_cleanup_path(path: &str) -> Option<String> {
    let normalized = normalize_pack_path(path)?;
    if apply_plan::is_saves_path(&normalized) {
        return None;
    }

    Some(normalized)
}

/// [`normalize_cleanup_path`] without the `/saves` refusal, for paths a pack
/// itself ships: a CurseForge world addon or a pack's `saves` overrides stage
/// a world the apply creates (the planner still never changes an existing
/// save).
pub fn normalize_pack_path(path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return None;
    }

    let mut parts = Vec::new();
    for component in Path::new(&path[1..]).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            _ => return None,
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(format!("/{}", parts.join("/")))
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "platform")]
pub enum PackVersionFile {
    Curseforge {
        project_id: u32,
        file_id: u32,
    },
    Modrinth {
        project_id: String,
        version_id: String,
    },
}

impl From<Modpack> for PackVersionFile {
    fn from(value: Modpack) -> Self {
        match value {
            Modpack::Curseforge(CurseforgeModpack {
                project_id,
                file_id,
            }) => Self::Curseforge {
                project_id,
                file_id,
            },
            Modpack::Modrinth(ModrinthModpack {
                project_id,
                version_id,
            }) => Self::Modrinth {
                project_id,
                version_id,
            },
        }
    }
}

impl From<PackVersionFile> for Modpack {
    fn from(value: PackVersionFile) -> Self {
        match value {
            PackVersionFile::Curseforge {
                project_id,
                file_id,
            } => Self::Curseforge(CurseforgeModpack {
                project_id,
                file_id,
            }),
            PackVersionFile::Modrinth {
                project_id,
                version_id,
            } => Self::Modrinth(ModrinthModpack {
                project_id,
                version_id,
            }),
        }
    }
}

/// Manager-side result of [`ManagerRef::repair_preview`]; the FE-facing
/// `FERepairPreview` (`api/instance/mod.rs`) is built from this via `From`.
/// `with_re_enable`/`without_re_enable` are the two `plan` outcomes over the
/// SAME disk scan — see [`ManagerRef::repair_preview`]'s own doc for why both
/// are always computed rather than one selected by a parameter; every other
/// field (`untracked`, `duplicates`, `has_packinfo`) doesn't depend on
/// `re_enable_disabled` at all, so it isn't duplicated per variant.
#[derive(Debug)]
pub struct RepairPreview {
    pub has_packinfo: bool,
    pub with_re_enable: RepairPlanVariant,
    pub without_re_enable: RepairPlanVariant,
    pub untracked: Vec<UntrackedFile>,
    pub duplicates: Vec<DuplicateGroup>,
}

/// One `re_enable_disabled` setting's worth of [`apply_plan::plan`] output —
/// see [`RepairPreview::with_re_enable`]/[`RepairPreview::without_re_enable`].
#[derive(Debug, Default)]
pub struct RepairPlanVariant {
    /// Path-sorted (guaranteed by [`apply_plan::plan`]'s own output).
    pub entries: Vec<apply_plan::PlanEntry>,
    pub counts: RepairCounts,
}

/// Per-bucket tallies over [`RepairPreview::entries`] — see [`tally_counts`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RepairCounts {
    pub restore_modified: u32,
    pub restore_deleted: u32,
    pub unchanged: u32,
    pub disabled_kept: u32,
    pub re_enabled: u32,
    pub stale_dropped: u32,
    pub saves_skipped: u32,
}

/// One file [`untracked_files_for_preview`] found on disk that no packinfo
/// key names literally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UntrackedFile {
    pub path: String,
    pub size: u64,
    pub label: UntrackedLabel,
    /// Whether this exact path is a member of the set
    /// [`walk_untracked_files`] would actually remove if ticked for cleanup.
    /// Always `true` for [`UntrackedLabel::Unknown`]. For
    /// [`UntrackedLabel::DisabledPackFile`] this depends on coexistence —
    /// see [`is_coexisting_disabled_twin`]: `true` for a stale twin left
    /// beside its still-present enabled copy, `false` for a tracked path's
    /// *sole* on-disk representation, which cleanup must never be able to
    /// remove out from under the planner.
    pub deletable: bool,
    pub origin: Option<OriginVerdict>,
    pub selection: Option<CleanupSelection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UntrackedLabel {
    Unknown,
    DisabledPackFile,
}

/// Filled in by the origin checker (a later task) via [`origin_verdict_for`];
/// always `None` until that checker exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginVerdict {
    ShippedIn {
        version_name: String,
        version_id: String,
    },
    CurrentVersion,
    Incomplete,
    Unknown,
}

/// Every currently-enabled mod file sharing a modid with at least one other
/// currently-enabled mod file — see [`group_duplicates`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub modid: String,
    pub files: Vec<DuplicateSide>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateSide {
    pub path: String,
    pub pack_owned: bool,
}

/// A packinfo-style key's top-level directory segment, for restricting a
/// walk to only the directories a pack could plausibly reference:
/// `"mods"` for `/mods/a.jar`. A key with no nested directory at all
/// (`/options.txt`) has no such segment, so it maps to the empty string — a
/// pseudo top-level shared by every root-level file. That sharing is
/// deliberate: a single packinfo-tracked root file is enough to allow
/// walking every OTHER root-level file too, not just that one exact name —
/// mirroring how a single tracked file under `/mods` allows walking the
/// rest of `/mods`, not just that one file.
fn top_level_segment(key: &str) -> &str {
    let rest = key.strip_prefix('/').unwrap_or(key);
    rest.split_once('/').map_or("", |(dir, _)| dir)
}

/// The top-level segments (see [`top_level_segment`]) `target` — and, if
/// given, `old` — declare, unioned. This is the walk-restriction allow-list
/// both [`walk_untracked_files`] and [`untracked_files_for_preview`] derive
/// from whatever packinfo(s) they have and pass down to [`walk_data_files`],
/// so a directory the pack has never referenced at all (logs, crash
/// reports, screenshots, …) is never surfaced as untracked content. A
/// target with zero tracked files yields an empty allow-list, hence an
/// empty walk — there is nothing to compare "untracked" against.
fn top_level_segments(
    target: &packinfo::PackInfo,
    old: Option<&packinfo::PackInfo>,
) -> HashSet<String> {
    target
        .files
        .keys()
        .chain(old.into_iter().flat_map(|o| o.files.keys()))
        .map(|k| top_level_segment(k).to_string())
        .collect()
}

/// True iff `key` — assumed to already be classified untracked (its raw
/// form is absent from packinfo) and to end in `.disabled` — has its bare
/// (enabled) spelling ALSO present in the very same walk, i.e. `all_files`
/// (no extra I/O: the walk already found it if it's there). This is what
/// distinguishes a genuinely stale, safely-removable leftover twin (the
/// pack's own enabled copy is right there, so the `.disabled` file is pure
/// redundant garbage) from a tracked path's *sole* on-disk representation,
/// which cleanup must never be allowed to remove — that copy, disabled or
/// not, is what the planner itself owns and will act on. Meaningless
/// (returns `false`) for a key with no `.disabled` suffix, since such a key
/// has no "bare" counterpart to coexist with.
fn is_coexisting_disabled_twin(key: &str, all_files: &HashMap<String, PathBuf>) -> bool {
    key.strip_suffix(".disabled")
        .is_some_and(|bare| all_files.contains_key(bare))
}

/// Walks `instance_data` once (skipping `/saves`, `/.install_audit`, and any
/// top-level directory or root-level file not named by `allowed_top_level` —
/// see [`top_level_segments`]) and returns every remaining file's raw walked
/// key mapped to that file's own real [`PathBuf`] — never normalized, cased,
/// or otherwise derived from anything other than what the filesystem
/// actually reports. Shared by [`walk_untracked_files`] (repair's
/// user-requested-cleanup eligibility set, consumed by
/// `run/modpack.rs::apply_user_cleanup`) and [`untracked_files_for_preview`]
/// (the repair preview's untracked-file listing) — each derives its own
/// `allowed_top_level` from whatever packinfo(s) it has and applies its own
/// classification on top of this single walk, rather than walking the tree
/// a second time.
///
/// `/saves` and `/.install_audit` are excluded unconditionally, regardless
/// of `allowed_top_level` — a modpack override can legitimately ship files
/// under `/saves` (packinfo CAN track a `/saves/...` key; the planner always
/// forces `Keep`/`InSaveFolder` for it regardless), which would otherwise
/// let `"saves"` slip into the allow-list and defeat this protection.
///
/// Never bails: a walk failure is `tracing::warn!`-logged and simply yields
/// however much of the set was collected before the failure (possibly
/// empty). Note this restricts the *returned set*, not the underlying
/// traversal — `NormalizedWalkdir` has no subtree-skip API, so an excluded
/// top-level directory's contents are still enumerated on disk, just never
/// inserted into the result.
pub async fn walk_data_files(
    instance_data: &Path,
    allowed_top_level: &HashSet<String>,
) -> HashMap<String, PathBuf> {
    let mut files = HashMap::new();

    let mut walker = match NormalizedWalkdir::new(instance_data) {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("failed to walk instance data dir {instance_data:?}: {e}");
            return files;
        }
    };

    loop {
        let entry = match walker.next() {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(e) => {
                tracing::warn!("failed to walk instance data dir {instance_data:?}: {e}");
                break;
            }
        };
        if entry.is_dir {
            continue;
        }

        let key = entry.relative_path.to_string();
        if apply_plan::is_saves_path(&key)
            || disk_scan::resolves_into_saves(instance_data, &entry.entry.path())
            || key.starts_with("/.install_audit")
        {
            continue;
        }
        if !allowed_top_level.contains(top_level_segment(&key)) {
            continue;
        }

        files.insert(key, entry.entry.path());
    }

    files
}

/// Every file [`walk_data_files`] finds that is safe to remove on a
/// user-supplied cleanup request — the ground truth
/// `run/modpack.rs::apply_user_cleanup` checks a cleanup path against,
/// keyed by the raw spelling the walk itself observed and mapped to that
/// same entry's own real [`PathBuf`].
///
/// A file whose raw key IS present in `old` or `target` is never included —
/// that is the pack's own live copy, and deleting it would fight the
/// planner, which already owns that path. When `fs_case_insensitive` is set,
/// a raw key that merely case-aliases a tracked key (e.g. disk spells it
/// `/mods/foo.jar`, packinfo tracks `/mods/Foo.jar`) is treated identically —
/// on such a filesystem the two spellings resolve to the same physical file,
/// so it is exactly as pack-owned as an exact match and must never be
/// classified untracked or deletable either. A file whose raw key is absent
/// but whose `.disabled`-stripped key IS tracked (exactly or, when
/// case-insensitive, by fold) is included only when
/// [`is_coexisting_disabled_twin`] finds its enabled sibling ALSO on disk
/// right now — a genuinely stale leftover twin — and excluded when it's the
/// tracked path's *sole* on-disk representation, disabled or not, which is
/// exactly what the planner already owns and will act on; cleanup must
/// never be able to remove it out from under that. Anything else is
/// unrelated to the pack entirely and always included.
pub async fn walk_untracked_files(
    instance_data: &Path,
    old_packinfo: Option<&packinfo::PackInfo>,
    target_packinfo: &packinfo::PackInfo,
    fs_case_insensitive: bool,
) -> HashMap<String, PathBuf> {
    let all_files = walk_packinfo_scoped_files(instance_data, target_packinfo, old_packinfo).await;

    // Built once for the whole walk, not per lookup, and only when the
    // filesystem itself can't tell two differently-cased spellings apart.
    let folded = if fs_case_insensitive {
        fold_tracked_keys(target_packinfo, old_packinfo)
    } else {
        HashMap::new()
    };

    all_files
        .iter()
        .filter(|(key, _)| {
            let raw_tracked = target_packinfo.files.contains_key(key.as_str())
                || old_packinfo.is_some_and(|p| p.files.contains_key(key.as_str()));
            if raw_tracked {
                return false;
            }
            if fs_case_insensitive && folded.contains_key(&key.to_ascii_lowercase()) {
                return false;
            }

            let bare_tracked = key.strip_suffix(".disabled").is_some_and(|k| {
                target_packinfo.files.contains_key(k)
                    || old_packinfo.is_some_and(|p| p.files.contains_key(k))
                    || (fs_case_insensitive && folded.contains_key(&k.to_ascii_lowercase()))
            });
            if bare_tracked {
                return is_coexisting_disabled_twin(key, &all_files);
            }

            true
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// ASCII-only case fold of every key `target` tracks, unioned with `old`'s if
/// given, mapping the folded spelling to the original. Built once per walk by
/// both [`walk_untracked_files`] and [`untracked_files_for_preview`], and
/// consulted only when their own `fs_case_insensitive` flag is set. ASCII
/// fold only, deliberately: NTFS/APFS case-folding tables differ from
/// Unicode simple folding (e.g. Turkish İ), and ASCII covers the practical
/// modpack namespace without chasing filesystem-specific folding rules.
fn fold_tracked_keys<'a>(
    target: &'a packinfo::PackInfo,
    old: Option<&'a packinfo::PackInfo>,
) -> HashMap<String, &'a str> {
    target
        .files
        .keys()
        .chain(old.into_iter().flat_map(|p| p.files.keys()))
        .map(|k| (k.to_ascii_lowercase(), k.as_str()))
        .collect()
}

/// Walks `instance_data` scoped to `packinfo`'s (and, if given, `old`'s)
/// referenced top-level directories and returns every file found there —
/// both tracked and untracked — keyed by its raw walked path. A thin,
/// shared wrapper around [`top_level_segments`] + [`walk_data_files`].
///
/// Deliberately does NOT filter tracked keys out itself, despite what a name
/// like "untracked files" might suggest: [`untracked_files_for_preview`]
/// needs the *tracked* entries still present in this same map too, to tell a
/// stale coexisting disabled twin (see [`is_coexisting_disabled_twin`]) from
/// a tracked path's sole on-disk representation — a genuinely
/// untracked-only view would silently break that check. Callers that only
/// want the untracked subset (e.g. `origin_check`) filter
/// `packinfo.files.contains_key(key)` out themselves.
///
/// Shared by [`untracked_files_for_preview`], [`walk_untracked_files`], and
/// [`origin_check::run_check_pack_origin`](origin_check).
async fn walk_packinfo_scoped_files(
    instance_data: &Path,
    packinfo: &packinfo::PackInfo,
    old: Option<&packinfo::PackInfo>,
) -> HashMap<String, PathBuf> {
    let allowed = top_level_segments(packinfo, old);
    walk_data_files(instance_data, &allowed).await
}

/// Builds [`RepairPreview::untracked`]: every file [`walk_data_files`] finds
/// whose raw walked key is not itself a literal `packinfo` key. A tracked
/// path's own on-disk file (enabled or disabled) is excluded here — it is
/// already represented in [`apply_plan::plan`]'s `entries` — but a
/// `.disabled`-suffixed key whose *stripped* form IS a packinfo key
/// (typically a stale twin left beside a since-restored enabled copy — the
/// planner has no way to see a path's "other" spelling once
/// [`disk_scan::scan_disk_state`] picks one, and it never deletes such
/// twins outright) is still listed, labeled
/// [`UntrackedLabel::DisabledPackFile`] rather than [`UntrackedLabel::Unknown`]
/// so the UI can tell the user it is a disabled copy of a real pack file
/// rather than unrelated content. `deletable` mirrors exactly what
/// [`walk_untracked_files`] would actually remove for that same path — see
/// [`UntrackedFile::deletable`]. `origin_results` is the instance's last
/// completed [`origin_check::check_pack_origin`](InstanceManager) run, if
/// any — see [`origin_verdict_for`]. When `fs_case_insensitive` is set, a raw
/// key that merely case-aliases a `packinfo` key (or its `.disabled`-stripped
/// form) is treated as tracked too, the same fold [`walk_untracked_files`]
/// applies — see [`fold_tracked_keys`].
async fn untracked_files_for_preview(
    instance_data: &Path,
    packinfo: &packinfo::PackInfo,
    origin_results: Option<&origin_check::OriginResults>,
    context: Option<&PackProofContext>,
    fs_case_insensitive: bool,
) -> anyhow::Result<Vec<UntrackedFile>> {
    // old == target == packinfo for a preview (see `repair_preview`'s own
    // docs), so packinfo's own keys already cover the union this needs.
    let all_files = walk_packinfo_scoped_files(instance_data, packinfo, None).await;

    // Built once for the whole preview, not per lookup, and only when the
    // filesystem itself can't tell two differently-cased spellings apart.
    let folded = if fs_case_insensitive {
        fold_tracked_keys(packinfo, None)
    } else {
        HashMap::new()
    };

    let mut untracked = Vec::new();

    for (key, path) in &all_files {
        if packinfo.files.contains_key(key) {
            continue;
        }
        if fs_case_insensitive && folded.contains_key(&key.to_ascii_lowercase()) {
            continue;
        }

        let (label, deletable) = match key.strip_suffix(".disabled") {
            Some(bare)
                if packinfo.files.contains_key(bare)
                    || (fs_case_insensitive && folded.contains_key(&bare.to_ascii_lowercase())) =>
            {
                (
                    UntrackedLabel::DisabledPackFile,
                    is_coexisting_disabled_twin(key, &all_files),
                )
            }
            _ => (UntrackedLabel::Unknown, true),
        };

        let size = tokio::fs::metadata(path).await?.len();
        let hash = origin_check::hash_file_sha512(path).await.ok();
        let origin = hash
            .as_ref()
            .and_then(|hash| origin_verdict_for(origin_results, key, hash, context));
        let selection = context
            .zip(hash)
            .filter(|_| deletable && origin != Some(OriginVerdict::CurrentVersion))
            .map(|(context, hash)| CleanupSelection {
                path: key.clone(),
                sha512: hex::encode(hash),
                context: context.clone(),
                proven: matches!(origin, Some(OriginVerdict::ShippedIn { .. })),
            });

        untracked.push(UntrackedFile {
            path: key.clone(),
            size,
            label,
            deletable,
            origin,
            selection,
        });
    }

    untracked.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(untracked)
}

/// Seam for the origin checker: given the instance's cached
/// [`origin_check::OriginResults`] (if [`check_pack_origin`](InstanceManager)
/// has ever completed for it) and an untracked file's path, returns its
/// cached origin verdict. `None` when no check has ever run, or when this
/// exact path wasn't assigned a verdict by the last completed run (e.g. it
/// appeared on disk after that run, or that run failed before reaching it).
fn origin_verdict_for(
    results: Option<&origin_check::OriginResults>,
    path: &str,
    hash: &[u8; 64],
    context: Option<&PackProofContext>,
) -> Option<OriginVerdict> {
    let results = results?;
    if context.is_none()
        || results.context.as_ref() != context
        || results.checked_hashes.get(path) != Some(hash)
    {
        return None;
    }
    results.verdicts.get(path).cloned()
}

/// Tallies [`RepairPreview::counts`] from the `(action, reason)` of every
/// plan entry. Each bucket maps to exactly one combination the
/// [`Repair`](apply_plan::ApplyMode::Repair) planner can produce; an
/// `(action, reason)` pair outside all seven buckets (only reachable in
/// [`ApplyMode::VersionChange`](apply_plan::ApplyMode::VersionChange), which
/// the preview never runs) is silently not counted rather than treated as an
/// error, so this stays total over any input.
fn tally_counts(entries: &[apply_plan::PlanEntry]) -> RepairCounts {
    use apply_plan::{PlanAction::*, PlanReason::*};

    let mut counts = RepairCounts::default();

    for entry in entries {
        match (&entry.action, &entry.reason) {
            (Replace, RepairOverwrote { .. }) | (ReplaceDisabled, RepairOverwrote { .. }) => {
                counts.restore_modified += 1
            }
            (Create, RepairRestored) => counts.restore_deleted += 1,
            (Keep, Unchanged) => counts.unchanged += 1,
            (Keep, DisabledByUser) => counts.disabled_kept += 1,
            (ReEnable, _) => counts.re_enabled += 1,
            (Delete, _) => counts.stale_dropped += 1,
            (Keep, InSaveFolder) => counts.saves_skipped += 1,
            _ => {}
        }
    }

    counts
}

/// Minimal per-file view [`group_duplicates`] needs. Deliberately not
/// [`carbon_repos::repos::mod_file_cache::ModFullRow`] itself, whose ~30
/// largely-irrelevant metadata columns would make the pure grouping logic
/// painful to construct in tests.
#[derive(Debug, Clone)]
struct DuplicateCandidate {
    modid: Option<String>,
    enabled: bool,
    addon_type: AddonType,
    filename: String,
}

/// Groups every currently-enabled candidate by modid; a group with more than
/// one member is a real duplicate (the same mod installed under more than
/// one file). `path` is rebuilt in packinfo-style form (`/<addon folder
/// name>/<filename>`) from the addon type's actual on-disk folder NAME
/// (e.g. `shaderpacks`, not [`AddonType::to_db_string`]'s `shaders`) via
/// [`AddonType::get_folder_path`], mirroring the folder resolution
/// `mods.rs` (`enable_mod`/`delete_mod`) already uses — so `pack_owned` can
/// check that exact string against `packinfo`.
fn group_duplicates(
    candidates: &[DuplicateCandidate],
    packinfo: Option<&packinfo::PackInfo>,
    instance_path: &InstancePath,
) -> Vec<DuplicateGroup> {
    let mut by_modid: HashMap<&str, Vec<&DuplicateCandidate>> = HashMap::new();
    for candidate in candidates {
        if !candidate.enabled {
            continue;
        }
        if let Some(modid) = &candidate.modid {
            by_modid.entry(modid.as_str()).or_default().push(candidate);
        }
    }

    let mut groups: Vec<DuplicateGroup> = by_modid
        .into_iter()
        .filter(|(_, members)| members.len() > 1)
        .map(|(modid, members)| {
            let mut files: Vec<DuplicateSide> = members
                .into_iter()
                .map(|candidate| {
                    let folder_name = candidate
                        .addon_type
                        .get_folder_path(instance_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| candidate.addon_type.to_db_string().to_string());
                    let path = format!("/{folder_name}/{}", candidate.filename);
                    let pack_owned = packinfo.is_some_and(|p| p.files.contains_key(&path));

                    DuplicateSide { path, pack_owned }
                })
                .collect();
            files.sort_by(|a, b| a.path.cmp(&b.path));

            DuplicateGroup {
                modid: modid.to_string(),
                files,
            }
        })
        .collect();

    groups.sort_by(|a, b| a.modid.cmp(&b.modid));
    groups
}

#[cfg(test)]
mod tests {
    use super::apply_plan::{PlanAction, PlanEntry, PlanReason};
    use super::packinfo::{self, FileHashes, PackInfo};
    use super::{
        AddonType, DuplicateCandidate, RepairCounts, RepairMarkerFile, UntrackedLabel,
        group_duplicates, normalize_cleanup_path, normalize_pack_path, tally_counts,
        untracked_files_for_preview, walk_untracked_files,
    };
    use carbon_rt_path::InstancePath;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// One request ending leaves the other's check stoppable.
    #[test]
    fn version_change_check_ending_only_removes_its_own_entry() {
        use super::{InstanceId, VersionChangeCheck, VersionChangeChecks};
        use tokio_util::sync::CancellationToken;
        let checks = VersionChangeChecks::new();
        let id = InstanceId(7);
        let (first, second) = (CancellationToken::new(), CancellationToken::new());
        let first_check = VersionChangeCheck::begin(&checks, id, &first);
        let second_check = VersionChangeCheck::begin(&checks, id, &second);
        drop(second_check);
        assert_eq!(checks.get(&id).unwrap().len(), 1);
        checks.get(&id).unwrap()[0].cancel();
        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
        drop(first_check);
        assert!(!checks.contains_key(&id));
    }

    // --- normalize_cleanup_path ---------------------------------------

    #[test]
    fn accepts_and_normalizes_a_well_formed_path() {
        assert_eq!(
            normalize_cleanup_path("/mods/leftover.jar"),
            Some("/mods/leftover.jar".to_string())
        );
    }

    #[test]
    fn accepts_a_filename_that_merely_contains_a_dotdot_substring() {
        // ".." only matters as an entire path COMPONENT, never as a
        // substring of a longer segment — "foo..bar.jar" is a perfectly
        // legal, single filename and must not be treated as traversal.
        assert_eq!(
            normalize_cleanup_path("/mods/foo..bar.jar"),
            Some("/mods/foo..bar.jar".to_string())
        );
    }

    #[test]
    fn rejects_a_path_missing_the_leading_slash() {
        assert_eq!(normalize_cleanup_path("mods/relative.jar"), None);
    }

    #[test]
    fn rejects_a_bare_root() {
        assert_eq!(normalize_cleanup_path("/"), None);
    }

    #[test]
    fn rejects_a_curdir_component() {
        assert_eq!(normalize_cleanup_path("/./mods/tracked.jar"), None);
    }

    #[test]
    fn rejects_a_parentdir_component() {
        assert_eq!(normalize_cleanup_path("/mods/../etc/passwd"), None);
    }

    #[test]
    fn rejects_a_smuggled_second_root() {
        // Stripping exactly one leading '/' from "//etc/passwd" leaves
        // "/etc/passwd" — still absolute. `Path::join`'s documented
        // behaviour for an absolute `path` argument is to replace the base
        // entirely, so joining this onto the instance data dir would
        // escape it completely rather than merely resolve to a weird
        // sub-path.
        assert_eq!(normalize_cleanup_path("//etc/passwd"), None);
    }

    #[test]
    fn rejects_a_saves_prefixed_path() {
        assert_eq!(normalize_cleanup_path("/saves/world/level.dat"), None);
    }

    #[test]
    fn rejects_a_curdir_path_that_would_otherwise_land_under_saves() {
        // Must be rejected at the component-validity stage, before ever
        // getting far enough to ask "is the normalized key under /saves?".
        assert_eq!(normalize_cleanup_path("/./saves/world/level.dat"), None);
    }

    #[test]
    fn pack_paths_keep_saves_and_still_refuse_traversal() {
        assert_eq!(
            normalize_pack_path("/saves/world/level.dat"),
            Some("/saves/world/level.dat".to_string())
        );
        assert_eq!(normalize_pack_path("/saves/../mods/a.jar"), None);
        assert_eq!(normalize_pack_path("//etc/passwd"), None);
        assert_eq!(normalize_pack_path("/"), None);
    }

    // --- RepairMarkerFile -----------------------------------------------

    #[test]
    fn repair_marker_file_round_trips_through_json() {
        let marker = RepairMarkerFile {
            re_enable_disabled: true,
            cleanup_paths: vec![
                "/mods/leftover.jar".to_string(),
                "/config/old.cfg".to_string(),
            ],
            ..Default::default()
        };

        let text = serde_json::to_string(&marker).expect("marker must serialize");
        let parsed: RepairMarkerFile =
            serde_json::from_str(&text).expect("marker must deserialize");

        assert_eq!(parsed, marker);
    }

    #[test]
    fn repair_marker_file_default_matches_the_repair_modpack_placeholder_args() {
        // `RepairModpack/index.tsx` currently sends `{ cleanup_paths: [],
        // re_enable_disabled: false }` (real values land in a later task) —
        // this pins `Default` staying in lockstep with that placeholder so a
        // future change to one is not silently inconsistent with the other.
        let marker = RepairMarkerFile::default();
        assert_eq!(marker.re_enable_disabled, false);
        assert!(marker.cleanup_paths.is_empty());
    }

    // --- untracked_files_for_preview -------------------------------------

    #[tokio::test]
    async fn untracked_files_for_preview_labels_unknown_vs_coexisting_disabled_pack_file_and_excludes_saves()
     {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        std::fs::create_dir_all(data.join("mods")).unwrap();
        // The live, pack-tracked copy — must never appear as untracked.
        std::fs::write(data.join("mods/tracked.jar"), b"tracked-bytes").unwrap();
        // A stray `.disabled` twin COEXISTING beside that live copy — a
        // stale leftover: untracked, labeled DisabledPackFile, and
        // (unlike a sole twin, see the dedicated test below) deletable.
        std::fs::write(data.join("mods/tracked.jar.disabled"), b"stale-twin-bytes").unwrap();
        // A file the packinfo has never heard of — plain Unknown, always
        // deletable.
        std::fs::write(data.join("mods/mystery.jar"), b"mystery-bytes").unwrap();
        // Never surfaced at all, tracked or not.
        std::fs::create_dir_all(data.join("saves/world")).unwrap();
        std::fs::write(data.join("saves/world/level.dat"), b"save-bytes").unwrap();

        let mut files = HashMap::new();
        files.insert(
            "/mods/tracked.jar".to_string(),
            FileHashes {
                sha512: [1; 64],
                md5: [1; 16],
            },
        );
        let packinfo = PackInfo { files };

        let result = untracked_files_for_preview(data, &packinfo, None, None, false)
            .await
            .expect("walking a real temp dir must not fail");

        let by_path: HashMap<&str, &super::UntrackedFile> =
            result.iter().map(|f| (f.path.as_str(), f)).collect();

        assert!(
            !by_path.contains_key("/mods/tracked.jar"),
            "a live tracked file must never appear as untracked, got {result:?}"
        );
        let twin = by_path
            .get("/mods/tracked.jar.disabled")
            .expect("stale disabled twin must be listed");
        assert_eq!(twin.label, UntrackedLabel::DisabledPackFile);
        assert!(
            twin.deletable,
            "a twin COEXISTING beside its live enabled copy is a stale leftover and must be deletable"
        );
        let mystery = by_path
            .get("/mods/mystery.jar")
            .expect("unrelated file must be listed");
        assert_eq!(mystery.label, UntrackedLabel::Unknown);
        assert!(mystery.deletable, "an Unknown row is always deletable");
        assert!(
            !by_path.keys().any(|k| k.starts_with("/saves")),
            "saves must be excluded entirely, got {result:?}"
        );
    }

    #[tokio::test]
    async fn untracked_files_for_preview_marks_a_sole_disabled_twin_not_deletable() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        std::fs::create_dir_all(data.join("mods")).unwrap();
        // ONLY the disabled spelling exists — this IS the tracked path's
        // sole on-disk representation (the planner owns it, e.g.
        // Keep/DisabledByUser), not a stale leftover.
        std::fs::write(data.join("mods/tracked.jar.disabled"), b"sole-twin-bytes").unwrap();

        let mut files = HashMap::new();
        files.insert(
            "/mods/tracked.jar".to_string(),
            FileHashes {
                sha512: [1; 64],
                md5: [1; 16],
            },
        );
        let packinfo = PackInfo { files };

        let result = untracked_files_for_preview(data, &packinfo, None, None, false)
            .await
            .expect("walking a real temp dir must not fail");

        let twin = result
            .iter()
            .find(|f| f.path == "/mods/tracked.jar.disabled")
            .expect("the sole twin must still be listed (labeled, just not deletable)");
        assert_eq!(twin.label, UntrackedLabel::DisabledPackFile);
        assert!(
            !twin.deletable,
            "a tracked path's SOLE on-disk representation must never be reported deletable"
        );
    }

    #[tokio::test]
    async fn untracked_files_for_preview_never_walks_a_directory_the_pack_never_references() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(data.join("mods/tracked.jar"), b"tracked-bytes").unwrap();
        std::fs::write(data.join("mods/mystery.jar"), b"mystery-bytes").unwrap();
        // None of these top-level dirs are ever packinfo-tracked by any
        // real modpack — every played instance accumulates them.
        std::fs::create_dir_all(data.join("logs")).unwrap();
        std::fs::write(data.join("logs/latest.log"), b"log-bytes").unwrap();
        std::fs::create_dir_all(data.join("crash-reports")).unwrap();
        std::fs::write(data.join("crash-reports/crash-1.txt"), b"crash-bytes").unwrap();
        std::fs::create_dir_all(data.join("screenshots")).unwrap();
        std::fs::write(data.join("screenshots/2026-08-07_1.png"), b"png-bytes").unwrap();

        let mut files = HashMap::new();
        files.insert(
            "/mods/tracked.jar".to_string(),
            FileHashes {
                sha512: [1; 64],
                md5: [1; 16],
            },
        );
        let packinfo = PackInfo { files };

        let result = untracked_files_for_preview(data, &packinfo, None, None, false)
            .await
            .expect("walking a real temp dir must not fail");

        assert_eq!(
            result.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
            vec!["/mods/mystery.jar"],
            "only the untracked file under an ALLOWED (packinfo-referenced) \
             directory may appear; logs/crash-reports/screenshots must never \
             surface, got {result:?}"
        );
    }

    // --- case-alias fold: walk_untracked_files / untracked_files_for_preview
    //
    // A packinfo tracks `/mods/Foo.jar`; the file actually on disk is
    // spelled `/mods/foo.jar` — the shape a case-only pack rename or a
    // user's own OS produces on a case-insensitive filesystem, where the
    // two spellings are one physical file. Byte-exact `contains_key`
    // against the tracked key alone would misclassify that disk spelling as
    // untracked and, in `walk_untracked_files`'s case, deletable — the
    // "mirror hazard" alongside the planner's own Delete-guard.

    #[tokio::test]
    async fn walk_untracked_files_case_variant_of_tracked_path_is_not_listed_when_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(data.join("mods/foo.jar"), b"tracked-bytes").unwrap();

        let target = PackInfo {
            files: HashMap::from([(
                "/mods/Foo.jar".to_string(),
                FileHashes {
                    sha512: [1; 64],
                    md5: [1; 16],
                },
            )]),
        };

        let result = walk_untracked_files(data, None, &target, true).await;

        assert!(
            !result.contains_key("/mods/foo.jar"),
            "a case-variant spelling of a tracked path must never be classified \
             untracked/deletable on a case-insensitive filesystem, got {result:?}"
        );
    }

    #[tokio::test]
    async fn walk_untracked_files_case_variant_of_tracked_path_is_listed_when_sensitive() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(data.join("mods/foo.jar"), b"tracked-bytes").unwrap();

        let target = PackInfo {
            files: HashMap::from([(
                "/mods/Foo.jar".to_string(),
                FileHashes {
                    sha512: [1; 64],
                    md5: [1; 16],
                },
            )]),
        };

        let result = walk_untracked_files(data, None, &target, false).await;

        assert!(
            result.contains_key("/mods/foo.jar"),
            "on a case-sensitive filesystem the two spellings are genuinely \
             distinct files, so the disk spelling must be listed untracked, \
             got {result:?}"
        );
    }

    #[tokio::test]
    async fn untracked_files_for_preview_case_variant_of_tracked_path_is_excluded_when_insensitive()
    {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(data.join("mods/foo.jar"), b"tracked-bytes").unwrap();

        let packinfo = PackInfo {
            files: HashMap::from([(
                "/mods/Foo.jar".to_string(),
                FileHashes {
                    sha512: [1; 64],
                    md5: [1; 16],
                },
            )]),
        };

        let result = untracked_files_for_preview(data, &packinfo, None, None, true)
            .await
            .expect("walking a real temp dir must not fail");

        assert!(
            !result.iter().any(|f| f.path == "/mods/foo.jar"),
            "a case-variant spelling of a tracked path must not be surfaced as \
             untracked on a case-insensitive filesystem, got {result:?}"
        );
    }

    #[tokio::test]
    async fn untracked_files_for_preview_case_variant_of_tracked_path_is_listed_when_sensitive() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(data.join("mods/foo.jar"), b"tracked-bytes").unwrap();

        let packinfo = PackInfo {
            files: HashMap::from([(
                "/mods/Foo.jar".to_string(),
                FileHashes {
                    sha512: [1; 64],
                    md5: [1; 16],
                },
            )]),
        };

        let result = untracked_files_for_preview(data, &packinfo, None, None, false)
            .await
            .expect("walking a real temp dir must not fail");

        assert!(
            result.iter().any(|f| f.path == "/mods/foo.jar"),
            "on a case-sensitive filesystem the disk spelling is a genuinely \
             distinct, untracked file, got {result:?}"
        );
    }

    // --- tally_counts ------------------------------------------------------

    #[test]
    fn tally_counts_maps_each_repair_bucket_from_hand_built_plan_entries() {
        let entries = vec![
            PlanEntry {
                path: "/mods/a.jar".to_string(),
                action: PlanAction::Replace,
                reason: PlanReason::RepairOverwrote {
                    original: [1; 16],
                    current: [2; 16],
                },
            },
            PlanEntry {
                path: "/mods/b.jar".to_string(),
                action: PlanAction::ReplaceDisabled,
                reason: PlanReason::RepairOverwrote {
                    original: [1; 16],
                    current: [2; 16],
                },
            },
            PlanEntry {
                path: "/mods/c.jar".to_string(),
                action: PlanAction::Create,
                reason: PlanReason::RepairRestored,
            },
            PlanEntry {
                path: "/mods/d.jar".to_string(),
                action: PlanAction::Keep,
                reason: PlanReason::Unchanged,
            },
            PlanEntry {
                path: "/mods/e.jar".to_string(),
                action: PlanAction::Keep,
                reason: PlanReason::DisabledByUser,
            },
            PlanEntry {
                path: "/mods/f.jar".to_string(),
                action: PlanAction::ReEnable,
                reason: PlanReason::ReEnabled,
            },
            PlanEntry {
                path: "/mods/g.jar".to_string(),
                action: PlanAction::Delete,
                reason: PlanReason::PackDropped,
            },
            PlanEntry {
                path: "/saves/world/level.dat".to_string(),
                action: PlanAction::Keep,
                reason: PlanReason::InSaveFolder,
            },
        ];

        let counts = tally_counts(&entries);

        assert_eq!(
            counts,
            RepairCounts {
                // Replace/RepairOverwrote (a.jar) + ReplaceDisabled/RepairOverwrote (b.jar)
                restore_modified: 2,
                restore_deleted: 1,
                unchanged: 1,
                disabled_kept: 1,
                re_enabled: 1,
                stale_dropped: 1,
                saves_skipped: 1,
            }
        );
    }

    #[test]
    fn tally_counts_of_empty_entries_is_all_zero() {
        assert_eq!(tally_counts(&[]), RepairCounts::default());
    }

    // --- group_duplicates ---------------------------------------------------

    #[test]
    fn group_duplicates_groups_enabled_files_sharing_a_modid_and_flags_pack_ownership() {
        let instance_path = InstancePath::new(PathBuf::from("/fake-instance"));

        let candidates = vec![
            DuplicateCandidate {
                modid: Some("jei".to_string()),
                enabled: true,
                addon_type: AddonType::Mods,
                filename: "jei-1.jar".to_string(),
            },
            DuplicateCandidate {
                modid: Some("jei".to_string()),
                enabled: true,
                addon_type: AddonType::Mods,
                filename: "jei-2.jar".to_string(),
            },
            // Disabled duplicate: must not count toward the group at all.
            DuplicateCandidate {
                modid: Some("jei".to_string()),
                enabled: false,
                addon_type: AddonType::Mods,
                filename: "jei-3-disabled.jar".to_string(),
            },
            // Only one enabled file: not a duplicate.
            DuplicateCandidate {
                modid: Some("solo".to_string()),
                enabled: true,
                addon_type: AddonType::Mods,
                filename: "solo.jar".to_string(),
            },
            // No modid at all: never grouped.
            DuplicateCandidate {
                modid: None,
                enabled: true,
                addon_type: AddonType::Mods,
                filename: "no-modid.jar".to_string(),
            },
        ];

        let mut files = HashMap::new();
        files.insert(
            "/mods/jei-1.jar".to_string(),
            FileHashes {
                sha512: [1; 64],
                md5: [1; 16],
            },
        );
        let packinfo = PackInfo { files };

        let groups = group_duplicates(&candidates, Some(&packinfo), &instance_path);

        assert_eq!(
            groups.len(),
            1,
            "only jei has more than one ENABLED file sharing a modid, got {groups:?}"
        );
        let group = &groups[0];
        assert_eq!(group.modid, "jei");
        assert_eq!(
            group.files.len(),
            2,
            "the disabled jei-3 file must not count toward the group"
        );
        assert_eq!(group.files[0].path, "/mods/jei-1.jar");
        assert!(group.files[0].pack_owned);
        assert_eq!(group.files[1].path, "/mods/jei-2.jar");
        assert!(!group.files[1].pack_owned);
    }

    #[test]
    fn group_duplicates_with_no_packinfo_marks_nothing_pack_owned() {
        let instance_path = InstancePath::new(PathBuf::from("/fake-instance"));
        let candidates = vec![
            DuplicateCandidate {
                modid: Some("jei".to_string()),
                enabled: true,
                addon_type: AddonType::Mods,
                filename: "jei-1.jar".to_string(),
            },
            DuplicateCandidate {
                modid: Some("jei".to_string()),
                enabled: true,
                addon_type: AddonType::Mods,
                filename: "jei-2.jar".to_string(),
            },
        ];

        let groups = group_duplicates(&candidates, None, &instance_path);

        assert_eq!(groups.len(), 1);
        assert!(groups[0].files.iter().all(|f| !f.pack_owned));
    }

    // --- repair_preview (integration) --------------------------------------
    //
    // Exercises the whole manager fn against a real (temp-dir-backed) App,
    // unlike the pure-function tests above: proves packinfo load, disk_scan,
    // apply_plan, and the untracked walk actually wire together, and — the
    // binding requirement on this task — that the preview is read-only: it
    // must detect corruption without ever executing a plan entry against
    // the real file.
    //
    // A new instance's first install is still pending in both fixtures. It
    // never touched installed files, so Repair would replace it with exactly
    // the plan shown: the preview is withheld only while a pending setup may
    // have changed files (see `PendingSetupStatus::preview_withheld`).

    #[tokio::test]
    async fn repair_preview_detects_corruption_without_touching_the_file() {
        use crate::domain::instance::info;
        use crate::managers::instance::InstanceVersionSource;
        use md5::{Digest, Md5};

        let app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await.unwrap();
        let instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("test"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: Default::default(),
                    },
                )),
                String::new(),
            )
            .await
            .unwrap();

        let instance_path = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path("test");

        let correct_md5: [u8; 16] = Md5::digest(b"correct-pack-bytes").into();
        let mut files = HashMap::new();
        files.insert(
            "/mods/a.jar".to_string(),
            FileHashes {
                sha512: [0; 64],
                md5: correct_md5,
            },
        );
        let recorded = PackInfo { files };

        tokio::fs::write(
            instance_path.get_root().join("packinfo.json"),
            packinfo::make_packinfo(recorded).unwrap(),
        )
        .await
        .unwrap();

        let mods_dir = instance_path.get_data_path().join("mods");
        tokio::fs::create_dir_all(&mods_dir).await.unwrap();
        let jar_path = mods_dir.join("a.jar");
        tokio::fs::write(&jar_path, b"corrupted-on-disk-bytes")
            .await
            .unwrap();

        let status = app
            .instance_manager()
            .pending_setup(instance_id)
            .await
            .unwrap();
        assert!(status.pending && !status.preview_withheld);

        let preview = app
            .instance_manager()
            .repair_preview(instance_id)
            .await
            .unwrap();

        assert!(preview.has_packinfo);
        assert_eq!(preview.without_re_enable.counts.restore_modified, 1);
        let entry = preview
            .without_re_enable
            .entries
            .iter()
            .find(|e| e.path == "/mods/a.jar")
            .expect("packinfo-tracked path must have a plan entry");
        assert_eq!(entry.action, PlanAction::Replace);

        // A present-but-modified file's plan entry never depends on
        // `re_enable_disabled` (that flag only governs the `Disabled` arm) —
        // both variants must agree over the same disk scan.
        assert_eq!(
            preview.with_re_enable.counts, preview.without_re_enable.counts,
            "a modified-but-present file's outcome must not depend on re_enable_disabled"
        );

        // Read-only: the corrupted file on disk must be untouched by the
        // preview — repair_preview must never execute any plan entry.
        let bytes_after = tokio::fs::read(&jar_path).await.unwrap();
        assert_eq!(bytes_after, b"corrupted-on-disk-bytes");
    }

    #[tokio::test]
    async fn repair_preview_with_no_packinfo_reports_has_packinfo_false() {
        use crate::domain::instance::info;
        use crate::managers::instance::InstanceVersionSource;

        let app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await.unwrap();
        let instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("test-no-packinfo"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: Default::default(),
                    },
                )),
                String::new(),
            )
            .await
            .unwrap();

        let status = app
            .instance_manager()
            .pending_setup(instance_id)
            .await
            .unwrap();
        assert!(status.pending && !status.preview_withheld);

        let preview = app
            .instance_manager()
            .repair_preview(instance_id)
            .await
            .unwrap();

        assert!(!preview.has_packinfo);
        assert!(preview.with_re_enable.entries.is_empty());
        assert_eq!(preview.with_re_enable.counts, RepairCounts::default());
        assert!(preview.without_re_enable.entries.is_empty());
        assert_eq!(preview.without_re_enable.counts, RepairCounts::default());
        assert!(preview.untracked.is_empty());
        assert!(preview.duplicates.is_empty());
    }
    #[tokio::test]
    async fn task03_preview_rejects_origin_for_edited_content() {
        use super::{OriginVerdict, origin_check::OriginResults};
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("config")).unwrap();
        let path = tmp.path().join("config/retired.cfg");
        std::fs::write(&path, b"historical").unwrap();
        let record = PackInfo {
            files: [(
                "/config/current.cfg".into(),
                FileHashes {
                    md5: [0; 16],
                    sha512: [0; 64],
                },
            )]
            .into_iter()
            .collect(),
        };
        let context = super::PackProofContext {
            platform: "modrinth".into(),
            project_id: "pack".into(),
            version_id: "current".into(),
        };
        let results = OriginResults {
            context: Some(context.clone()),
            checked_hashes: [(
                "/config/retired.cfg".into(),
                super::origin_check::hash_file_sha512(&path).await.unwrap(),
            )]
            .into_iter()
            .collect(),
            verdicts: [(
                "/config/retired.cfg".into(),
                OriginVerdict::ShippedIn {
                    version_id: "older".into(),
                    version_name: "Older".into(),
                },
            )]
            .into_iter()
            .collect(),
        };
        let before =
            untracked_files_for_preview(tmp.path(), &record, Some(&results), Some(&context), false)
                .await
                .unwrap();
        assert!(matches!(
            before[0].origin,
            Some(OriginVerdict::ShippedIn { .. })
        ));
        std::fs::write(&path, b"user edits").unwrap();
        let after =
            untracked_files_for_preview(tmp.path(), &record, Some(&results), Some(&context), false)
                .await
                .unwrap();
        assert_eq!(after[0].origin, None);
    }
    #[test]
    fn task03_selection_admission_and_persisted_marker_bind_content_context() {
        use super::{
            CleanupSelection, OriginVerdict, PackProofContext, origin_check::OriginResults,
            validate_cleanup_selections,
        };
        let context = PackProofContext {
            platform: "modrinth".into(),
            project_id: "pack".into(),
            version_id: "installed".into(),
        };
        let mut selection = CleanupSelection {
            path: "/config/retired.cfg".into(),
            sha512: hex::encode([1u8; 64]),
            context: context.clone(),
            proven: true,
        };
        let cache = OriginResults {
            context: Some(context.clone()),
            checked_hashes: [(selection.path.clone(), [1u8; 64])].into_iter().collect(),
            verdicts: [(
                selection.path.clone(),
                OriginVerdict::ShippedIn {
                    version_name: "Old".into(),
                    version_id: "old".into(),
                },
            )]
            .into_iter()
            .collect(),
        };
        assert!(validate_cleanup_selections(&[selection.clone()], &context, Some(&cache)).is_ok());
        assert!(validate_cleanup_selections(&[selection.clone()], &context, None).is_err());
        selection.sha512 = hex::encode([2u8; 64]);
        assert!(validate_cleanup_selections(&[selection.clone()], &context, Some(&cache)).is_err());
        selection.proven = false;
        assert!(validate_cleanup_selections(&[selection.clone()], &context, None).is_ok());
        let marker = RepairMarkerFile {
            cleanup_selections: vec![selection.clone()],
            cleanup_batch: Some("cleanup-owned-storage".into()),
            ..Default::default()
        };
        assert_eq!(
            serde_json::from_str::<RepairMarkerFile>(&serde_json::to_string(&marker).unwrap())
                .unwrap(),
            marker
        );
        selection.context.version_id = "different".into();
        assert!(validate_cleanup_selections(&[selection.clone()], &context, None).is_err());
        selection.context = context.clone();
        selection.sha512 = "malformed".into();
        assert!(validate_cleanup_selections(&[selection], &context, None).is_err());
    }
}
