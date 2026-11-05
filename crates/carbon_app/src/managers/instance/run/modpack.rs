use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::info::{
    self, Instance, JavaOverride, Modpack, ModpackInfo, StandardVersion,
};
use crate::domain::instance::{self as domain, GameLogId, InstanceId};
use crate::domain::java::{JavaComponent, JavaComponentType, SystemJavaProfileName};
use crate::domain::metrics::GDLMetricsEvent;
use crate::domain::vtask::VisualTaskId;
use crate::managers::AppInner;
use crate::managers::instance::log::{
    GameLog, LogEntry, LogEntrySourceKind, format_message_as_log4j_event,
};
use crate::managers::instance::modpack::session::{self, Kind, Owner, Phase, Producer, Session};
use crate::managers::instance::modpack::{
    PackVersionFile, RepairMarkerFile, apply_plan, disk_scan, normalize_cleanup_path, packinfo,
    walk_untracked_files,
};
use crate::managers::java::java_checker::{JavaChecker, RealJavaChecker};
use crate::managers::java::managed::Step;
use crate::managers::minecraft::assets::get_assets_dir;
use crate::managers::minecraft::minecraft::get_lwjgl_meta;
use crate::managers::minecraft::modrinth;
use crate::managers::minecraft::{UpdateValue, curseforge, gdlpack};
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::managers::modplatforms::modrinth::convert_mr_version_to_standard_version;
use crate::managers::vtask::Subtask;
use crate::util::{NormalizedWalkdir, publish};
use crate::{
    domain::instance::info::{GameVersion, ModLoader, ModLoaderType},
    managers::{
        self, ManagerRef,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
    },
};
use anyhow::{Context, anyhow, bail};
use carbon_net::{DownloadOptions, Downloadable};
use carbon_parsing::log::{LogParser, ParsedItem};
use carbon_platforms::curseforge::filters::ModFileParameters;
use carbon_platforms::modrinth::search::VersionID;
use carbon_rt_path::InstancePath;
use chrono::{DateTime, Local, Utc};
use futures::Future;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore, watch};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tracing::{info, trace};

pub type TSubtasks = Arc<TSubtasksInner>;

pub struct TSubtasksInner {
    pub t_request_version_info: Subtask,
    pub t_download_files: Subtask,
    pub t_scan_files: Subtask,
    pub t_generating_packinfo: Subtask,
    pub t_request_modloader_info: Subtask,
    pub t_request_minecraft_files: Subtask,
    pub t_download_java: Subtask,
    pub t_extract_java: Subtask,
    pub t_apply_staging: Subtask,
    pub t_fill_cache: Subtask,
    pub t_extract_natives: Subtask,
    pub t_reconstruct_assets: Subtask,
    pub t_forge_processors: Option<Subtask>,
    pub t_neoforge_processors: Option<Subtask>,
    pub t_finalize_import: Option<Subtask>,
}

/// Whether `tmp-packinfo.json` (the full post-apply record) must be
/// (re)derived by walking `.setup/staging`, or is already the authoritative
/// target record left by an earlier, possibly-interrupted pass over this
/// exact change-pack-version session and must be preserved untouched.
///
/// `apply_started` (`.setup/apply-started`, written by
/// [`process_modpack_staging`] right before it calls [`execute_plan`], the
/// only thing that ever destructively consumes `.setup/staging`) is the
/// discriminator — deliberately *not* `skip_overrides`, which flips true as
/// soon as override extraction finishes, well before the mod-download phase
/// and the `tmp-packinfo.json` write even run. A crash in that gap leaves
/// `skip_overrides` true while `.setup/staging` is still completely
/// untouched; keying regeneration on it alone would suppress regeneration
/// for a record that was never even written, silently discarding the
/// staged download and completing the launch with nothing applied. Whether
/// `execute_plan` has actually *started* is the only thing that tells us
/// re-walking staging might no longer be safe:
///
/// - `apply_started` false: staging is guaranteed unconsumed (execute_plan
///   cannot run without a `tmp-packinfo.json` to plan against, and nothing
///   else touches staging), so re-deriving `tmp-packinfo.json` from it is
///   always safe — this covers a first pass, a crash before extraction
///   finished, and a crash anywhere between extraction finishing and
///   `execute_plan` actually starting (including a crash mid-write of
///   `tmp-packinfo.json` itself).
/// - `apply_started` true: `.setup/staging` may be partially or fully
///   consumed by whatever `execute_plan` already applied before a crash.
///   Re-deriving `tmp-packinfo.json` from it now would silently drop every
///   already-applied path from the very record meant to describe the target
///   state (the resumed run then treats those paths as dropped by the pack
///   and can delete them). Leaving `tmp-packinfo.json` untouched is correct
///   whether it currently exists (the apply is still in flight; this
///   preserves the complete pre-apply record for
///   [`process_modpack_staging`] to resume against) or not (the apply
///   already finished and promoted it to `packinfo.json`;
///   `process_modpack_staging` recognises that on its own, via the same
///   `apply_started` marker, and skips straight to finishing cleanup — see
///   its doc comment for the other half of this invariant).
fn tmp_packinfo_must_be_regenerated(file_provided: bool, apply_started: bool) -> bool {
    file_provided && !apply_started
}

/// Whether [`process_modpack_staging`] finding `tmp-packinfo.json` missing
/// proves the apply already fully completed and got promoted to
/// `packinfo.json`, rather than simply never having started.
///
/// `packinfo_exists` alone is not enough: a *version change* on an
/// already-installed modpack has an old `packinfo.json` on disk from before
/// the change even began, so its mere presence proves nothing about whether
/// *this* apply ran. `apply_started` (`.setup/apply-started`) is what
/// proves it — written only right before [`execute_plan`] is first called
/// for the record currently being applied, so its presence is proof
/// `execute_plan` actually ran for that record; combined with `tmp` being
/// gone (checked by the caller before this is consulted), the only way
/// `execute_plan` can have run *and* `tmp` be gone is a successful promote.
/// Requiring both `apply_started` and `packinfo_exists` — rather than
/// either alone — is deliberate: `apply_started` alone can't rule out a
/// crash between promoting (which needs `packinfo_exists`) and finishing
/// (this function isn't reached once `packinfo_exists` when `apply_started`
/// is false, since `tmp_packinfo_must_be_regenerated` would have already
/// regenerated `tmp` in that case, before this function ever runs). Only
/// `apply_started && packinfo_exists` is unambiguous proof of a completed,
/// promoted apply; any other combination (most importantly `apply_started`
/// false, whether or not an old `packinfo_exists`) must never be read as
/// "already applied" — that would silently mark a change complete having
/// applied nothing at all.
fn staging_apply_already_promoted(apply_started: bool, packinfo_exists: bool) -> bool {
    apply_started && packinfo_exists
}

/// This function prepares the modpack in a staging directory in the instance folder.
/// The original instane data is not modified.
///
/// It downloads and processes all required files STRICTLY belonging to the modpack.
///
/// When it's done, it creates the staging-packinfo.json and packinfo.json file and marks the modpack as installed through
/// the modpack-complete disk flag.
#[cfg(test)]
pub async fn process_modpack(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    deep_check: bool,
    config: Instance,
    instance_shortpath: String,
    task: &VisualTask,
    has_callback_task: bool,
    session_owner: &mut Option<Owner>,
) -> anyhow::Result<(
    TSubtasks,
    Option<StandardVersion>,
    Option<RepairMarkerFile>,
    Option<disk_scan::DiskScan>,
)> {
    process_modpack_cancellable(
        app,
        instance_id,
        deep_check,
        config,
        instance_shortpath,
        task,
        has_callback_task,
        session_owner,
        &tokio_util::sync::CancellationToken::new(),
    )
    .await
}

pub async fn process_modpack_cancellable(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    deep_check: bool,
    config: Instance,
    instance_shortpath: String,
    task: &VisualTask,
    has_callback_task: bool,
    session_owner: &mut Option<Owner>,
    cancel: &tokio_util::sync::CancellationToken,
) -> anyhow::Result<(
    TSubtasks,
    Option<StandardVersion>,
    Option<RepairMarkerFile>,
    Option<disk_scan::DiskScan>,
)> {
    // An attempt's preparation folder holds nothing worth keeping until its
    // downloads start (see `reuse_earlier_downloads`): only overrides, which
    // every attempt extracts again. Removing it when the attempt fails
    // before then keeps repeated failures, a full disk among them, from
    // piling up copies.
    let mut unclaimed = None;
    let result = prepare_modpack(
        app,
        instance_id,
        deep_check,
        config,
        instance_shortpath,
        task,
        has_callback_task,
        session_owner,
        cancel,
        &mut unclaimed,
    )
    .await;
    if let (Err(_), Some(folder)) = (&result, unclaimed) {
        match tokio::fs::remove_dir_all(&folder).await {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(?error, path = %folder.display(), "failed preparation kept");
            }
        }
    }
    result
}

async fn prepare_modpack(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    deep_check: bool,
    mut config: Instance,
    instance_shortpath: String,
    task: &VisualTask,
    has_callback_task: bool,
    session_owner: &mut Option<Owner>,
    cancel: &tokio_util::sync::CancellationToken,
    unclaimed: &mut Option<PathBuf>,
) -> anyhow::Result<(
    TSubtasks,
    Option<StandardVersion>,
    Option<RepairMarkerFile>,
    Option<disk_scan::DiskScan>,
)> {
    anyhow::ensure!(!cancel.is_cancelled(), "Modpack preparation cancelled");
    let installed_snapshot = config.clone();
    let mut version: Option<StandardVersion> = None;

    let runtime_path = app.settings_manager().runtime_path.clone();
    let instance_path = runtime_path
        .get_instances()
        .get_instance_path(&instance_shortpath);

    let instance_root = instance_path.get_root();
    let setup_path = instance_root.join(".setup");
    let is_setup = setup_path.is_dir();
    let is_modpack_complete = setup_path.join("modpack-complete").exists();

    let mut setup_session = Session::load(&instance_root).await?;
    // A record that is not this instance's own (written before it had an id,
    // copied, or kept from an earlier registration of the folder; see
    // `Session::binds_to`) from a non-pack producer has explicit kind
    // provenance. It needs no inferred content target and never runs pack
    // apply. A pack request (or repair) copied before its archive arrived is
    // the same: nothing was staged, so the instance downloads the pack for
    // itself.
    if let Some(saved) = setup_session.as_ref() {
        let qualification = saved
            .copy_qualification(&instance_root, instance_id)
            .await?;
        if qualification == Some(false) {
            // Apply evidence beside it: it may have changed files.
            bail!(
                "{}",
                super::super::mods::uncontinued_setup_refusal(
                    &config,
                    super::super::mods::PendingChange::Play,
                    saved.copied_to == Some(instance_id.0),
                )
            );
        }
        if qualification == Some(true) {
            setup_session = Some(saved.qualify_copy(&instance_root, instance_id).await?);
        }
    }
    let copied_session = setup_session
        .as_ref()
        .filter(|session| session.binds_to(instance_id))
        .cloned();
    if copied_session.is_some() {
        setup_session = None;
    }
    if let Some(session) = &setup_session {
        let owner = session.owner()?;
        if owner.instance_id != instance_id.0 {
            bail!(
                "foreign setup remains preserved; reconstruct its original inputs before adoption"
            );
        }
        if let Some(expected) = session_owner.as_ref() {
            session.check_owner(expected)?;
        }
        *session_owner = Some(owner);
        if let Some(pending) = &session.pending_config {
            if let Some(GameVersion::Standard(resolved)) =
                super::super::schema::parse_instance_config(pending)?
                    .game_configuration
                    .version
            {
                version = Some(resolved);
            }
        }
    }
    let reconstruct = is_setup && setup_session.is_none();
    // Unknown markerless state never reuses residual staging. Only original
    // archives can qualify it, including older consumed-disabled overrides.
    // Preparation has already adopted every untouched 2.0.40 shape under the
    // instance gate (Session::adopt_legacy); what reaches here has no record.
    if reconstruct && !session::has_original_archive(&setup_path) {
        if session::complete_without_staging(&instance_root)? {
            let requested_pack = match tokio::fs::read_to_string(
                setup_path.join("change-pack-version.json"),
            )
            .await
            {
                Ok(text) => Some(text),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            let adopted = Session::legacy_complete(&config, instance_id, requested_pack)?;
            adopted.create(&instance_root).await?;
            *session_owner = Some(adopted.owner()?);
            setup_session = Some(adopted);
        } else if let Some(copied) = &copied_session {
            bail!(
                "{}",
                super::super::mods::uncontinued_setup_refusal(
                    &config,
                    super::super::mods::PendingChange::Play,
                    copied.copied_to == Some(instance_id.0),
                )
            );
        } else if config.modpack.is_some() {
            // Play never resumes a setup that may have applied part of a
            // pack: regenerating its target from consumed staging can delete
            // files. Repair replaces it from a fresh archive instead.
            bail!(
                "This modpack's last install or update was interrupted part-way through applying its files, so it can't be resumed safely. Use Repair on this instance to reinstall the modpack; your worlds are kept."
            );
        } else {
            bail!(
                "This instance's last install was interrupted part-way through applying files, so it can't be resumed safely. To install it again, delete the hidden .setup folder inside the instance folder, then press Play."
            );
        }
    }
    let transition_sources = reconstruction_source_requirements(
        &instance_root,
        setup_session.as_ref(),
        copied_session.as_ref(),
    )
    .await?;
    // A previously saved reconstructed session may predate acquisition of
    // originals needed by an unadopted transition. Resume that qualification;
    // never demote an Applying/Applied operation to pre-apply authority.
    let needs_originals = setup_session.as_ref().is_some_and(|session| {
        session.producer == Producer::Reconstructed
            && session.phase == Phase::Resolved
            && session.target.as_ref().is_some_and(|target| {
                transition_sources
                    .iter()
                    .any(|path| !target.sources.contains_key(path))
            })
    });
    let resolved = setup_session.as_ref().is_some_and(|session| {
        session.kind != Kind::Pack || session.target.is_some() && !needs_originals
    });
    let staging_dir = if !resolved && is_setup {
        setup_path.join(format!("preparing-{}", uuid::Uuid::new_v4()))
    } else {
        setup_path.join("staging")
    };

    let packinfo_path = instance_root.join("packinfo.json");
    let tmp_packinfo_path = instance_root.join("tmp-packinfo.json");

    // Absent marker (including a `.setup` left behind by an older build that
    // never wrote one) means an ordinary version change.
    let repair_marker_path = setup_path.join("repair");
    let mut repair_options: Option<RepairMarkerFile> =
        match tokio::fs::read_to_string(&repair_marker_path).await {
            Ok(s) => Some(serde_json::from_str(&s).context("while parsing repair marker json")?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };

    let requested_pack =
        match tokio::fs::read_to_string(setup_path.join("change-pack-version.json")).await {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
    if let Some(session) = &setup_session {
        if session.original_options != repair_options {
            bail!("repair options changed after setup admission");
        }
        if session.requested_pack != requested_pack {
            bail!("requested pack changed after setup admission");
        }
        repair_options = session.original_options.clone();
    }

    if let Some(copied) = &copied_session {
        if copied.original_options != repair_options || copied.requested_pack != requested_pack {
            bail!("copied setup options/target differ from their original admission");
        }
    }
    // Named for what it's used for below (skip-optimisation oracle for the
    // platform prep fns), not merely what it holds. A repair judges "needs
    // download" against what is actually on disk right now, not against the
    // record — a corrupt or missing file must be re-fetched even when the
    // record says it was fine.
    //
    // The download oracle avoids fetching already-verified bytes. Staging
    // always performs a fresh bounded old/target scan before planning;
    // the earlier oracle never authorizes a later live-file mutation.
    let mut oracle_diagnostics = Vec::new();
    let (skip_oracle, oracle_disk_scan) = match &repair_options {
        Some(_) => {
            let (packinfo, disk_scan, diagnostics) =
                disk_scan::scan_repair_oracle(&instance_path.get_data_path()).await?;
            oracle_diagnostics = diagnostics;
            (Some(packinfo), Some(disk_scan))
        }
        None => (
            match tokio::fs::read_to_string(&packinfo_path).await {
                Ok(text) => {
                    Some(packinfo::parse_packinfo(&text).context("while parsing packinfo json")?)
                }
                Err(_) => None,
            },
            None,
        ),
    };

    // Only transition members need independent pristine acquisition. Keep
    // all unrelated descriptor skips and the complete original oracle intact.
    let preparation_oracle = skip_oracle.as_ref().map(|oracle| {
        let mut oracle = oracle.clone();
        for path in &transition_sources {
            oracle.files.remove(path);
        }
        oracle
    });
    let t_modpack =
        match is_setup && (!is_modpack_complete || reconstruct || needs_originals) && !resolved {
            true => Some((
                task.subtask(Translation::InstanceTaskLaunchRequestModpack),
                task.subtask(Translation::InstanceTaskLaunchDownloadModpack),
                task.subtask(Translation::InstanceTaskLaunchDownloadModpackFiles),
                task.subtask(Translation::InstanceTaskLaunchExtractModpackFiles),
                task.subtask(Translation::InstanceTaskLaunchRequestAddonMetadata),
            )),
            false => None,
        };

    let t_apply_staging = task.subtask(Translation::InstanceTaskLaunchApplyStagedPatches);

    let t_request_version_info = task.subtask(Translation::InstanceTaskLaunchRequestVersions);

    let t_download_files = task.subtask(Translation::InstanceTaskLaunchDownloadFiles);
    t_download_files.set_weight(20.0);

    let t_scan_files = task.subtask(Translation::InstanceTaskLaunchCheckingFiles);
    t_scan_files.set_weight(5.0);

    let t_generating_packinfo = task.subtask(Translation::InstanceTaskGeneratingPackInfo);

    let t_request_modloader_info = task.subtask(Translation::InstanceTaskRequestModloaderInfo);

    let t_request_minecraft_files = task.subtask(Translation::InstanceTaskRequestMinecraftFiles);

    let t_download_java = task.subtask(Translation::InstanceTaskLaunchDownloadJava);

    let t_extract_java = task.subtask(Translation::InstanceTaskLaunchExtractJava);

    let t_fill_cache = task.subtask(Translation::InstanceTaskFillCache);

    let t_extract_natives = task.subtask(Translation::InstanceTaskLaunchExtractNatives);

    let t_reconstruct_assets = task.subtask(Translation::InstanceTaskReconstructAssets);

    // Fresh modpack installs may not have the version in config until
    // modpack processing resolves it, so is_setup must keep creating the
    // subtasks unconditionally; loader-keyed creation covers every launch
    // after that so regeneration has a progress subtask to report through.
    let has_loader = |wanted: ModLoaderType| {
        matches!(
            &config.game_configuration.version,
            Some(GameVersion::Standard(v))
                if v.modloaders.iter().any(|m| m.type_ == wanted)
        )
    };

    let t_forge_processors = match is_setup || has_loader(ModLoaderType::Forge) {
        true => Some(task.subtask(Translation::InstanceTaskLaunchRunForgeProcessors)),
        false => None,
    };

    let t_neoforge_processors = match is_setup || has_loader(ModLoaderType::Neoforge) {
        true => Some(task.subtask(Translation::InstanceTaskLaunchRunNeoforgeProcessors)),
        false => None,
    };

    let t_finalize_import = if has_callback_task {
        Some(task.subtask(Translation::FinalizingImport))
    } else {
        None
    };

    task.edit(|data| data.state = TaskState::KnownProgress)
        .await;

    let change_version_path = setup_path.join("change-pack-version.json");

    if let Some((
        t_request,
        t_download_packfile,
        t_download_modpack_files,
        t_extract_files,
        t_addon_metadata,
    )) = t_modpack
    {
        let mut modpack_downloads = Vec::new();
        if ["curseforge", "modrinth", "gdlpack"]
            .iter()
            .filter(|name| setup_path.join(name).is_file())
            .count()
            > 1
        {
            bail!(
                "multiple original pack archives make setup provenance ambiguous; all inputs retained"
            );
        }

        let cffile_path = setup_path.join("curseforge");
        let mrfile_path = setup_path.join("modrinth");
        let gdlpack_path = setup_path.join("gdlpack");

        // Is this required? Can we not extract them twice? Extraction should be idempotent.
        // TODO: look into this
        let skip_overrides_path = setup_path.join("modpack-skip-overrides");
        // Private pristine reconstruction, never residual staging: every
        // attempt extracts the overrides afresh. Only downloads an earlier
        // attempt finished are reused, by hash (`reuse_earlier_downloads`).
        let skip_overrides = false;

        // Whether `process_modpack_staging` has ever started consuming
        // `.setup/staging` for the record currently on disk — written right
        // before it calls `execute_plan` (see there) and never removed
        // except by `.setup` itself being wiped (full completion, or a fresh
        // `change_modpack`/`repair_modpack`). This is deliberately a
        // *different* signal from `skip_overrides`: `skip_overrides` flips
        // true as soon as extraction finishes, well before the mod-download
        // phase and the `tmp-packinfo.json` write below even run, so a crash
        // in that gap would leave `skip_overrides` true while staging is
        // still completely untouched — keying the regeneration decision on
        // it alone would suppress regeneration for a record that was never
        // even written, losing the staged download outright. Whether
        // `execute_plan` has actually started is the only thing that tells
        // us staging might no longer be safe to re-walk.
        let apply_started_path = setup_path.join("apply-started");
        let apply_started = false; // full target comes from original inputs below

        let modpack = match tokio::fs::read_to_string(&change_version_path).await {
            Ok(text) => Some(Modpack::from(serde_json::from_str::<PackVersionFile>(
                &text,
            )?)),
            Err(_) => None,
        };

        enum Modplatform {
            Curseforge,
            Modrinth,
            GDLPack,
        }

        t_request.start_opaque();
        #[cfg(test)]
        app.instance_manager
            .test_hooks
            .pause("setup_pack_request", instance_id)
            .await;

        // If a cf, mr, or gdlpack file is provided, we don't need to do anything.
        // In case a modpack (from a change-pack-version.json file) is provided,
        // we need to download the modpack zip file.
        let file = match (
            cffile_path.is_file(),
            mrfile_path.is_file(),
            gdlpack_path.is_file(),
            &modpack,
        ) {
            (false, false, false, None) => {
                t_request.complete_opaque();
                None
            }
            (true, _, _, _) => {
                t_request.complete_opaque();
                Some(Modplatform::Curseforge)
            }
            (_, true, _, _) => {
                t_request.complete_opaque();
                Some(Modplatform::Modrinth)
            }
            (_, _, true, _) => {
                t_request.complete_opaque();
                Some(Modplatform::GDLPack)
            }
            (false, false, false, Some(Modpack::Curseforge(modpack))) => {
                // A lookup: a quit drops it rather than wait on a stalled network.
                let file = crate::managers::runtime_lifecycle::lookup_or_cancel(
                    cancel,
                    app.modplatforms_manager()
                        .curseforge
                        .get_mod_file(ModFileParameters {
                            file_id: modpack.file_id as i32,
                            mod_id: modpack.project_id as i32,
                        }),
                )
                .await?
                .data;

                t_request.complete_opaque();

                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(UpdateValue::<(u64, u64)>::new((0, 0)));

                t_download_packfile.start_opaque();
                let completion = tokio::spawn(async move {
                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let (downloaded, total) = modpack_progress_rx.borrow().0;
                            t_download_packfile.update_download(
                                downloaded as u32,
                                total as u32,
                                true,
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_download_packfile.complete_opaque();
                });

                let result = curseforge::download_modpack_zip(
                    &app,
                    &file,
                    &cffile_path,
                    modpack_progress_tx,
                    &cancel,
                )
                .await;
                completion.await?;
                result?;

                Some(Modplatform::Curseforge)
            }
            (false, false, false, Some(Modpack::Modrinth(modpack))) => {
                let file = crate::managers::runtime_lifecycle::lookup_or_cancel(
                    cancel,
                    app.modplatforms_manager()
                        .modrinth
                        .get_version(VersionID(modpack.version_id.clone())),
                )
                .await?
                .files
                .into_iter()
                .reduce(|a, b| if b.primary { b } else { a })
                .ok_or_else(|| {
                    anyhow!(
                        "Modrinth project '{}' version '{}' does not have a file",
                        modpack.project_id,
                        modpack.version_id
                    )
                })?;

                t_request.complete_opaque();

                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(UpdateValue::<(u64, u64)>::new((0, 0)));

                t_download_packfile.start_opaque();

                let completion = tokio::spawn(async move {
                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let (downloaded, total) = modpack_progress_rx.borrow().0;
                            t_download_packfile.update_download(
                                downloaded as u32,
                                total as u32,
                                true,
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_download_packfile.complete_opaque();
                });

                let result = modrinth::download_mrpack(
                    &app,
                    &file,
                    &mrfile_path,
                    modpack_progress_tx,
                    &cancel,
                )
                .await;
                completion.await?;
                result?;

                Some(Modplatform::Modrinth)
            }
        };

        anyhow::ensure!(
            !cancel.is_cancelled(),
            "Modpack preparation cancelled after archive download settled"
        );
        let archive_name = match file {
            Some(Modplatform::Curseforge) => Some("curseforge"),
            Some(Modplatform::Modrinth) => Some("modrinth"),
            Some(Modplatform::GDLPack) => Some("gdlpack"),
            None => None,
        };
        // Taken before the proof's hash, so a write during it shows too.
        let archive_stamp = match archive_name {
            Some(name) => file_stamp(&setup_path.join(name)).await,
            None => None,
        };
        let archive_proof = match archive_name {
            Some(name) => Some(session::Archive {
                name: name.into(),
                sha512: hex::encode(
                    crate::managers::runtime_lifecycle::run_owned_phase(cancel, || async {
                        crate::managers::instance::modpack::origin_check::hash_file_sha512(
                            &setup_path.join(name),
                        )
                        .await
                    })
                    .await?,
                ),
            }),
            None => bail!(
                "pack setup has no original archive; refusing to infer a version-only install"
            ),
        };

        // Temporarily create a staging directory and download the modpack files there
        *unclaimed = Some(staging_dir.clone());
        crate::managers::runtime_lifecycle::run_owned_phase(cancel, || async {
            tokio::fs::create_dir_all(&staging_dir.join("instance")).await?;
            Ok(())
        })
        .await?;
        let instance_prep_path = InstancePath::new(staging_dir.clone());

        let mut skipped_mods = Vec::new();

        // Prepaers the list of modpack downloadable files and the manifest, as
        // well as extract the overrides in it
        let v: Option<StandardVersion> = match file {
            Some(Modplatform::Curseforge) => {
                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(curseforge::ProgressState::new());

                t_addon_metadata.start_opaque();

                let completion = tokio::spawn(async move {
                    let mut tracker = curseforge::ProgressState::new();

                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let progress = modpack_progress_rx.borrow();

                            tracker.extract_addon_overrides.update_from(
                                &progress.extract_addon_overrides,
                                |(completed, total)| {
                                    t_extract_files.update_items(completed as u32, total as u32);
                                },
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_extract_files.complete_opaque();
                });

                let modpack_info = curseforge::prepare_modpack_from_zip_cancellable(
                    &app,
                    &cffile_path,
                    &instance_prep_path,
                    skip_overrides,
                    preparation_oracle.as_ref(),
                    t_addon_metadata,
                    modpack_progress_tx,
                    cancel,
                )
                .await
                .map_err(|e| {
                    tracing::error!("Error preparing modpack: {:?}", e);
                    e
                });

                completion.await?;
                let modpack_info = modpack_info?;

                tokio::fs::create_dir_all(skip_overrides_path).await?;

                for (downloadable, skip) in modpack_info.downloadables {
                    match skip {
                        Some(skippath) => skipped_mods.push(skippath),
                        None => modpack_downloads.push(downloadable),
                    }
                }

                let curseforge_version = modpack_info.manifest.minecraft;

                let dummy_string = daedalus::BRANDING
                    .get_or_init(daedalus::Branding::default)
                    .dummy_replace_string
                    .clone();

                let gdl_version = convert_cf_version_to_standard_version(
                    app.clone(),
                    curseforge_version,
                    dummy_string,
                )
                .await?;

                Some(gdl_version)
            }
            Some(Modplatform::Modrinth) => {
                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(modrinth::ProgressState::Idle);

                let completion = tokio::spawn(async move {
                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let progress = modpack_progress_rx.borrow();
                            match *progress {
                                modrinth::ProgressState::Idle => {}
                                modrinth::ProgressState::ExtractingPackOverrides(count, total) => {
                                    t_extract_files.update_items(count as u32, total as u32)
                                }
                                modrinth::ProgressState::AcquiringPackMetadata(count, total) => {
                                    t_addon_metadata.update_items(count as u32, total as u32)
                                }
                            }
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_addon_metadata.complete_opaque();
                    t_extract_files.complete_opaque();
                });

                let modpack_info = modrinth::prepare_modpack_from_mrpack_cancellable(
                    &app,
                    &mrfile_path,
                    &instance_prep_path,
                    skip_overrides,
                    preparation_oracle.as_ref(),
                    modpack_progress_tx,
                    cancel,
                )
                .await;

                completion.await?;
                let modpack_info = modpack_info?;

                tokio::fs::create_dir_all(skip_overrides_path).await?;

                for (downloadable, skip) in modpack_info.downloadables {
                    match skip {
                        Some(skippath) => skipped_mods.push(skippath),
                        None => modpack_downloads.push(downloadable),
                    }
                }

                let modrinth_version = modpack_info.index.dependencies;

                let gdl_version =
                    convert_mr_version_to_standard_version(app.clone(), modrinth_version).await?;

                Some(gdl_version)
            }
            Some(Modplatform::GDLPack) => {
                // gdlpack's own skip predicate (`gdlpack.rs`'s
                // `existing_packinfo`/`skip_path` lookup) only checks
                // whether a path is PRESENT in the oracle, never whether its
                // hash matches — unlike the curseforge/modrinth prep fns.
                // Under an ordinary version change that is merely a missed
                // optimisation (the record is trusted anyway); under repair
                // the oracle is a live disk scan, so a merely-*existing*
                // damaged file would be skip-optimised as-is and its
                // corrupt hash promoted into packinfo as canonical —
                // laundering the corruption instead of fixing it. No
                // `Modpack` variant can select GDLPack today (making this
                // unreachable in practice), but refuse outright rather than
                // leave a live trap for whenever one can.
                if repair_options.is_some() {
                    bail!("repair is not supported for GDLPack-installed instances");
                }

                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(gdlpack::ProgressState::Idle);

                let completion = tokio::spawn(async move {
                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let progress = *modpack_progress_rx.borrow();
                            match progress {
                                gdlpack::ProgressState::Idle => {}
                                gdlpack::ProgressState::ResolvingFiles(count, total) => {
                                    t_addon_metadata.update_items(count as u32, total as u32)
                                }
                                gdlpack::ProgressState::ExtractingOverrides(count, total) => {
                                    t_extract_files.update_items(count as u32, total as u32)
                                }
                            }
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_addon_metadata.complete_opaque();
                    t_extract_files.complete_opaque();
                });

                let modpack_info = gdlpack::prepare_modpack_from_gdlpack(
                    &app,
                    &gdlpack_path,
                    &instance_prep_path,
                    skip_overrides,
                    preparation_oracle.as_ref(),
                    modpack_progress_tx,
                )
                .await;

                completion.await?;
                let modpack_info = modpack_info?;

                tokio::fs::create_dir_all(skip_overrides_path).await?;

                for (downloadable, skip) in modpack_info.downloadables {
                    match skip {
                        Some(skippath) => skipped_mods.push(skippath),
                        None => modpack_downloads.push(downloadable),
                    }
                }

                Some(modpack_info.version)
            }
            None => None,
        };

        let reused = crate::managers::runtime_lifecycle::run_owned_phase(cancel, || async {
            Ok(reuse_earlier_downloads(&setup_path, &staging_dir, &modpack_downloads).await)
        })
        .await?;
        // From here on this attempt's folder holds its downloads.
        *unclaimed = None;

        let concurrency = app
            .settings_manager()
            .get_settings()
            .await?
            .concurrent_downloads;

        let (progress_watch_tx, mut progress_watch_rx) =
            tokio::sync::watch::channel(carbon_net::Progress::new());

        t_download_modpack_files.start_opaque();

        // dropped when the sender is dropped
        let completion = tokio::spawn(async move {
            while progress_watch_rx.changed().await.is_ok() {
                {
                    let progress = progress_watch_rx.borrow();
                    t_download_modpack_files.update_download(
                        progress.current_size as u32,
                        progress.total_size as u32,
                        false,
                    );
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            t_download_modpack_files.complete_opaque();
        });

        #[cfg(test)]
        if !modpack_downloads.is_empty() {
            app.instance_manager
                .test_hooks
                .pause("setup_original_acquisition", instance_id)
                .await;
        }
        // Actually downloads the modpack files. A reused file is accepted only
        // when its bytes hash to the pack's, never by its size alone.
        let download_result = carbon_net::download_multiple(
            &modpack_downloads[..],
            DownloadOptions::builder()
                .cancel_token(cancel.clone())
                .concurrency(concurrency as usize)
                .progress_sender(progress_watch_tx)
                .deep_check(deep_check || reused > 0)
                .build(),
        )
        .await
        .with_context(|| {
            format!("Failed to download modpack instance files for instance {instance_id}")
        });
        completion.await?;
        download_result?;

        if let Some(v) = v {
            tracing::info!("Modpack version: {v:?}");

            version = Some(v.clone());
            config.modpack = modpack.map(|modpack| ModpackInfo {
                modpack,
                locked: config.modpack.map(|m| m.locked).unwrap_or(true),
            });

            config.game_configuration.version = Some(GameVersion::Standard(StandardVersion {
                release: v.release.clone(),
                modloaders: v.modloaders.clone(),
            }));
        }

        // Only generate staging-packinfo if we actually processed a modpack
        // (i.e., file was Some). Otherwise this is just a version/modloader change
        // and we should not touch the existing mods.
        if tmp_packinfo_must_be_regenerated(file.is_some(), apply_started) {
            // normally there would be a problem here because we would be skipping any mods removed by users
            // but since we dont try to update those anyway its fine.
            //
            // Cloned rather than moved: `skipped_mods` is walked again below
            // to merge its hashes into the freshly scanned packinfo, since
            // `scan_dir` never staged these paths and so cannot see them.
            let mut files = skipped_mods.clone();
            // snapshot filetree before applying
            let mut walker = NormalizedWalkdir::new(&staging_dir.join("instance"))?;
            while let Some(entry) = walker.next()? {
                if entry.is_dir {
                    continue;
                }
                files.push(entry.relative_path.to_string());
            }
            drop_shadowed_disabled_twins(&staging_dir.join("instance"), &mut files).await?;

            t_generating_packinfo.start_opaque();

            let files_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
            // At this point the modpack files are all in the staging directory, so that's the path we need to scan.
            // The packinfo on the other hand is in the instance folder itself.
            let mut packinfo =
                packinfo::scan_dir(&instance_prep_path.get_data_path(), Some(&files_refs)).await?;

            // Skip-optimised files were never staged, so the scan cannot see
            // them. "Skipped" means the oracle's hash matched the manifest's,
            // so the oracle entry IS the target hash — merge it, or every
            // unchanged file falls out of the record (the stale-survivor bug).
            for skipped in &skipped_mods {
                if packinfo.files.contains_key(skipped) {
                    continue;
                }
                let Some(hashes) = skip_oracle.as_ref().and_then(|o| o.files.get(skipped)) else {
                    bail!(
                        "skip-optimised path {skipped} has no oracle entry — refusing to write an incomplete packinfo"
                    );
                };
                packinfo.files.insert(skipped.clone(), hashes.clone());
            }

            let mut session = match setup_session.take() {
                Some(session) => session,
                None => {
                    let mut session = Session::new(
                        &installed_snapshot,
                        Producer::Reconstructed,
                        Kind::Pack,
                        repair_options.clone(),
                    )?;
                    // Legacy configured metadata may already be pending. Such
                    // adoption never grants cancellation back to an inferred A.
                    session.instance_id = Some(instance_id.0);
                    session.requested_pack = requested_pack.clone();
                    if let Some(copied) = &copied_session {
                        session.adopted_from = copied.owner().ok();
                        session.installed_config = copied.installed_config.clone();
                    }
                    session
                }
            };
            if let Some(proof) = &archive_proof {
                // The proof was hashed earlier in this same preparation. An
                // archive nothing wrote since still holds those bytes; any
                // other is read again in full.
                let archive = setup_path.join(&proof.name);
                if archive_stamp.is_none() || file_stamp(&archive).await != archive_stamp {
                    let actual = hex::encode(
                        crate::managers::instance::modpack::origin_check::hash_file_sha512(
                            &archive,
                        )
                        .await?,
                    );
                    if actual != proof.sha512 {
                        bail!("original archive changed during reconstruction");
                    }
                }
                if session
                    .archive
                    .as_ref()
                    .is_some_and(|old| old.name != proof.name || old.sha512 != proof.sha512)
                {
                    bail!("original archive changed after admission");
                }
            }
            if let Some(copied) = &copied_session {
                if let Some(original) = &copied.archive {
                    if archive_proof.as_ref().is_none_or(|proof| {
                        proof.name != original.name || proof.sha512 != original.sha512
                    }) {
                        bail!("copied original archive no longer matches source provenance");
                    }
                }
                if let Some(original) = &copied.target {
                    if session::binding(&packinfo, repair_options.as_ref())?.0 != original.digest {
                        bail!("reconstructed copy differs from complete original target");
                    }
                }
            }
            if let Some(original) = &session.target {
                if session::binding(&packinfo, repair_options.as_ref())?.0 != original.digest {
                    bail!("reconstructed inputs differ from admitted complete target");
                }
            }
            session.archive = archive_proof;
            session.pending_config =
                Some(super::super::schema::make_instance_config(config.clone())?);
            session
                .bind_target(
                    &instance_root,
                    &staging_dir,
                    &packinfo,
                    skipped_mods.clone(),
                )
                .await?;
            let owner = session.owner()?;
            let live_staging = setup_path.join("staging");
            // Folders of fresh downloads, which a scanner may still hold on
            // Windows: renamed with a short wait for it (see rename_dir).
            if live_staging.try_exists()? {
                publish::rename_dir(
                    &live_staging,
                    &setup_path.join(format!("prior-staging-{}", uuid::Uuid::new_v4())),
                )
                .await?;
            }
            publish::rename_dir(&staging_dir, &live_staging).await?;
            #[cfg(test)]
            app.instance_manager
                .test_hooks
                .pause("setup_staging_saved", instance_id)
                .await;
            if let Some(copied) = &copied_session {
                let saved = Session::load(&instance_root)
                    .await?
                    .context("copy/private provenance disappeared")?;
                if saved.operation != copied.operation || saved.instance_id != copied.instance_id {
                    bail!("copy/private provenance changed during qualification");
                }
                session::atomic_json(&setup_path.join(session::RECORD), &session, false).await?;
            } else if Session::load(&instance_root).await?.is_some() {
                session.save(&instance_root, &owner).await?;
            } else {
                session.create(&instance_root).await?;
            }
            #[cfg(test)]
            app.instance_manager
                .test_hooks
                .pause("setup_session_saved", instance_id)
                .await;
            adopt_legacy_transitions(&app, &instance_root, &session).await?;
            *session_owner = Some(owner);
            finish_preparation_records(&app, instance_id, &instance_root, &session).await?;
            setup_session = Some(session);

            t_generating_packinfo.complete_opaque();
        } else {
            t_generating_packinfo.complete_opaque();
        }

        if !oracle_diagnostics.is_empty() {
            let mut universe = BTreeSet::new();
            // The oracle is not the old record: include declared paths even if
            // this scan could not open them.
            for record in [&packinfo_path, &tmp_packinfo_path] {
                match tokio::fs::read_to_string(record).await {
                    Ok(text) => universe.extend(packinfo::parse_packinfo(&text)?.files.into_keys()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            let insensitive =
                disk_scan::probe_case_insensitive(&instance_path.get_data_path()).await;
            disk_scan::validate_oracle_diagnostics(&oracle_diagnostics, &universe, insensitive)?;
        }

        trace!("marking modpack initialization as complete");

        tracing::info!("queueing metadata caching for running instance");
        t_fill_cache.start_opaque();

        // Files this setup wrote carry new stamps, so an ordinary pass picks
        // them up without re-reading every unchanged addon.
        app.meta_cache_manager()
            .queue_caching(
                crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
                false,
            )
            .await;

        t_fill_cache.complete_opaque();

        trace!("queued metadata caching");
    }

    let subtasks = TSubtasksInner {
        t_request_version_info,
        t_download_files,
        t_scan_files,
        t_generating_packinfo,
        t_request_modloader_info,
        t_request_minecraft_files,
        t_download_java,
        t_apply_staging,
        t_extract_java,
        t_fill_cache,
        t_extract_natives,
        t_reconstruct_assets,
        t_forge_processors,
        t_neoforge_processors,
        t_finalize_import,
    };

    Ok((
        Arc::new(subtasks),
        version,
        repair_options,
        oracle_disk_scan,
    ))
}

/// Apply an admitted complete target, retain original sources through audit and
/// promotion (where the volume has hard links; see [`publish_staged`]), then
/// merge pending metadata into the current configuration. The
/// durable Applied phase permits recovery across the promotion/config gap;
/// neither a missing marker nor a packinfo file alone proves completion.
pub async fn process_modpack_staging(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    instance_shortpath: String,
    t_subtasks: &TSubtasks,
    repair_options: Option<RepairMarkerFile>,
    oracle_disk_scan: Option<disk_scan::DiskScan>,
    session_owner: Option<&Owner>,
) -> anyhow::Result<()> {
    let runtime_path = app.settings_manager().runtime_path.clone();
    let instance_path = runtime_path
        .get_instances()
        .get_instance_path(&instance_shortpath);

    let instance_root = instance_path.get_root();
    let setup_path = instance_root.join(".setup");
    let is_first_run = setup_path.is_dir();
    if !is_first_run {
        return Ok(());
    }
    let manager = app.instance_manager();
    let operation = manager.lock_instance(instance_id).await;
    let (current_shortpath, _) = manager.current_config(instance_id).await?;
    if current_shortpath != instance_shortpath {
        bail!("instance root changed during preparation");
    }
    let owner = session_owner.context("setup is not admitted for apply")?;
    let mut session = Session::require(&instance_root, owner).await?;
    if session.original_options != repair_options {
        bail!("setup original options changed");
    }
    if session.kind != Kind::Pack {
        session.phase = Phase::Committed;
        session.save(&instance_root, owner).await?;
        return Ok(());
    }
    if matches!(session.phase, Phase::Applied | Phase::Committed) {
        finish_session_commit(&app, &operation, &instance_root, &mut session).await?;
        return Ok(());
    }
    // The one full read of the staged pack per apply, before anything below
    // publishes or records it. It is the first step of applying, and can take
    // a while for a large pack, so the progress shows it.
    t_subtasks.t_apply_staging.start_opaque();
    let sources = session
        .verify_sources(&setup_path.join("staging"), &instance_root.join("instance"))
        .await?;
    finish_preparation_records(&app, instance_id, &instance_root, &session).await?;
    adopt_legacy_transitions(&app, &instance_root, &session).await?;
    session::plain_directory(&setup_path.join("staging"))?;
    let bound = session
        .target
        .as_ref()
        .context("complete target has not been resolved")?;
    let bound_target = packinfo::parse_packinfo(&bound.packinfo)?;
    let bound_digest = bound.digest.clone();

    let staging_dir = setup_path.join("staging");

    if staging_dir.exists() {
        let staging_packinfo = setup_path.join("staging-packinfo.json");

        if !staging_packinfo.try_exists()? {
            bail!("owned pack staging inventory is missing; preserve original inputs");
        }

        let packinfo_path = instance_root.join("packinfo.json");
        let tmp_packinfo_path = instance_root.join("tmp-packinfo.json");
        let apply_started_path = setup_path.join("apply-started");

        if !tmp_packinfo_path.try_exists()? {
            tokio::fs::write(
                &tmp_packinfo_path,
                &session.target.as_ref().unwrap().packinfo,
            )
            .await?;
        }

        let old_packinfo = match tokio::fs::read_to_string(&packinfo_path).await {
            Ok(s) => Some(packinfo::parse_packinfo(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let target_packinfo =
            packinfo::parse_packinfo(&tokio::fs::read_to_string(&tmp_packinfo_path).await?)?;
        if session::binding(&target_packinfo, repair_options.as_ref())?.0 != bound_digest {
            bail!("temporary target differs from admitted complete target");
        }

        // Staged set: files physically present under .setup/staging/instance,
        // as packinfo-style keys.
        let mut staged = HashSet::new();
        let mut walker = NormalizedWalkdir::new(&staging_dir.join("instance"))?;
        while let Some(entry) = walker.next()? {
            if entry.is_dir {
                continue;
            }
            // Mirrors packinfo::scan_dir's own `.disabled` stripping
            // (packinfo/scan.rs:30-32): a pack that ships an override
            // disabled by default stages it under the `.disabled` name, but
            // tmp-packinfo.json (built by scan_dir) keys it under the
            // enabled name. `staged` has to match that key shape, or a path
            // whose only staged copy is `.disabled`-suffixed looks unstaged
            // to the planner and a fresh install of that path errors
            // permanently (MissingStagedSource).
            let mut key = entry.relative_path.to_string();
            if key.ends_with(".disabled") {
                key.truncate(key.len() - ".disabled".len());
            }
            staged.insert(key);
        }

        let universe: BTreeSet<String> = old_packinfo
            .iter()
            .flat_map(|p| p.files.keys().cloned())
            .chain(target_packinfo.files.keys().cloned())
            .collect();
        // The pre-download oracle is only a skip optimization. External
        // changes and filesystem aliases require a fresh declared-path scan.
        let disk_scan::DiskScan {
            states: disk,
            coexisting_disabled_twin_md5,
        } = scan_for_apply(&instance_root.join("instance"), &universe, oracle_disk_scan).await?;

        // Whether this instance's data dir lives on a filesystem that folds
        // path case together (Windows, default-configuration macOS) — see
        // `disk_scan::probe_case_insensitive`'s own doc for the fallback
        // semantics. Threaded into both the planner and the untracked-file
        // walk below so a case-only rename can't lose the file it renames
        // and a case-variant spelling of a tracked path is never treated as
        // untracked/deletable.
        let fs_case_insensitive =
            disk_scan::probe_case_insensitive(&instance_root.join("instance")).await;

        // Absent marker -> ordinary version-change reconciliation, unchanged
        // from before repair mode existed. Present -> re-reconcile every
        // pack-tracked path against `target` alone (see
        // `apply_plan::decide_repair`), regardless of what `old` says.
        let mode = match &repair_options {
            Some(options) => apply_plan::ApplyMode::Repair {
                re_enable_disabled: options.re_enable_disabled,
            },
            None => apply_plan::ApplyMode::VersionChange,
        };

        let planned = plan_staging(
            apply_plan::PlanInputs {
                old: old_packinfo.as_ref(),
                target: &target_packinfo,
                staged: &staged,
                disk: &disk,
                coexisting_disabled_twin_md5: &coexisting_disabled_twin_md5,
                mode,
                fs_case_insensitive,
            },
            &instance_root,
            repair_options.as_ref(),
            Some(owner),
        )
        .await;
        let mut entries = match planned {
            Ok(entries) => entries,
            Err(error) => {
                return Err(unstaged_plan_error(&instance_root, &mut session, owner, error).await);
            }
        };

        if entries
            .iter()
            .any(|entry| entry.reason == apply_plan::PlanReason::DisabledReplaceResumed)
        {
            bail!(
                "legacy disabled replacement needs pristine archive reconstruction; matching live hashes are not ownership"
            );
        }

        disk_scan::protect_resolved_saves(
            &mut entries,
            &instance_root.join("instance"),
            old_packinfo.as_ref(),
        );

        session.phase = Phase::Applying;
        session.save(&instance_root, owner).await?;

        // Preserve the existing durable apply boundary for retries. New
        // publication retains sources and owned representation outcomes where
        // the volume has hard links; legacy setup, and publication on volumes
        // without them, may have consumed them, so the full target record
        // remains authoritative in every case.
        {
            let marker_file = File::create(&apply_started_path).await?;
            marker_file.sync_all().await?;
        }
        fsync_dir(&setup_path).await?;

        let mut representation_notes = Vec::new();
        let entries = execute_bound_plan(
            &entries,
            &instance_root,
            &staging_dir,
            &target_packinfo,
            repair_options.as_ref(),
            &mut representation_notes,
            Some(owner),
            &sources,
        )
        .await?;

        // Repair-only: paths the user explicitly ticked for removal in the
        // preview, applied after the plan so a cleanup can never race the
        // pack's own reconciliation of the same path.
        let cleanup_context = {
            let manager = app.instance_manager();
            let instances = manager.instances.read().await;
            instances
                .get(&instance_id)
                .and_then(|i| i.data().ok())
                .and_then(|d| d.config.modpack.as_ref())
                .map(|p| crate::managers::instance::modpack::PackProofContext::from(&p.modpack))
        };
        let mut user_removed = match &repair_options {
            Some(options) => {
                apply_bound_cleanup(
                    options,
                    old_packinfo.as_ref(),
                    &target_packinfo,
                    &instance_root,
                    fs_case_insensitive,
                    cleanup_context.as_ref(),
                    Some(owner),
                )
                .await
            }
            None => CleanupReport::default(),
        };

        user_removed.notes.extend(representation_notes);

        trace!("Creating update audit files");
        let audit_dir = instance_root.join(".install_audit");

        // delete old audit dir if it exists
        if (audit_dir.exists()) {
            tokio::fs::remove_dir_all(&audit_dir).await?;
        }

        tokio::fs::create_dir(&audit_dir).await?;

        let audit_file = audit_dir.join("audit.txt");
        let audit_txt = render_cleanup_audit(&entries, &user_removed);
        // Sync through the writing handle: Windows FlushFileBuffers requires
        // a write-capable handle.
        let mut audit_handle = File::create(&audit_file).await?;
        audit_handle.write_all(audit_txt.as_bytes()).await?;
        audit_handle.sync_all().await?;
        session::sync_dir(&audit_dir).await?;
        session::sync_dir(&instance_root).await?;

        // Durable Applied precedes either publication. A restart verifies the
        // exact bound target and repeats the current-field config merge.
        session.phase = Phase::Applied;
        session.save(&instance_root, owner).await?;
        finish_session_commit(&app, &operation, &instance_root, &mut session).await?;
        trace!("Staging complete");
        t_subtasks.t_apply_staging.complete_opaque();

        drop(operation);
        // Trigger caching now that modpack installation is complete
        app.meta_cache_manager()
            .watch_and_prioritize(Some(
                crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
            ))
            .await;
    }

    Ok(())
}

// Resolved itself owns the complete target and verified source inventory.
// Publishing the two derived records is restartable at either await boundary;
// stale/truncated records are retained, never used as target authority.
// Both records derive from the bound target, not from the staged bytes, so
// the sources are not hashed here: preparation scanned them moments before
// binding, and the apply hashes them once before it publishes anything.
async fn finish_preparation_records(
    app: &Arc<AppInner>,
    id: InstanceId,
    root: &Path,
    session: &Session,
) -> anyhow::Result<()> {
    if session.phase != Phase::Resolved {
        return Ok(());
    }
    let owner = session.owner()?;
    let saved = Session::require(root, &owner).await?;
    if saved.phase != Phase::Resolved {
        bail!("setup phase changed during preparation publication");
    }
    let target = saved
        .target
        .as_ref()
        .context("resolved setup lacks complete target")?;
    let mut inventory: Vec<_> = target
        .sources
        .values()
        .cloned()
        .chain(target.skipped.iter().cloned())
        .collect();
    inventory.sort();
    inventory.dedup();
    for (name, path, bytes) in [
        (
            "setup_inventory_saved",
            root.join(".setup/staging-packinfo.json"),
            serde_json::to_vec_pretty(&inventory)?,
        ),
        (
            "setup_target_saved",
            root.join("tmp-packinfo.json"),
            target.packinfo.as_bytes().to_vec(),
        ),
    ] {
        Session::require(root, &owner).await?;
        let same = match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                tokio::fs::read(&path).await? == bytes
            }
            Ok(_) => bail!(
                "preparation record is not a plain file; preserved: {}",
                path.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if !same {
            replace_preparation_record(&path, root, bytes).await?;
        }
        #[cfg(test)]
        app.instance_manager.test_hooks.pause(name, id).await;
    }
    Ok(())
}

// Original legacy/current preparation records are evidence, even when their
// contents are truncated or stale. Preserve exact bytes before publishing the
// independently reconstructed and session-bound complete replacement.
async fn replace_preparation_record(
    path: &Path,
    root: &Path,
    bytes: Vec<u8>,
) -> anyhow::Result<()> {
    let setup = root.join(".setup");
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                bail!(
                    "preparation record is not a plain file; preserved: {}",
                    path.display()
                );
            }
            let prior = setup.join(format!(
                "prior-record-{}-{}",
                path.file_name()
                    .context("record has no filename")?
                    .to_string_lossy(),
                uuid::Uuid::new_v4()
            ));
            session::atomic_bytes(&prior, tokio::fs::read(path).await?, true).await?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    // tmp-packinfo.json sits in the instance folder itself, which may be a
    // link to another drive; only the setup's own folders must be plain.
    if path.parent() == Some(root) {
        session::atomic_bytes_in_root(path, bytes, false).await
    } else {
        session::atomic_bytes(path, bytes, false).await
    }
}

async fn finish_session_commit(
    app: &Arc<AppInner>,
    operation: &super::super::InstanceOperationGuard,
    root: &Path,
    session: &mut Session,
) -> anyhow::Result<()> {
    let owner = session.owner()?;
    Session::require(root, &owner).await?;
    let target = session
        .target
        .as_ref()
        .context("applied session has no complete target")?;
    let tmp = root.join("tmp-packinfo.json");
    let promoted = root.join("packinfo.json");
    if tmp.try_exists()? {
        let info = packinfo::parse_packinfo(&tokio::fs::read_to_string(&tmp).await?)?;
        if session::binding(&info, session.original_options.as_ref())?.0 != target.digest {
            bail!("pending promotion does not match setup target");
        }
        tokio::fs::rename(&tmp, &promoted).await?;
        session::sync_dir(root).await?;
        #[cfg(test)]
        app.instance_manager
            .test_hooks
            .pause("pack_promoted", operation.instance_id())
            .await;
    }
    let info = packinfo::parse_packinfo(&tokio::fs::read_to_string(&promoted).await?)?;
    if session::binding(&info, session.original_options.as_ref())?.0 != target.digest {
        bail!("installed record is not this session's promoted target");
    }
    let prepared = super::super::schema::parse_instance_config(
        session
            .pending_config
            .as_ref()
            .context("pending configuration missing")?,
    )?;
    let original = super::super::schema::parse_instance_config(&session.installed_config)?.modpack;
    app.instance_manager()
        .publish_modpack_config_owned(operation, prepared, original)
        .await?;
    session.phase = Phase::Committed;
    session.save(root, &owner).await?;
    finish_promoted_staging(&root.join(".setup/staging"), &root.join(".setup")).await?;
    app.invalidate(GET_MODPACK_INFO, Some(owner.instance_id.into()));
    Ok(())
}

async fn scan_for_apply(
    data: &Path,
    universe: &BTreeSet<String>,
    oracle: Option<disk_scan::DiskScan>,
) -> anyhow::Result<disk_scan::DiskScan> {
    drop(oracle);
    disk_scan::scan_disk_state(data, universe).await
}

/// The apply's answer to a plan that needs a staged copy of a file the
/// preparation never staged. Preparation skips a file that is already in
/// place with the pack's bytes, and the apply judges the files again: one
/// that went missing or changed in between (antivirus, or a Repair that was
/// interrupted and retried later) has nothing to be restored from.
///
/// Before the apply started nothing changed, so the setup goes back to be
/// prepared again (see [`Session::release_target`]) and the next Play
/// downloads that file. Once it started, the setup can't be prepared again
/// over files it may have changed, so the error names its way out instead.
async fn unstaged_plan_error(
    root: &Path,
    session: &mut Session,
    owner: &Owner,
    error: anyhow::Error,
) -> anyhow::Error {
    let Some(apply_plan::PlanError::MissingStagedSource { path }) =
        error.downcast_ref::<apply_plan::PlanError>()
    else {
        return error;
    };
    let file = path.trim_start_matches('/').to_owned();
    if session.phase == Phase::Resolved {
        match session.release_target(root, owner).await {
            Ok(()) => {
                return error.context(format!(
                    "{file} was changed or removed after this modpack's files were checked, so they have to be checked again. Nothing in the instance was changed. Press Play to try again; the files already downloaded are reused."
                ));
            }
            Err(release) => {
                tracing::warn!(?release, "setup could not be prepared again");
            }
        }
    }
    let way_out = session.staged_way_out();
    error.context(format!(
        "The modpack file {file} has no downloaded copy to install, so this setup can't be finished. {way_out}"
    ))
}

/// Size and modification time of `path`, to tell whether anything wrote it
/// since. None when either is unavailable, which callers treat as changed.
async fn file_stamp(path: &Path) -> Option<(u64, std::time::SystemTime)> {
    let metadata = tokio::fs::symlink_metadata(path).await.ok()?;
    Some((metadata.len(), metadata.modified().ok()?))
}

/// A pack can ship a file both enabled and as its `.disabled` twin, say a
/// mod it downloads plus an override of the same name that is disabled. Both
/// stand for one target path. The bare file is the one installed, as before
/// (the apply prefers it, see [`resolve_staged`]), so the twin is removed
/// from this attempt's own preparation and the target records the bare
/// file's hashes. A bare file that preparation skipped because it is already
/// installed counts the same, so an update ends where a fresh install does.
async fn drop_shadowed_disabled_twins(
    staged: &Path,
    files: &mut Vec<String>,
) -> anyhow::Result<()> {
    let present: HashSet<String> = files.iter().cloned().collect();
    // Mirrors `packinfo::scan_dir`'s key: one `.disabled` is stripped, so only
    // a bare spelling collides with its twin.
    let shadowed: Vec<String> = files
        .iter()
        .filter(|file| {
            file.strip_suffix(".disabled")
                .is_some_and(|bare| !bare.ends_with(".disabled") && present.contains(bare))
        })
        .cloned()
        .collect();
    if shadowed.is_empty() {
        return Ok(());
    }
    files.retain(|file| !shadowed.contains(file));
    for twin in shadowed {
        tracing::warn!(
            "{twin}: the modpack also ships this file enabled, so the enabled copy is installed"
        );
        match tokio::fs::remove_file(staged.join(twin.trim_start_matches('/'))).await {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Puts the files earlier, unfinished attempts of this setup downloaded
/// where `current`, this attempt's preparation, downloads them, then removes
/// those attempts' folders: a retry downloads only what is still missing, and
/// `.setup` holds at most one partial download. Best effort: a file that
/// can't be reused is downloaded again.
///
/// An earlier attempt's `preparing-*` folder is never installed from (the
/// finished one is renamed to `staging` first), so its files are moved. A
/// `staging` beside an unfinished attempt (one 2.0.40 left, or the staging a
/// re-preparation replaces) is only hard-linked from and stays as it was.
/// Only downloads that carry a checksum are reused, and the caller makes the
/// download hash every file already in place, so a reused file is kept only
/// when its bytes are the pack's. Overrides are never reused: this attempt
/// has extracted its own. Returns how many files were reused.
async fn reuse_earlier_downloads(
    setup: &Path,
    current: &Path,
    downloads: &[Downloadable],
) -> usize {
    let wanted: Vec<PathBuf> = downloads
        .iter()
        .filter(|download| download.checksum.is_some())
        .filter_map(|download| download.path.strip_prefix(current).ok())
        .filter(|relative| {
            relative
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
        })
        .map(Path::to_path_buf)
        .collect();
    let setup = setup.to_owned();
    let current = current.to_owned();
    match tokio::task::spawn_blocking(move || {
        reuse_earlier_downloads_blocking(&setup, &current, &wanted)
    })
    .await
    {
        Ok(Ok(reused)) => reused,
        Ok(Err(error)) => {
            tracing::warn!(?error, "earlier downloads not reused");
            0
        }
        Err(error) => {
            tracing::warn!(?error, "earlier downloads not reused");
            0
        }
    }
}

fn reuse_earlier_downloads_blocking(
    setup: &Path,
    current: &Path,
    wanted: &[PathBuf],
) -> io::Result<usize> {
    let plain_dir =
        |path: &Path| std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir());
    let mut earlier = Vec::new();
    for entry in std::fs::read_dir(setup)? {
        let path = entry?.path();
        let named = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("preparing-"));
        if named && path != current && plain_dir(&path) {
            let modified = std::fs::symlink_metadata(&path)
                .and_then(|metadata| metadata.modified())
                .ok();
            earlier.push((modified, path));
        }
    }
    // The newest attempt got furthest.
    earlier.sort_by(|a, b| b.0.cmp(&a.0));
    let staging = Some(setup.join("staging")).filter(|staging| plain_dir(staging));
    // Every folder on the way must be a plain one too, so a link can never
    // lead a move outside the setup.
    let plain_file_beneath = |root: &Path, relative: &Path| {
        let mut path = root.to_path_buf();
        let mut components = relative.components().peekable();
        while let Some(component) = components.next() {
            path.push(component);
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                return false;
            };
            let last = components.peek().is_none();
            if (last && !metadata.is_file()) || (!last && !metadata.is_dir()) {
                return false;
            }
        }
        true
    };
    let mut reused = 0;
    for relative in wanted {
        let destination = current.join(relative);
        match std::fs::symlink_metadata(&destination) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            // Extracted by this attempt, or reused already.
            _ => continue,
        }
        let mut place = |source: PathBuf, link: bool| {
            let placed = destination
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| {
                    if link {
                        std::fs::hard_link(&source, &destination)
                    } else {
                        std::fs::rename(&source, &destination)
                    }
                });
            match placed {
                Ok(()) => {
                    reused += 1;
                    true
                }
                Err(error) => {
                    tracing::debug!(?error, source = %source.display(), "earlier download not reused");
                    false
                }
            }
        };
        let moved = earlier.iter().any(|(_, folder)| {
            plain_file_beneath(folder, relative) && place(folder.join(relative), false)
        });
        if !moved {
            if let Some(staging) = &staging {
                if plain_file_beneath(staging, relative) {
                    place(staging.join(relative), true);
                }
            }
        }
    }
    for (_, folder) in &earlier {
        if let Err(error) = std::fs::remove_dir_all(folder) {
            tracing::warn!(?error, path = %folder.display(), "earlier preparation kept");
        }
    }
    Ok(reused)
}

/// Finishes the leftover cleanup of a staging pass whose apply already fully
/// landed — `packinfo_path` (checked by the caller before this is invoked)
/// already fully describes the target state, so all that is left is
/// removing the now-superfluous `staging_dir` and recording
/// `modpack-complete`. Used both for a pass that just promoted
/// `tmp-packinfo.json` itself and for a resumed pass that finds the promote
/// already happened (`tmp-packinfo.json` gone, `.setup/apply-started`
/// present, `packinfo.json` present — see `staging_apply_already_promoted`,
/// which the caller uses to tell this apart from an apply that simply never
/// started) — see [`process_modpack_staging`]'s doc comment for the
/// invariant this is the tail of. Never touches `packinfo_path` or
/// `tmp_packinfo_path` itself.
async fn finish_promoted_staging(staging_dir: &Path, setup_path: &Path) -> anyhow::Result<()> {
    match tokio::fs::remove_dir_all(staging_dir).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    tokio::fs::write(setup_path.join("modpack-complete"), "").await?;
    Ok(())
}

/// Fsyncs `dir`'s own directory entry (not its contents) — the other half
/// of durably creating a file: a file's own `sync_all` only guarantees its
/// bytes reached disk, not that the directory entry pointing at it did too.
/// Without this, a crash can leave a freshly-created file's bytes durable on
/// disk yet the directory listing that would make it discoverable again
/// still unflushed, so the file appears to have never been created at all
/// after an unclean restart — undermining `sync_all`'s own guarantee for
/// exactly the callers (like the `apply-started` marker write) that depend
/// on "this file exists" surviving a crash in a specific order relative to
/// other writes.
///
/// Windows currently has no directory flush here. File sync and source
/// retention do not promise power-loss ordering on every supported volume.
#[cfg(windows)]
async fn fsync_dir(_dir: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(not(windows))]
async fn fsync_dir(dir: &Path) -> anyhow::Result<()> {
    #[cfg(test)]
    {
        staging_test::record_sync(dir);
        staging_test::run_hook("sync-directory", dir);
        staging_test::io_fault("sync-directory", dir)?;
    }
    // A volume that does not sync folders (a VirtualBox shared folder, some
    // network mounts) leaves the entries to the system.
    crate::util::durable::sync_dir_async(dir).await?;
    Ok(())
}

/// Persist each newly-created recovery edge before moving a live file.
async fn sync_recovery_hierarchy(root: &Path, batch: &Path) -> anyhow::Result<()> {
    fsync_dir(root).await?;
    fsync_dir(&root.join(".repair-recovery")).await?;
    fsync_dir(batch).await
}

async fn sync_recovery_payload(payload: &Path) -> anyhow::Result<()> {
    #[cfg(test)]
    staging_test::io_fault("sync-file", payload)?;
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true);
    // Windows FlushFileBuffers requires a write-capable handle. Do not
    // change permissions: an unsupported/denied flush preserves the payload
    // and reports failure rather than claiming durable completion.
    #[cfg(windows)]
    options.write(true);
    options.open(payload).await?.sync_all().await?;
    Ok(())
}

/// A staged file that no longer holds the bytes it is published as. After an
/// interrupted apply that linked it into the instance, an edit made in place
/// there changes it too.
#[derive(Debug)]
struct StagedSourceChanged(PathBuf);

impl std::fmt::Display for StagedSourceChanged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "staged content changed before publication: {:?}", self.0)
    }
}

impl std::error::Error for StagedSourceChanged {}

/// The audit note for a pack file whose staged copy changed after the apply
/// started: what happened, and how to get the pack's version back.
fn source_changed_note(path: &str) -> String {
    format!(
        "{path} was changed while this setup was interrupted, so the modpack's copy of it could \
         not be installed. Use Repair Modpack to restore it."
    )
}

/// Copy to a private, complete destination-volume file, verify it against
/// the target record, then publish. Originals remain in staging until the
/// complete target is promoted and staging is finalized. Representation
/// changes and recovery restores publish this way; ordinary pack files are
/// linked or moved ([`publish_staged`]) and copied only where neither
/// reaches their folder.
async fn publish_staged_copy(
    source: &Path,
    destination: &Path,
    expected: Option<[u8; 64]>,
    no_clobber: bool,
) -> anyhow::Result<bool> {
    let parent = destination
        .parent()
        .context("publication destination has no parent")?;
    let published = copy_staged(source, destination, expected, no_clobber).await?;
    if published {
        fsync_dir(parent).await?;
    }
    Ok(published)
}

/// [`publish_staged_copy`] without the sync of the destination's folder: the
/// copy's own bytes are synced before it is published, and the caller owes
/// its folder one sync (see [`Publications::finish`]).
async fn copy_staged(
    source: &Path,
    destination: &Path,
    expected: Option<[u8; 64]>,
    no_clobber: bool,
) -> anyhow::Result<bool> {
    let source = source.to_path_buf();
    let destination = destination.to_path_buf();
    let parent = destination
        .parent()
        .context("publication destination has no parent")?
        .to_path_buf();
    tokio::task::spawn_blocking(move || -> anyhow::Result<bool> {
        use sha2::Digest;
        use std::io::{Read, Write};
        let mut input = std::fs::File::open(&source)?;
        let mut private = publish::temp_in(&parent)?;
        let mut digest = sha2::Sha512::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let n = input.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            digest.update(&buffer[..n]);
            private.write_all(&buffer[..n])?;
        }
        let actual: [u8; 64] = digest.finalize().into();
        if expected.is_some_and(|hash| hash != actual) {
            return Err(StagedSourceChanged(source).into());
        }
        private
            .as_file()
            .set_permissions(input.metadata()?.permissions())?;
        private.as_file().sync_all()?;
        #[cfg(test)]
        staging_test::run_hook("publish", &destination);
        if no_clobber {
            match publish::persist_noclobber(private, &destination) {
                Ok(()) => Ok(true),
                Err(error)
                    if error.kind() == std::io::ErrorKind::AlreadyExists
                        || destination.symlink_metadata().is_ok() =>
                {
                    Ok(false)
                }
                Err(error) => Err(error.into()),
            }
        } else {
            publish::persist(private, &destination)?;
            Ok(true)
        }
    })
    .await?
}

/// What publishing one ordinary staged file did.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Publication {
    /// The destination is now a second name of the staged file. Its folder
    /// still needs one sync before the apply is recorded as Applied.
    Linked,
    /// The destination's folder can't hold a link to the staged file
    /// (`reason`), so the staged file itself was moved there, as 2.0.40
    /// moved every pack file: no byte is written and no space is used, but
    /// the staged name is gone, so a resumed apply finds the file at its
    /// destination instead (see [`Session::verify_sources`]). The
    /// destination's folder and the staging folder it left (`vacated`) still
    /// need one sync each.
    Moved { reason: String, vacated: PathBuf },
    /// Neither a link nor a move reached the destination (`reason`): its
    /// folder is on another drive, or the move was refused, say while
    /// another program held the staged file. A verified copy, its bytes
    /// synced, was published instead, and the staged original stays until
    /// the setup commits. Its folder still needs one sync.
    Copied { reason: String },
    /// Nothing was published: a Create found its destination occupied.
    Kept,
    /// Nothing was published: the staged file no longer holds the pack's
    /// bytes. Only a resumed apply meets this, after its earlier attempt
    /// linked the file into the instance and it was edited there in place.
    SourceChanged,
}

/// Destination folders whose earlier publications were refused, so that
/// their later files skip what can't work there. Each apply learns its own.
#[derive(Default)]
struct Routes {
    /// Folders that refused a link to a staged file: their files are moved.
    unlinkable: HashSet<PathBuf>,
    /// Folders a move can't reach (another drive): their files are copied.
    unmovable: HashSet<PathBuf>,
}

/// Publishes an ordinary staged file as cheaply as 2.0.40's rename did.
///
/// Where the destination's folder takes one, that is a hard link: it writes
/// no bytes and needs no flush, and the staged name stays until the setup is
/// committed, so `verify_sources` and a resumed apply still find every
/// original. That includes a disabled replacement interrupted before its
/// bare file is gone. A Create (`no_clobber`) links straight to
/// `destination`, which never replaces anything that arrived there first,
/// such as a late save. A replacement links to a private, hidden, non-`.jar`
/// sibling and renames that over `destination`.
///
/// Volumes without hard links (FAT32, exFAT, some network shares) get
/// 2.0.40's move of the staged file instead, never replacing for a Create,
/// and `routes` remembers the folder so its later files go straight to the
/// move. That keeps the pack's bytes on the drive once and writes none of
/// them again. Only a folder on another drive, which no rename reaches, or a
/// move that was refused gets a copy, checked against the target hash. std
/// adds the Windows long-path prefix to the link and the rename itself.
async fn publish_staged(
    source: &Path,
    destination: &Path,
    expected: [u8; 64],
    no_clobber: bool,
    routes: &mut Routes,
) -> anyhow::Result<Publication> {
    // A replacement whose destination already is the staged file resumes an
    // earlier attempt that linked it. The planner replaces only a file that
    // differs from the target, so the shared bytes were edited in place
    // since, and there is no pack copy left to publish. The hash decides,
    // not the match alone: Windows compares a 64-bit file index, which ReFS
    // does not keep unique, and a source that still holds the pack's bytes
    // is linked as usual.
    if !no_clobber {
        let (a, b) = (destination.to_owned(), source.to_owned());
        if tokio::task::spawn_blocking(move || publish::same_file(&a, &b)).await?
            && crate::managers::instance::modpack::origin_check::hash_file_sha512(source).await?
                != expected
        {
            return Ok(Publication::SourceChanged);
        }
    }
    let folder = destination
        .parent()
        .context("publication destination has no parent")?
        .to_owned();
    let mut reason = None;
    if !routes.unlinkable.contains(&folder) {
        let (source, destination) = (source.to_owned(), destination.to_owned());
        let linked =
            tokio::task::spawn_blocking(move || link_staged(&source, &destination, no_clobber))
                .await??;
        match linked {
            Ok(publication) => return Ok(publication),
            Err(error) => {
                routes.unlinkable.insert(folder.clone());
                reason = Some(format!("a link was refused: {error}"));
            }
        }
    }
    if !routes.unmovable.contains(&folder) {
        let (moved_source, moved_destination) = (source.to_owned(), destination.to_owned());
        let moved = tokio::task::spawn_blocking(move || {
            move_staged(&moved_source, &moved_destination, no_clobber)
        })
        .await??;
        match moved {
            Ok(true) => {
                return Ok(Publication::Moved {
                    reason: reason
                        .unwrap_or_else(|| "an earlier link in this folder was refused".into()),
                    vacated: source
                        .parent()
                        .context("staged source has no parent")?
                        .to_owned(),
                });
            }
            Ok(false) => return Ok(Publication::Kept),
            Err(error) => {
                if publish::crosses_devices(&error) {
                    routes.unmovable.insert(folder);
                }
                reason = Some(format!("a move was refused: {error}"));
            }
        }
    }
    let reason = reason.unwrap_or_else(|| "an earlier move into this folder was refused".into());
    tracing::trace!(
        "Copying {} instead of linking or moving it: {reason}",
        destination.display()
    );
    Ok(
        match copy_staged(source, destination, Some(expected), no_clobber).await? {
            true => Publication::Copied { reason },
            false => Publication::Kept,
        },
    )
}

/// The link half of [`publish_staged`]. The inner error is a link this
/// volume or folder refused; the caller moves or copies instead.
fn link_staged(
    source: &Path,
    destination: &Path,
    no_clobber: bool,
) -> anyhow::Result<Result<Publication, io::Error>> {
    #[cfg(test)]
    {
        staging_test::run_hook("publish", destination);
        staging_test::io_fault("link", destination)?;
    }
    if no_clobber {
        return Ok(match staged_hard_link(source, destination, destination) {
            Ok(()) => Ok(Publication::Linked),
            Err(error)
                if error.kind() == io::ErrorKind::AlreadyExists
                    || destination.symlink_metadata().is_ok() =>
            {
                Ok(Publication::Kept)
            }
            Err(error) => Err(error),
        });
    }
    let private = private_link_path(destination)?;
    if let Err(error) = staged_hard_link(source, &private, destination) {
        return Ok(Err(error));
    }
    let renamed = rename_staged_link(&private, destination);
    if private.symlink_metadata().is_ok() {
        // Only this attempt's own second name of the staged file goes. A
        // failed rename leaves it, and so does a POSIX rename between two
        // names of one file, which succeeds without doing anything.
        let _ = std::fs::remove_file(&private);
    }
    renamed?;
    Ok(Ok(Publication::Linked))
}

/// The move half of [`publish_staged`], 2.0.40's rename of the staged file:
/// `true` once it is at `destination`, `false` when a Create found
/// `destination` occupied (nothing moved). The inner error is a move this
/// folder refused, such as one to another drive; the caller copies instead.
fn move_staged(
    source: &Path,
    destination: &Path,
    no_clobber: bool,
) -> anyhow::Result<Result<bool, io::Error>> {
    #[cfg(test)]
    {
        staging_test::run_hook("publish", destination);
        staging_test::io_fault("move", destination)?;
        if let Some(error) = staging_test::move_fault(destination) {
            return Ok(Err(error));
        }
    }
    if no_clobber {
        return Ok(match publish::rename_noclobber(source, destination) {
            Ok(()) => Ok(true),
            Err(error)
                if error.kind() == io::ErrorKind::AlreadyExists
                    || destination.symlink_metadata().is_ok() =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        });
    }
    Ok(std::fs::rename(source, destination).map(|()| true))
}

/// A unique name next to `destination` for a replacement's link. Hidden on
/// Unix, and never a `.jar`, so a name left by a crash between the link and
/// the rename is not loaded as a mod.
fn private_link_path(destination: &Path) -> anyhow::Result<PathBuf> {
    Ok(destination
        .parent()
        .context("publication destination has no parent")?
        .join(format!(
            ".gdl-publish-{}.tmp",
            uuid::Uuid::new_v4().simple()
        )))
}

/// `std::fs::hard_link`, with a test seam keyed by the final destination.
fn staged_hard_link(source: &Path, link: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(test)]
    if let Some(error) = staging_test::link_fault(destination) {
        return Err(error);
    }
    #[cfg(not(test))]
    let _ = destination;
    std::fs::hard_link(source, link)
}

fn rename_staged_link(private: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(test)]
    staging_test::io_fault("link-rename", destination)?;
    std::fs::rename(private, destination)
}

/// The ordinary publications of one apply: their audit outcome, and what
/// they still owe once every entry is done.
struct Publications {
    data: PathBuf,
    /// Folders that received a link, a moved staged file or a copy, in this
    /// attempt or an interrupted earlier one, and those above them inside
    /// `data`, since the apply may have created them. Published files are
    /// not flushed one by one; these folders are synced once each instead.
    folders: BTreeSet<PathBuf>,
    /// Staging folders that a moved file left, synced once each too, so a
    /// staged name can't come back once committing deletes the staging.
    vacated: BTreeSet<PathBuf>,
    routes: Routes,
    /// Files moved or copied because their folder refused the link, and the
    /// first reason of each, for one log line per apply instead of one per
    /// file.
    moved: usize,
    move_reason: Option<String>,
    copied: usize,
    copy_reason: Option<String>,
}

impl Publications {
    fn new(data: PathBuf) -> Self {
        Self {
            data,
            folders: BTreeSet::new(),
            vacated: BTreeSet::new(),
            routes: Routes::default(),
            moved: 0,
            move_reason: None,
            copied: 0,
            copy_reason: None,
        }
    }

    /// Owes a sync to `published`'s folder and to those above it inside the
    /// data folder.
    fn published(&mut self, published: &Path) {
        let mut folder = published.parent();
        while let Some(current) = folder {
            if !current.starts_with(&self.data) || !self.folders.insert(current.to_owned()) {
                break;
            }
            folder = current.parent();
        }
    }

    /// Records what publishing `entry` to `destination` did. `pack_md5` is
    /// the target's md5 of the entry.
    async fn record(
        &mut self,
        entry: &mut apply_plan::PlanEntry,
        publication: Publication,
        destination: &Path,
        pack_md5: [u8; 16],
        notes: &mut Vec<String>,
    ) -> anyhow::Result<()> {
        use apply_plan::{PlanAction, PlanReason};
        match publication {
            Publication::Linked => self.published(destination),
            Publication::Moved { reason, vacated } => {
                self.published(destination);
                self.vacated.insert(vacated);
                self.moved += 1;
                self.move_reason.get_or_insert(reason);
            }
            // copy_staged synced the copy's bytes, not its folder.
            Publication::Copied { reason } => {
                self.published(destination);
                self.copied += 1;
                self.copy_reason.get_or_insert(reason);
            }
            Publication::Kept => {
                entry.action = PlanAction::Keep;
                entry.reason = PlanReason::PreservedExisting;
            }
            // Whatever the user made of the file in the meantime stays: an
            // edit, a replacement or a deletion.
            Publication::SourceChanged => {
                entry.action = PlanAction::Keep;
                entry.reason = match disk_scan::probe_md5(destination).await? {
                    Some(current) => PlanReason::ModifiedByUser {
                        original: pack_md5,
                        current,
                    },
                    None => PlanReason::DeletedByUser,
                };
                let note = source_changed_note(&entry.path);
                tracing::warn!("{note}");
                notes.push(note);
            }
        }
        Ok(())
    }

    /// Makes every published entry durable. The caller records Applied next,
    /// and committing then deletes the staged names.
    async fn finish(self) -> anyhow::Result<()> {
        if let Some(reason) = &self.move_reason {
            tracing::info!(
                "Moved {} staged files instead of linking them ({reason})",
                self.moved
            );
        }
        if let Some(reason) = &self.copy_reason {
            tracing::info!(
                "Copied {} staged files instead of linking or moving them ({reason})",
                self.copied
            );
        }
        for folder in self.folders.iter().chain(&self.vacated) {
            sync_linked_folder(folder).await?;
        }
        // The plain fsyncs above handed every folder to its drive in order;
        // one full flush of each drive's cache makes them all durable. That
        // is one drive unless a folder is linked to another one, which got
        // copies.
        #[cfg(target_vendor = "apple")]
        {
            use std::os::unix::fs::MetadataExt;
            let mut drives = HashSet::new();
            for folder in self.folders.iter().chain(&self.vacated) {
                if drives.insert(tokio::fs::metadata(folder).await?.dev()) {
                    File::open(folder).await?.sync_all().await?;
                }
            }
        }
        Ok(())
    }
}

/// One folder of the publication barrier: [`fsync_dir`], except on Apple
/// platforms. There std's `sync_all` is F_FULLFSYNC, a flush of the whole
/// drive cache on every call, so each folder gets a plain fsync instead and
/// [`Publications::finish`] flushes each drive once at the end, as SQLite
/// does.
#[cfg(not(target_vendor = "apple"))]
async fn sync_linked_folder(dir: &Path) -> anyhow::Result<()> {
    fsync_dir(dir).await
}

#[cfg(target_vendor = "apple")]
async fn sync_linked_folder(dir: &Path) -> anyhow::Result<()> {
    #[cfg(test)]
    {
        staging_test::record_sync(dir);
        staging_test::run_hook("sync-directory", dir);
        staging_test::io_fault("sync-directory", dir)?;
    }
    let dir = dir.to_owned();
    tokio::task::spawn_blocking(move || -> io::Result<()> {
        use std::os::fd::AsRawFd;
        let folder = std::fs::File::open(&dir)?;
        // SAFETY: `folder` owns the descriptor and outlives the call.
        if unsafe { libc::fsync(folder.as_raw_fd()) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    })
    .await??;
    Ok(())
}

// This records one representation change in the existing setup. It is not
// setup admission or a session identifier. Task8 binds this consumer to its
// sole versioned session; target/options binding already rejects re-use by
// a changed request. No defaults turn legacy absence into ownership.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RepresentationTransition {
    schema_version: u32,
    owner: Option<Owner>,
    target_digest: String,
    options_digest: String,
    recovery_batch: String,
    entry: apply_plan::PlanEntry,
    disabled: bool,
    destination_before: Option<String>,
    opposite_before: Option<String>,
    published: bool,
    completed: bool,
    conflict: bool,
}

fn transition_binding(
    target: &packinfo::PackInfo,
    options: Option<&RepairMarkerFile>,
) -> anyhow::Result<(String, String)> {
    session::binding(target, options)
}

/// Names a path's representation record and its recovery entries (both the
/// representation manifest and the cleanup entry). 32 hex characters, the
/// first 16 bytes of the path's SHA-512, keep
/// `.repair-recovery/<batch>/<key>/entry.json` about 76 characters past the
/// instance root, short enough for Explorer, the Recycle Bin and backup tools
/// on long instance names. Every record stores the full path and is refused
/// on a mismatch, so the key only has to spread paths apart.
fn record_key(path: &str) -> String {
    use sha2::Digest;
    hex::encode(&sha2::Sha512::digest(path.as_bytes())[..16])
}

fn transition_path(root: &Path, path: &str) -> PathBuf {
    root.join(".setup/representation-transitions")
        .join(format!("{}.json", record_key(path)))
}

fn transition_batch(root: &Path, record: &RepresentationTransition) -> PathBuf {
    root.join(".repair-recovery").join(&record.recovery_batch)
}

fn check_plain_directory(path: &Path) -> anyhow::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("expected plain directory: {}", path.display());
    }
    Ok(())
}

async fn ensure_plain_directory(path: &Path) -> anyhow::Result<()> {
    match tokio::fs::create_dir(path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => check_plain_directory(path),
        Err(e) => Err(e.into()),
    }
}

fn validate_transition(
    record: &RepresentationTransition,
    binding: &(String, String),
    target: &packinfo::PackInfo,
    owner: Option<&Owner>,
) -> anyhow::Result<()> {
    use apply_plan::PlanAction;
    let mut batch_components = Path::new(&record.recovery_batch).components();
    let valid_batch = matches!(
        batch_components.next(),
        Some(std::path::Component::Normal(_))
    ) && batch_components.next().is_none();
    if !valid_batch
        || match owner {
            Some(owner) => record.schema_version != 2 || record.owner.as_ref() != Some(owner),
            None => record.schema_version != 1 || record.owner.is_some(),
        }
        || record.target_digest != binding.0
        || record.options_digest != binding.1
        || normalize_cleanup_path(&record.entry.path).as_ref() != Some(&record.entry.path)
        || !target.files.contains_key(&record.entry.path)
        || !matches!(
            (&record.entry.action, record.disabled),
            (PlanAction::Replace, true) | (PlanAction::ReEnable, false)
        )
        || record.completed && !record.published && !record.conflict
        || [&record.destination_before, &record.opposite_before]
            .iter()
            .any(|hash| {
                hash.as_ref()
                    .is_some_and(|hash| hex::decode(hash).map_or(true, |bytes| bytes.len() != 64))
            })
    {
        bail!("representation transition does not match authoritative target/original options");
    }
    Ok(())
}

// This pass requests original inputs only; it grants no apply authority.
// Complete target/options/archive/manifest validation still follows extraction.
async fn reconstruction_source_requirements(
    root: &Path,
    current: Option<&Session>,
    copied: Option<&Session>,
) -> anyhow::Result<BTreeSet<String>> {
    let mut paths = BTreeSet::new();
    let directory = root.join(".setup/representation-transitions");
    if !directory.try_exists()? {
        return Ok(paths);
    }
    check_plain_directory(&directory)?;
    let mut files = tokio::fs::read_dir(&directory).await?;
    while let Some(file) = files.next_entry().await? {
        if file.path().extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let record = read_transition(&file.path()).await?;
        if normalize_cleanup_path(&record.entry.path).as_ref() != Some(&record.entry.path)
            || file.path() != transition_path(root, &record.entry.path)
        {
            bail!("transition source requirement has invalid path provenance");
        }
        let known_owner = record.owner.as_ref().is_some_and(|owner| {
            current.and_then(|s| s.owner().ok()).as_ref() == Some(owner)
                || current.and_then(|s| s.adopted_from.as_ref()) == Some(owner)
                || copied.and_then(|s| s.owner().ok()).as_ref() == Some(owner)
        });
        if !(record.schema_version == 1
            && record.owner.is_none()
            && current.is_none_or(|s| s.producer == Producer::Reconstructed)
            || record.schema_version == 2 && known_owner)
        {
            bail!("transition source requirement has foreign ownership; preserved");
        }
        paths.insert(record.entry.path);
    }
    Ok(paths)
}

/// Adopt only after full original-input reconstruction. Both the mutable
/// intent and immutable retained manifest carry the new owner. An interrupted
/// two-file adoption is recognized by comparing the same original contents;
/// no live mutation runs until all entries are settled.
async fn adopt_legacy_transitions(
    app: &Arc<AppInner>,
    root: &Path,
    session: &Session,
) -> anyhow::Result<()> {
    let directory = root.join(".setup/representation-transitions");
    if !directory.try_exists()? {
        return Ok(());
    }
    check_plain_directory(&directory)?;
    let bound = session
        .target
        .as_ref()
        .context("transition adoption lacks full target")?;
    let target = packinfo::parse_packinfo(&bound.packinfo)?;
    let binding = transition_binding(&target, session.original_options.as_ref())?;
    let owner = session.owner()?;
    let mut entries = tokio::fs::read_dir(&directory).await?;
    while let Some(file) = entries.next_entry().await? {
        if file.path().extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let mut record = read_transition(&file.path()).await?;
        if record.schema_version == 2 && record.owner.as_ref() == Some(&owner) {
            validate_transition(&record, &binding, &target, Some(&owner))?;
            continue;
        }
        if session.producer != Producer::Reconstructed {
            bail!("unowned legacy transition in current setup; retain original inputs");
        }
        let archive = session
            .archive
            .as_ref()
            .context("legacy adoption requires original archive provenance")?;
        let hash = crate::managers::instance::modpack::origin_check::hash_file_sha512(
            &root.join(".setup").join(&archive.name),
        )
        .await?;
        if hex::encode(hash) != archive.sha512 {
            bail!("legacy original archive changed");
        }
        if record.schema_version == 2 {
            let previous = session
                .adopted_from
                .as_ref()
                .context("foreign representation owner has no qualified copy provenance")?;
            validate_transition(&record, &binding, &target, Some(previous))?;
        } else {
            validate_transition(&record, &binding, &target, None)?;
        }
        if file.path() != transition_path(root, &record.entry.path) {
            bail!("legacy transition filename mismatch");
        }
        let spelling = bound
            .sources
            .get(&record.entry.path)
            .context("legacy transition has no reconstructed original source")?;
        if record.disabled != spelling.ends_with(".disabled")
            && record.entry.action != apply_plan::PlanAction::ReEnable
        {
            bail!("legacy transition differs from original source representation");
        }
        let manifest = transition_batch(root, &record)
            .join(record_key(&record.entry.path))
            .join("entry.json");
        let mut initial = record.clone();
        initial.published = false;
        initial.completed = false;
        initial.conflict = false;
        let mut adopted_initial = initial.clone();
        adopted_initial.schema_version = 2;
        adopted_initial.owner = Some(owner.clone());
        if manifest.try_exists()? {
            let saved = read_transition(&manifest).await?;
            if saved != initial && saved != adopted_initial {
                bail!("legacy immutable recovery manifest does not match its intent");
            }
            write_transition(&manifest, &adopted_initial, false).await?;
            #[cfg(test)]
            app.instance_manager
                .test_hooks
                .pause(
                    "setup_transition_manifest_saved",
                    InstanceId(owner.instance_id),
                )
                .await;
        } else if transition_batch(root, &record)
            .join(record_key(&record.entry.path))
            .join("payload")
            .try_exists()?
        {
            bail!("legacy recovery payload has no manifest; retained for recovery");
        }
        record.schema_version = 2;
        record.owner = Some(owner.clone());
        write_transition(&file.path(), &record, false).await?;
    }
    Ok(())
}

async fn read_transition(path: &Path) -> anyhow::Result<RepresentationTransition> {
    let metadata = tokio::fs::symlink_metadata(path).await?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!(
            "representation transition is not a plain file: {}",
            path.display()
        );
    }
    Ok(serde_json::from_slice(&tokio::fs::read(path).await?)?)
}

async fn write_transition(
    path: &Path,
    record: &RepresentationTransition,
    new: bool,
) -> anyhow::Result<()> {
    #[cfg(test)]
    staging_test::io_fault("transition-record", path)?;
    let bytes = serde_json::to_vec_pretty(record)?;
    let path = path.to_owned();
    let parent = path
        .parent()
        .context("transition path has no parent")?
        .to_owned();
    let sync_parent = parent.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        use std::io::Write;
        let mut private = publish::temp_in(&parent)?;
        private.write_all(&bytes)?;
        private.as_file().sync_all()?;
        if new {
            publish::persist_noclobber(private, &path)?;
        } else {
            publish::persist(private, &path)?;
        }
        Ok(())
    })
    .await??;
    fsync_dir(&sync_parent).await
}

async fn representation_hash(path: &Path) -> anyhow::Result<Option<String>> {
    match tokio::fs::symlink_metadata(path).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
        Ok(m) if m.is_file() && !m.file_type().is_symlink() => Ok(Some(hex::encode(
            crate::managers::instance::modpack::origin_check::hash_file_sha512(path).await?,
        ))),
        _ => bail!(
            "representation path is not a plain file: {}",
            path.display()
        ),
    }
}

/// Owned transitions take precedence over fresh disk-derived decisions.
/// Exclude them before pure planning, so resume never reinterprets the
/// completed twin as a newly disabled file or demands a consumed legacy input.
async fn plan_staging(
    inputs: apply_plan::PlanInputs<'_>,
    root: &Path,
    options: Option<&RepairMarkerFile>,
    owner: Option<&Owner>,
) -> anyhow::Result<Vec<apply_plan::PlanEntry>> {
    if let Some(owner) = owner {
        Session::require(root, owner).await?;
    }
    let directory = root.join(".setup/representation-transitions");
    if !directory.try_exists()? {
        return Ok(apply_plan::plan(inputs)?);
    }
    check_plain_directory(&directory)?;
    let binding = transition_binding(inputs.target, options)?;
    let mut resumed = std::collections::BTreeMap::new();
    let mut records = tokio::fs::read_dir(&directory).await?;
    while let Some(file) = records.next_entry().await? {
        // An interrupted private tempfile is diagnostic debris, never intent.
        if file.path().extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let record = read_transition(&file.path()).await.with_context(|| {
            format!("preserved representation record: {}", file.path().display())
        })?;
        validate_transition(&record, &binding, inputs.target, owner).with_context(|| {
            format!("preserved representation record: {}", file.path().display())
        })?;
        if file.path() != transition_path(root, &record.entry.path) {
            bail!(
                "foreign representation transition filename: {}",
                file.path().display()
            );
        }
        resumed.insert(record.entry.path.clone(), record.entry);
    }
    let excluded = resumed.keys().cloned().collect();
    let mut entries = apply_plan::plan_remaining(inputs, &excluded)?;
    entries.extend(resumed.into_values());
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

/// `binding` is [`transition_binding`] of `target` and the apply's options,
/// computed once per apply: it covers the whole target.
async fn execute_representation(
    entry: &mut apply_plan::PlanEntry,
    root: &Path,
    staged: &Path,
    target: &packinfo::PackInfo,
    binding: &(String, String),
    notes: &mut Vec<String>,
    owner: Option<&Owner>,
) -> anyhow::Result<()> {
    // `owner` binds the transition records. execute_bound_plan has already
    // checked that it still owns the setup.
    use apply_plan::{PlanAction, PlanReason};
    let path = transition_path(root, &entry.path);
    let bare = root.join("instance").join(
        entry
            .path
            .strip_prefix('/')
            .context("invalid representation path")?,
    );
    let twin = disabled_sibling(&bare);
    let mut record = if path.try_exists()? {
        let record = read_transition(&path).await?;
        validate_transition(&record, binding, target, owner)?;
        if record.entry.path != entry.path {
            bail!("representation path mismatch");
        }
        record
    } else {
        let disabled = entry.action == PlanAction::Replace;
        let (dest, other) = if disabled {
            (&twin, &bare)
        } else {
            (&bare, &twin)
        };
        let record = RepresentationTransition {
            schema_version: if owner.is_some() { 2 } else { 1 },
            owner: owner.cloned(),
            target_digest: binding.0.clone(),
            options_digest: binding.1.clone(),
            recovery_batch: crate::managers::instance::modpack::create_cleanup_batch(root)?,
            entry: entry.clone(),
            disabled,
            destination_before: representation_hash(dest).await?,
            opposite_before: representation_hash(other).await?,
            published: false,
            completed: false,
            conflict: false,
        };
        validate_transition(&record, binding, target, owner)?;
        ensure_plain_directory(path.parent().unwrap()).await?;
        fsync_dir(&root.join(".setup")).await?;
        write_transition(&path, &record, true).await?;
        record
    };
    let (dest, other) = if record.disabled {
        (&twin, &bare)
    } else {
        (&bare, &twin)
    };
    let expected = target
        .files
        .get(&record.entry.path)
        .context("transition target missing")?
        .sha512;
    let batch = transition_batch(root, &record);
    let recovery_entry = batch.join(record_key(&entry.path));
    let payload = recovery_entry.join("payload");
    let expected_hex = Some(hex::encode(expected));
    let current = representation_hash(dest).await?;
    let mut source_changed = false;
    if record.completed {
        // Never collect another live occupant after completion. The payload
        // remains the original file, including later writes through open handles.
        if current != expected_hex
            || (!record.conflict && representation_hash(other).await?.is_some())
        {
            record.conflict = true;
            write_transition(&path, &record, false).await?;
        }
        // A conflict that published nothing and moved nothing may be one an
        // earlier attempt found from a changed source (see below). The record
        // does not keep the reason, so a resumed apply checks the source
        // again to give the same plain note. Only the note depends on it.
        if record.conflict && !record.published && !payload.try_exists()? {
            let source = resolve_staged(staged)
                .map(|(path, _)| path)
                .or_else(|| (!record.disabled).then(|| other.clone()));
            if let Some(source) = source.filter(|source| source.is_file()) {
                source_changed =
                    crate::managers::instance::modpack::origin_check::hash_file_sha512(&source)
                        .await
                        .is_ok_and(|hash| hash != expected);
            }
        }
    } else {
        ensure_plain_directory(&root.join(".repair-recovery")).await?;
        ensure_plain_directory(&batch).await?;
        sync_recovery_hierarchy(root, &batch).await?;
        ensure_plain_directory(&recovery_entry).await?;
        let manifest = recovery_entry.join("entry.json");
        if manifest.try_exists()? {
            let saved = read_transition(&manifest).await?;
            let mut initial = record.clone();
            initial.published = false;
            initial.completed = false;
            initial.conflict = false;
            if saved != initial {
                bail!("retained representation manifest mismatch");
            }
        } else {
            if payload.try_exists()? {
                bail!("retained representation payload has no manifest");
            }
            let mut initial = record.clone();
            initial.published = false;
            initial.completed = false;
            initial.conflict = false;
            write_transition(&manifest, &initial, true).await?;
        }
        fsync_dir(&batch).await?;
        if current != expected_hex {
            if record.published || current != record.destination_before {
                // A destination edited after intent was recorded is preserved.
                record.conflict = true;
                record.completed = true;
            } else {
                let source = resolve_staged(staged)
                    .map(|(path, _)| path)
                    .or_else(|| (!record.disabled).then(|| other.clone()))
                    .ok_or_else(|| missing_staged_error(&entry.path))?;
                tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
                match publish_staged_copy(&source, dest, Some(expected), false).await {
                    Ok(_) => {}
                    // The source no longer holds the pack's bytes: an earlier
                    // attempt linked it into the instance and it was edited
                    // there in place. Both live files stay as they are.
                    Err(error) if error.is::<StagedSourceChanged>() => {
                        tracing::warn!("{error:#}");
                        source_changed = true;
                        record.conflict = true;
                        record.completed = true;
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        if !record.completed {
            record.published = true;
            write_transition(&path, &record, false).await?;
            if !payload.try_exists()? && representation_hash(other).await?.is_some() {
                #[cfg(test)]
                staging_test::run_hook(
                    if record.disabled {
                        "remove-bare"
                    } else {
                        "remove-twin"
                    },
                    other,
                );
                // Retain the original inode rather than deleting a hash-matched
                // pathname. A concurrent edit remains recoverable even after
                // our post-move hash or through an already-open file handle.
                // A mods folder linked to another drive than the instance root
                // gets a published copy instead, and the hash below still
                // catches an edit made before it.
                publish::move_noclobber_async(other, &payload).await?;
            }
            if payload.try_exists()? {
                sync_recovery_payload(&payload).await?;
                fsync_dir(&recovery_entry).await?;
                fsync_dir(other.parent().unwrap()).await?;
                let moved = representation_hash(&payload).await?;
                if moved != record.opposite_before {
                    record.conflict = true;
                    let moved_hash =
                        crate::managers::instance::modpack::origin_check::hash_file_sha512(
                            &payload,
                        )
                        .await?;
                    publish_staged_copy(&payload, other, Some(moved_hash), true).await?;
                } else if representation_hash(other).await?.is_some() {
                    record.conflict = true;
                }
            }
            record.completed = true;
        }
        #[cfg(test)]
        staging_test::io_fault("transition-complete", &path)?;
        write_transition(&path, &record, false).await?;
    }
    *entry = record.entry.clone();
    if record.conflict {
        entry.action = PlanAction::Keep;
        entry.reason = PlanReason::PreservedExisting;
        let location = payload.strip_prefix(root).unwrap_or(&payload).display();
        let note = if source_changed {
            source_changed_note(&entry.path)
        } else if payload.try_exists()? {
            format!(
                "Representation conflict preserved for {}; recovery: {location}",
                entry.path
            )
        } else {
            format!(
                "Representation conflict preserved for {}; live files preserved, no counterpart moved",
                entry.path
            )
        };
        tracing::warn!("{note}");
        notes.push(note);
    } else if payload.try_exists()? {
        tracing::info!("Representation original retained: {}", payload.display());
    }
    Ok(())
}

#[cfg(test)]
pub(in crate::managers::instance) async fn execute_plan(
    entries: &[apply_plan::PlanEntry],
    root: &Path,
    staging: &Path,
    target: &packinfo::PackInfo,
) -> anyhow::Result<Vec<apply_plan::PlanEntry>> {
    execute_bound_plan(
        entries,
        root,
        staging,
        target,
        None,
        &mut Vec::new(),
        None,
        &Default::default(),
    )
    .await
}

/// Carries out every [`apply_plan::PlanEntry`] against the real filesystem.
/// Executes fresh decisions and resumes owned representation transitions;
/// publication collisions and preserved conflicts become actual audit outcomes.
/// `sources` is what the pre-apply source check found: files an interrupted
/// earlier attempt already linked, and staged sources that must not be
/// published because they changed since.
#[allow(clippy::too_many_arguments)]
async fn execute_bound_plan(
    entries: &[apply_plan::PlanEntry],
    instance_root: &Path,
    staging_dir: &Path,
    target: &packinfo::PackInfo,
    options: Option<&RepairMarkerFile>,
    notes: &mut Vec<String>,
    owner: Option<&Owner>,
    sources: &session::SourceCheck,
) -> anyhow::Result<Vec<apply_plan::PlanEntry>> {
    #[cfg(not(test))]
    if owner.is_none() {
        bail!("live apply requires an admitted setup owner");
    }
    use apply_plan::{PlanAction, PlanReason};
    // One full ownership check per apply, not one per file: the caller holds
    // the instance gate for the whole apply. Before each change a stat of the
    // record stands in for reloading it.
    let mut ownership = match owner {
        Some(owner) => Some(session::OwnershipWatch::begin(instance_root, owner).await?),
        None => None,
    };
    // What every representation record binds to: the whole target, so once
    // per apply rather than once per entry.
    let binding = transition_binding(target, options)?;
    let data_path = instance_root.join("instance");
    let mut outcomes = entries.to_vec();
    let mut publications = Publications::new(data_path.clone());
    // The attempt that linked or moved these may have stopped before its
    // barrier. Whatever this one plans for them, their staged names go at
    // commit.
    for published in &sources.published {
        publications.published(published);
    }
    for entry in &mut outcomes {
        // `strip_prefix` rather than `[1..]`: char-boundary-safe (an empty
        // key or one starting with a multibyte character would panic on a
        // byte-index slice), and it gives `None` instead of a garbage
        // substring for a key that doesn't actually start with '/'.
        let rel = entry
            .path
            .strip_prefix('/')
            .ok_or_else(|| anyhow!("packinfo key '{}' must start with '/'", entry.path))?;
        // Re-checked here rather than trusted from `parse_packinfo`: a `..`
        // segment survives `Path::join` as a literal component, and
        // `Path::starts_with` below compares components lexically without
        // ever resolving `..` — so "mods/../../evil" joined onto the data
        // dir would otherwise pass the containment check below even though
        // the real, OS-resolved path escapes the data dir entirely.
        if packinfo::has_dotdot_segment(&entry.path) {
            bail!("packinfo key '{}' contains a '..' path segment", entry.path);
        }
        let live = instance_root.join("instance").join(rel);
        let staged = staging_dir.join("instance").join(rel);
        // Belt-and-braces even though `parse_packinfo` already rejects a
        // doubled leading '/': `Path::join` with an absolute argument
        // REPLACES the base entirely, so a key like "//tmp/esc" (whose tail
        // after stripping one '/' is itself still absolute) would otherwise
        // point `remove_file`/`rename` below at a path outside the instance
        // — never trust a join alone before a destructive op.
        if !live.starts_with(&data_path) {
            bail!(
                "packinfo key '{}' escapes the instance data dir",
                entry.path
            );
        }
        let twin = disabled_sibling(&live);
        if disk_scan::resolves_into_saves(&data_path, &live)
            && (live.exists() || twin.exists() || entry.action != PlanAction::Create)
        {
            entry.action = PlanAction::Keep;
            entry.reason = PlanReason::InSaveFolder;
        }
        if entry.action != PlanAction::Keep
            && let Some(ownership) = &mut ownership
        {
            ownership.check().await?;
        }
        if entry.action != PlanAction::Keep
            && (transition_path(instance_root, &entry.path).try_exists()?
                || entry.action == PlanAction::ReEnable
                || entry.action == PlanAction::Replace
                    && resolve_staged(&staged).is_some_and(|(_, disabled)| disabled))
        {
            execute_representation(
                entry,
                instance_root,
                &staged,
                target,
                &binding,
                notes,
                owner,
            )
            .await?;
            continue;
        }
        let pack = target.files.get(&entry.path);
        let expected = pack.map(|h| h.sha512);
        let pack_md5 = pack.map(|h| h.md5).unwrap_or_default();
        // A staged source that changed since the apply started is never
        // published, whichever action needs it.
        let changed = sources.changed.contains(&entry.path);
        match entry.action {
            PlanAction::Keep => {}
            PlanAction::Delete => {
                tokio::fs::remove_file(&live).await?;
            }
            PlanAction::Replace => {
                let (source, is_disabled) =
                    resolve_staged(&staged).ok_or_else(|| missing_staged_error(&entry.path))?;
                debug_assert!(
                    !is_disabled,
                    "disabled Replace uses durable representation intent"
                );
                let publication = if changed {
                    Publication::SourceChanged
                } else {
                    publish_staged(
                        &source,
                        &live,
                        expected.context("missing target hash for staged publication")?,
                        false,
                        &mut publications.routes,
                    )
                    .await?
                };
                publications
                    .record(entry, publication, &live, pack_md5, notes)
                    .await?;
            }
            PlanAction::Create => {
                let (source, is_disabled) =
                    resolve_staged(&staged).ok_or_else(|| missing_staged_error(&entry.path))?;
                // A pack-shipped-disabled path must land disabled, not
                // enabled — the pack's own default is preserved.
                let dest = if is_disabled { &twin } else { &live };
                tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
                // Preserve observed representation intent as well as the
                // destination itself. No-clobber publication closes the
                // destination race after this twin check.
                let other = if is_disabled { &live } else { &twin };
                let publication = if other.try_exists()? {
                    Publication::Kept
                } else if changed {
                    Publication::SourceChanged
                } else {
                    publish_staged(
                        &source,
                        dest,
                        expected.context("missing target hash for staged publication")?,
                        true,
                        &mut publications.routes,
                    )
                    .await?
                };
                let kept = publication == Publication::Kept;
                publications
                    .record(entry, publication, dest, pack_md5, notes)
                    .await?;
                if kept && other == &twin && twin.exists() {
                    entry.reason = PlanReason::DisabledByUser;
                }
            }
            PlanAction::ReplaceDisabled => {
                let (source, _) =
                    resolve_staged(&staged).ok_or_else(|| missing_staged_error(&entry.path))?;
                let publication = if changed {
                    Publication::SourceChanged
                } else {
                    publish_staged(
                        &source,
                        &twin,
                        expected.context("missing target hash for staged publication")?,
                        false,
                        &mut publications.routes,
                    )
                    .await?
                };
                publications
                    .record(entry, publication, &twin, pack_md5, notes)
                    .await?;
            }
            PlanAction::ReEnable => unreachable!("ReEnable uses durable representation intent"),
        }
    }
    publications.finish().await?;
    Ok(outcomes)
}

#[derive(Default)]
struct CleanupReport {
    removed: Vec<String>,
    notes: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CleanupRecoveryEntry {
    schema_version: u32,
    selection: crate::managers::instance::modpack::CleanupSelection,
    payload: String,
    status: String,
}

async fn write_cleanup_record(entry: &Path, record: &CleanupRecoveryEntry) -> anyhow::Result<()> {
    let entry = entry.to_owned();
    let bytes = serde_json::to_vec_pretty(record)?;
    let parent = entry.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        use std::io::Write;
        let mut private = publish::temp_in(&entry)?;
        private.write_all(&bytes)?;
        private.as_file().sync_all()?;
        publish::persist(private, &entry.join("entry.json"))?;
        Ok(())
    })
    .await??;
    fsync_dir(&parent).await
}

async fn finish_cleanup_payload(
    entry: &Path,
    live: &Path,
    record: &mut CleanupRecoveryEntry,
    report: &mut CleanupReport,
) -> anyhow::Result<()> {
    use crate::managers::instance::modpack::origin_check::hash_file_sha512;
    let payload = entry.join("payload");
    sync_recovery_payload(&payload).await?;
    fsync_dir(entry).await?;
    fsync_dir(live.parent().context("cleanup path has no parent")?).await?;
    let moved = hash_file_sha512(&payload).await?;
    if hex::encode(moved) != record.selection.sha512.to_ascii_lowercase() {
        record.status = "preserved-conflict".into();
        let restored = publish_staged_copy(&payload, live, Some(moved), true).await?;
        let note = format!(
            "Cleanup preserved changed payload at {}; restored live copy: {restored}",
            payload.display()
        );
        tracing::warn!("{note}");
        report.notes.push(note);
    } else {
        record.status = "removed-at-user-request".into();
        if !live.try_exists()? {
            report.removed.push(record.selection.path.clone());
        }
        report.notes.push(format!(
            "Cleanup retained {}: {}",
            record.selection.path,
            payload.display()
        ));
    }
    write_cleanup_record(entry, record).await
}

/// Selected bytes are moved into retained storage outside setup and audit.
/// An existing entry is a retry, never consent to take a new live file.
async fn apply_bound_cleanup(
    options: &RepairMarkerFile,
    old: Option<&packinfo::PackInfo>,
    target: &packinfo::PackInfo,
    root: &Path,
    insensitive: bool,
    context: Option<&crate::managers::instance::modpack::PackProofContext>,
    owner: Option<&Owner>,
) -> CleanupReport {
    use crate::managers::instance::modpack::origin_check::hash_file_sha512;
    let mut report = CleanupReport::default();
    if !options.cleanup_paths.is_empty() {
        report
            .notes
            .push("Legacy cleanup paths preserved: no selected content proof.".into());
    }
    if options.cleanup_selections.is_empty() {
        return report;
    }
    if options
        .cleanup_selections
        .iter()
        .any(|s| Some(&s.context) != context)
    {
        report
            .notes
            .push("Cleanup preserved: installed pack context changed.".into());
        return report;
    }
    let Some(batch_name) = options.cleanup_batch.as_ref().filter(|name| {
        let mut components = Path::new(name).components();
        matches!(components.next(), Some(std::path::Component::Normal(_)))
            && components.next().is_none()
    }) else {
        report
            .notes
            .push("Cleanup skipped: no bound recovery batch.".into());
        return report;
    };
    let recovery = root.join(".repair-recovery");
    let batch = recovery.join(batch_name);
    for directory in [&recovery, &batch] {
        if !std::fs::symlink_metadata(directory)
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        {
            report.notes.push(format!(
                "Cleanup skipped: recovery directory unavailable: {}",
                directory.display()
            ));
            return report;
        }
    }
    if let Err(error) = sync_recovery_hierarchy(root, &batch).await {
        report.notes.push(format!(
            "Cleanup skipped: recovery durability prerequisite: {error:#}"
        ));
        return report;
    }
    report
        .notes
        .push(format!("Cleanup recovery: .repair-recovery/{batch_name}/"));
    let data = root.join("instance");
    // Containment is checked on resolved paths; without one nothing is moved.
    let canonical_data = match tokio::fs::canonicalize(&data).await {
        Ok(canonical) => canonical,
        Err(error) => {
            tracing::warn!(
                "skipping repair cleanup: could not resolve the instance data folder {}: {error}",
                data.display()
            );
            report.notes.push(format!(
                "Cleanup skipped: the instance data folder path could not be resolved: {error}"
            ));
            return report;
        }
    };
    let deletable = walk_untracked_files(&data, old, target, insensitive).await;
    #[cfg(not(test))]
    if owner.is_none() {
        report
            .notes
            .push("Cleanup preserved: no admitted setup owner".into());
        return report;
    }
    // Checked in full once; before each selection a stat of the record
    // stands in for reloading it, as in execute_bound_plan.
    let mut ownership = match owner {
        Some(owner) => match session::OwnershipWatch::begin(root, owner).await {
            Ok(watch) => Some(watch),
            Err(error) => {
                report.notes.push(format!(
                    "Cleanup preserved: setup ownership changed: {error:#}"
                ));
                return report;
            }
        },
        None => None,
    };
    for selection in &options.cleanup_selections {
        if let Some(ownership) = &mut ownership
            && let Err(error) = ownership.check().await
        {
            report.notes.push(format!(
                "Cleanup preserved: setup ownership changed: {error:#}"
            ));
            break;
        }
        let key = &selection.path;
        let mut expected = [0u8; 64];
        if normalize_cleanup_path(key).as_ref() != Some(key)
            || hex::decode_to_slice(&selection.sha512, &mut expected).is_err()
        {
            continue;
        }
        let entry = batch.join(record_key(key));
        let payload = entry.join("payload");
        // Do not reuse content-only ownership. The persisted entry belongs
        // to this selected path in this admitted batch; never move twice.
        if entry.exists() {
            let resumed = async {
                let mut record: CleanupRecoveryEntry =
                    serde_json::from_slice(&tokio::fs::read(entry.join("entry.json")).await?)?;
                if record.schema_version != 1
                    || record.selection != *selection
                    || record.payload != "payload"
                {
                    bail!("recovery entry does not match the admitted selection");
                }
                if !entry.symlink_metadata()?.is_dir()
                    || entry.symlink_metadata()?.file_type().is_symlink()
                {
                    bail!("recovery entry is not a plain directory");
                }
                let live = data.join(key.strip_prefix('/').unwrap());
                if record.status == "pending" && payload.is_file() {
                    let parent = live.parent().context("cleanup path has no parent")?;
                    if !tokio::fs::canonicalize(parent)
                        .await?
                        .starts_with(&canonical_data)
                        || disk_scan::resolves_into_saves(&data, &live)
                    {
                        bail!("cleanup recovery destination no longer eligible");
                    }
                    finish_cleanup_payload(&entry, &live, &mut record, &mut report).await?;
                }
                Ok::<_, anyhow::Error>(())
            }
            .await;
            report.notes.push(format!(
                "Cleanup retry retained recovery entry: {}{}",
                entry.display(),
                resumed
                    .err()
                    .map(|e| format!(" ({e:#})"))
                    .unwrap_or_default()
            ));
            continue;
        }
        let Some(live) = deletable.get(key) else {
            continue;
        };
        let Some(parent) = live.parent() else {
            continue;
        };
        if !tokio::fs::canonicalize(parent)
            .await
            .is_ok_and(|p| p.starts_with(&canonical_data))
            || disk_scan::resolves_into_saves(&data, live)
        {
            continue;
        }
        if hash_file_sha512(live).await.ok() != Some(expected) {
            report
                .notes
                .push(format!("Cleanup preserved changed file: {key}"));
            continue;
        }
        // create_dir is exclusive. Its deterministic per-selection name is
        // storage within this unique batch, not a setup admission marker.
        if let Err(e) = tokio::fs::create_dir(&entry).await {
            report.notes.push(format!("Cleanup skipped {key}: {e}"));
            continue;
        }
        let mut record = CleanupRecoveryEntry {
            schema_version: 1,
            selection: selection.clone(),
            payload: "payload".into(),
            status: "pending".into(),
        };
        let result = async {
            write_cleanup_record(&entry, &record).await?;
            fsync_dir(&batch).await?;
            #[cfg(test)]
            staging_test::run_hook("cleanup-move", live);
            // Copied when the file's folder is linked to another drive than
            // the instance root; the payload is new in this batch.
            publish::move_noclobber_async(live, &payload).await?;
            #[cfg(test)]
            staging_test::run_hook("cleanup-moved", live);
            sync_recovery_payload(&payload).await?;
            fsync_dir(&entry).await?;
            fsync_dir(parent).await?;
            finish_cleanup_payload(&entry, live, &mut record, &mut report).await
        }
        .await;
        if let Err(error) = result {
            let note = format!(
                "Cleanup preserved/skipped {key}: {error:#}; recovery: {}",
                payload.display()
            );
            tracing::warn!("{note}");
            report.notes.push(note);
        }
    }
    report
}

/// Resolves the physical staged file backing a plan entry, which may sit
/// under its bare name or — when the target ships this path disabled by
/// default — under the `.disabled`-suffixed name. `packinfo::scan_dir`
/// strips that suffix when keying `tmp-packinfo.json`
/// (`packinfo/scan.rs:30-32`), so a [`apply_plan::PlanEntry::path`] never
/// carries it even when the only staged copy does; this is where that gets
/// reconciled against what is physically on disk. The bare spelling wins
/// when (implausibly) both exist. `Some((path, true))` means the resolved
/// copy is the disabled spelling.
fn resolve_staged(staged_bare: &Path) -> Option<(PathBuf, bool)> {
    if staged_bare.is_file() {
        return Some((staged_bare.to_path_buf(), false));
    }
    let twin = disabled_sibling(staged_bare);
    twin.is_file().then_some((twin, true))
}

/// The planner already required a staged source to exist (via the same
/// path-normalised `staged` set) before choosing an action that needs one,
/// so `resolve_staged` failing here means that invariant broke, not a
/// normal runtime condition — still handled as a proper error rather than a
/// panic, since a real filesystem is involved.
fn missing_staged_error(path: &str) -> anyhow::Error {
    anyhow!(
        "planner chose an action requiring a staged source for {path}, but neither the bare nor \
         .disabled-suffixed staged copy exists on disk"
    )
}

/// `<name>` -> `<name>.disabled`, the on-disk convention for a disabled mod.
fn disabled_sibling(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap().to_owned();
    name.push(".disabled");
    path.with_file_name(name)
}

fn render_cleanup_audit(entries: &[apply_plan::PlanEntry], cleanup: &CleanupReport) -> String {
    let mut audit = render_audit(entries, &cleanup.removed);
    if !cleanup.notes.is_empty() {
        let after_title = audit.find('\n').map(|i| i + 1).unwrap_or(audit.len());
        audit.insert_str(after_title, &format!("{}\n", cleanup.notes.join("\n")));
    }
    audit
}

/// Renders the plain-text install audit `apps/desktop/e2e-tests/helpers/installAudit.ts`
/// parses. Pure and unit-tested (`staging_test::render_audit_golden`) — every
/// byte of an existing section is a format contract with that parser, so a
/// change here must stay in lockstep with it. `entries` are expected
/// path-sorted (guaranteed by [`apply_plan::plan`]'s own output), which is
/// what makes each section's line order deterministic.
fn render_audit(entries: &[apply_plan::PlanEntry], user_removed: &[String]) -> String {
    use apply_plan::{PlanAction, PlanReason};
    let mut skipped = String::new();
    let mut deleted = String::new();
    let mut replaced = String::new();
    let mut created = String::new();
    let mut unchanged = String::new();
    let mut re_enabled = String::new();

    for e in entries {
        let file = &e.path;
        match (&e.action, &e.reason) {
            (PlanAction::Keep, PlanReason::DeletedByUser) => {
                skipped += &format!(" - {file}: deleted by user\n")
            }
            (PlanAction::Keep, PlanReason::ModifiedByUser { original, current })
            | (PlanAction::Keep, PlanReason::DroppedButModified { original, current }) => {
                skipped += &format!(
                    " - {file}: modified by user\n     original md5: {}\n     current md5:  {}\n",
                    hex::encode(original),
                    hex::encode(current),
                )
            }
            (PlanAction::Keep, PlanReason::InSaveFolder) => {
                skipped += &format!(" - {file}: files in /saves will never be modified\n")
            }
            (PlanAction::Keep, PlanReason::DisabledByUser) => {
                skipped += &format!(" - {file}: disabled by user\n")
            }
            (PlanAction::Keep, PlanReason::PreservedExisting) => {
                skipped += &format!(" - {file}: already present\n")
            }
            // `surviving_path` is deliberately not interpolated into this
            // line: `installAudit.ts`'s `skipped` section parses the text
            // after the last ": " as an exact match against a closed
            // `REASONS` map, so the reason string has to be fixed, the same
            // way `ModifiedByUser`'s variable md5s live on separate
            // continuation lines rather than in the reason text itself.
            // Belongs in `skipped`, not `unchanged`: `installAudit.ts`
            // documents `unchanged` as "already matched the target", which a
            // suppressed Delete did not — the file was never touched, but
            // its old spelling and the target's spelling genuinely differ.
            (PlanAction::Keep, PlanReason::CaseAliasedByTarget { .. }) => {
                skipped += &format!(" - {file}: case-aliased with a tracked path\n")
            }
            (PlanAction::Keep, _) => unchanged += &format!(" - {file}\n"),
            (PlanAction::Delete, _) => deleted += &format!(" - {file}\n"),
            (PlanAction::Replace, _) | (PlanAction::ReplaceDisabled, _) => {
                replaced += &format!(" - {file}\n")
            }
            (PlanAction::Create, _) => created += &format!(" - {file}\n"),
            (PlanAction::ReEnable, _) => re_enabled += &format!(" - {file}\n"),
        }
    }

    let mut audit = "GDLauncher Modpack Install/Update Audit\n".to_string();
    audit += "\nFiles that could not be replaced:\n";
    audit += &skipped;
    audit += "\nFiles deleted:\n";
    audit += &deleted;
    audit += "\nFiles replaced:\n";
    audit += &replaced;
    audit += "\nFiles created:\n";
    audit += &created;
    audit += "\nFiles unchanged:\n";
    audit += &unchanged;
    audit += "\nFiles re-enabled:\n";
    audit += &re_enabled;
    audit += "\nFiles removed at user request:\n";
    for file in user_removed {
        audit += &format!(" - {file}\n");
    }
    audit
}

#[cfg(test)]
#[path = "staging_test.rs"]
pub(in crate::managers::instance) mod staging_test;
