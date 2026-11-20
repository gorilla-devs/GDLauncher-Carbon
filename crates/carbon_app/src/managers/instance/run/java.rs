use super::modpack::TSubtasks;
use crate::App;
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
use crate::managers::instance::modpack::{PackVersionFile, packinfo};
use crate::managers::instance::schema::make_instance_config;
use crate::managers::java::invalidate_java_lists;
use crate::managers::java::java_checker::{JavaChecker, RealJavaChecker};
use crate::managers::java::managed::{AlreadyInstalling, Step};
use crate::managers::java::resolve::{
    AutomaticPick, JavaRequirement, OverrideChoice, accepted_requirements, effective_profile,
    fallback_java, floor_major, is_missing_managed_java, javas_from_rows,
    link_requirement_profiles, need_requirement, override_choice, requirement_for,
    resolve_automatic_requirement,
};
use crate::managers::minecraft::assets::get_assets_dir;
use crate::managers::minecraft::minecraft::get_lwjgl_meta;
use crate::managers::minecraft::modrinth;
use crate::managers::minecraft::{UpdateValue, curseforge};
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::managers::modplatforms::modrinth::convert_mr_version_to_standard_version;
use crate::managers::vtask::Subtask;
use crate::util::NormalizedWalkdir;
use crate::{
    domain::instance::info::{GameVersion, ModLoader, ModLoaderType},
    managers::{
        self, ManagerRef,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
    },
};
use anyhow::{Context, Error, anyhow, bail};
use carbon_net::{DownloadOptions, Downloadable};
use carbon_parsing::log::{LogParser, ParsedItem};
use carbon_repos::repos::java as java_repo;
use chrono::{DateTime, Local, Utc};
use daedalus::minecraft::VersionInfo;
use futures::Future;
use md5::{Digest, Md5};
use std::collections::HashSet;
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
use tracing::{debug, info, trace};

pub async fn check_and_install(
    app: Arc<AppInner>,
    version_info: &VersionInfo,
    t_subtasks: &TSubtasks,
    version: &StandardVersion,
    java_override: Option<JavaOverride>,
    java_floor: Option<u8>,
    log: Option<&watch::Sender<GameLog>>,
    mut file: Option<&mut File>,
) -> anyhow::Result<JavaComponent> {
    let java_profile = daedalus::minecraft::MinecraftJavaProfile::try_from(
        version_info
            .java_version
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Java version not provided"))?
            .component
            .as_str(),
    )
    .with_context(|| anyhow!("instance java version unsupported"))?;

    // The original metadata profile. The override-ignore comparison below matches against
    // this, because it is the system-profile name the frontend stores for a system override.
    let required_java_system_profile = SystemJavaProfileName::try_from(java_profile)
        .with_context(|| anyhow!("System java version unsupported"))?;

    // The profile actually installed and validated (Forge 1.16.5 needs `LegacyFixed1`). That
    // patched value must NOT leak into the comparison above, or a system override on a 1.16.5
    // Forge instance would stop being auto-managed.
    let effective_java_system_profile = effective_profile(required_java_system_profile, version);

    tracing::debug!(
        "This instance requires system profile: {:?}",
        effective_java_system_profile
    );

    let need = requirement_for(effective_java_system_profile);
    // A Java floor a wrong-Java crash showed replaces Minecraft's need.
    let floor = floor_major(effective_java_system_profile, java_floor);
    let msg = match floor {
        Some(floor) => format!("This instance needs Java {floor} or later"),
        None => format!("Minecraft {} needs {}", version.release, need_text(&need)),
    };

    if let Some(ref mut file) = file {
        if let Some(log) = log {
            log.send_modify(|log| log.add_entry(LogEntry::system_message(msg.clone())));
        }
        file.write_all(format_message_as_log4j_event(&msg).as_bytes())
            .await?;
    }

    let managed_root = app
        .settings_manager()
        .runtime_path
        .get_managed_javas()
        .to_path();
    if let Some(JavaOverride::Path(Some(path))) = &java_override
        && is_missing_managed_java(path, &managed_root)
    {
        let msg = format!(
            "This instance is set to a GDLauncher Java that no longer exists ({path}), so it \
             uses Automatic"
        );
        tracing::warn!("{msg}");
        log_line(log, &mut file, &msg).await;
    }

    let (java, is_override) = launch_java(
        &app,
        required_java_system_profile,
        effective_java_system_profile,
        java_override,
        java_floor,
        t_subtasks,
    )
    .await?;

    // A user-set or imported override is used verbatim. Warn (without blocking, since the
    // override is a deliberate escape hatch) when its major version does not match the
    // profile this instance expects, so the user has a clear reason if the game fails to
    // start. Automatic only ever picks a Java that serves the instance.
    if is_override
        && !accepted_requirements(effective_java_system_profile, java_floor)
            .iter()
            .any(|req| req.accepts_component(&java))
    {
        let needs = match floor {
            Some(floor) => format!("it needs Java {floor} or later"),
            None => format!("Minecraft {} needs {}", version.release, need_text(&need)),
        };
        let warn_msg = format!(
            "Warning: this instance is set to Java {}, but {needs}. Minecraft may fail to \
             start; if it does, set the instance's Java to Automatic.",
            java.version.major,
        );
        tracing::warn!("{warn_msg}");
        // Best-effort: this warning must not block the launch (the override is a deliberate
        // escape hatch).
        log_line(log, &mut file, &warn_msg).await;
    }

    let msg = format!("Using Java: {java:#?}");

    if let Some(file) = file {
        if let Some(log) = log {
            log.send_modify(|log| log.add_entry(LogEntry::system_message(msg.clone())));
        }
        file.write_all(format_message_as_log4j_event(&msg).as_bytes())
            .await?;
    }

    Ok(java)
}

/// Adds `msg` to the game log. Best-effort: a failure to write it is logged,
/// never a failed launch.
async fn log_line(log: Option<&watch::Sender<GameLog>>, file: &mut Option<&mut File>, msg: &str) {
    let Some(file) = file else {
        return;
    };
    if let Some(log) = log {
        log.send_modify(|log| log.add_entry(LogEntry::system_message(msg.to_string())));
    }
    if let Err(e) = file
        .write_all(format_message_as_log4j_event(msg).as_bytes())
        .await
    {
        tracing::warn!({ error = ?e }, "Failed to write to the instance log");
    }
}

/// "Java 21", or "Java 8 below 8.0.312" for Forge 1.16.5.
fn need_text(need: &JavaRequirement) -> String {
    match &need.below {
        Some(below) => format!("Java {} below {below}", need.major),
        None => format!("Java {}", need.major),
    }
}

/// The Java a launch uses, and whether an override chose it. `required` is the
/// profile the version asks for, `effective` the one it really needs (see
/// [`effective_profile`]), and `floor` the instance's Java floor. The
/// override is read by [`override_choice`], the same function the Java
/// overview resolves instances with.
async fn launch_java(
    app: &App,
    required: SystemJavaProfileName,
    effective: SystemJavaProfileName,
    java_override: Option<JavaOverride>,
    floor: Option<u8>,
    t_subtasks: &TSubtasks,
) -> anyhow::Result<(JavaComponent, bool)> {
    let profiles = app
        .java_manager()
        .get_java_profiles()
        .await
        .map_err(|_| anyhow!("Could not get java profiles"))?;
    let javas = javas_from_rows(java_repo::get_all_java(&app.db).await?);
    let managed_root = app
        .settings_manager()
        .runtime_path
        .get_managed_javas()
        .to_path();

    match override_choice(
        java_override.as_ref(),
        Some(required),
        &profiles,
        &javas,
        &managed_root,
    ) {
        OverrideChoice::Path(path) => {
            trace!("Java path override: {path:?}");
            let java = RealJavaChecker::get_bin_info(
                &RealJavaChecker,
                &PathBuf::from(path),
                JavaComponentType::Custom,
            )
            .await?;
            Ok((java, true))
        }
        OverrideChoice::Linked { component, .. } => {
            trace!("Java profile override: {component:?}");
            Ok((component, true))
        }
        OverrideChoice::Automatic => Ok((
            automatic_java(app, effective, floor, t_subtasks).await?,
            false,
        )),
        OverrideChoice::ProfileAutomatic(profile) => {
            trace!(
                "Java profile override {profile:?} has no usable Java, using its automatic choice"
            );
            Ok((automatic_java(app, profile, None, t_subtasks).await?, true))
        }
    }
}

/// Each try either launches or changes something the next one sees: a Java
/// that failed its check is marked invalid, or another install finished.
const AUTOMATIC_JAVA_ATTEMPTS: usize = 6;

/// The automatic choice for `profile`, raised to the Java `floor`, checked,
/// linked to that major's system profiles, and downloaded first when there is
/// none. When the download fails, a found or user-added Java that serves the
/// instance is used instead.
///
/// When another install of the needed build is running (a background update
/// or the Add dialog), it waits for that one, without holding up other
/// launches, and gives up on it when it stops making progress.
async fn automatic_java(
    app: &App,
    profile: SystemJavaProfileName,
    floor: Option<u8>,
    t_subtasks: &TSubtasks,
) -> anyhow::Result<JavaComponent> {
    let instance_manager = app.instance_manager();
    let java_check_lock = &instance_manager.persistence_manager.java_check_lock;

    for _ in 0..AUTOMATIC_JAVA_ATTEMPTS {
        let mut guard = Some(java_check_lock.lock().await);
        let (req, pick) = resolve_automatic_requirement(app, profile, floor).await?;
        debug!("Automatic Java for {profile:?}: {pick:?}");

        if let AutomaticPick::Java { id, .. } = pick {
            if let Some(java) = app.java_manager().check_java_for(&id, &req).await? {
                // The launch has its Java: a failed link must not stop it.
                match link_requirement_profiles(&app.db, &req, &id).await {
                    Ok(true) => invalidate_java_lists(app),
                    Ok(false) => {}
                    Err(e) => tracing::warn!("could not link Java {} to its profiles: {e:#}", id),
                }
                return Ok(java);
            }
            continue;
        }

        let tile = JavaTileProgress::start(t_subtasks, req.major);
        let installed = app
            .java_manager()
            .require_java_install(&req, true, Some(tile.sender.clone()))
            .await;

        let failure = match installed {
            Ok(Some(java)) => {
                tile.finish().await?;
                return Ok(java);
            }
            Ok(None) => anyhow!("No usable java found after installation attempt"),
            Err(err) => {
                let in_flight = err
                    .downcast_ref::<AlreadyInstalling>()
                    .map(|in_flight| in_flight.build_id.clone());
                let Some(build_id) = in_flight else {
                    tile.finish().await?;
                    return fall_back(app, need_requirement(profile, floor), &req, err).await;
                };

                // Other launches must not queue behind someone else's install.
                drop(guard.take());
                let waited = app
                    .java_manager()
                    .managed_service
                    .wait_for_install(&build_id, Some(&tile.sender))
                    .await;
                tile.finish().await?;

                match waited {
                    // Its result is in the database now: resolve again.
                    Ok(()) => continue,
                    Err(err) => err,
                }
            }
        };

        let _guard = match guard {
            Some(guard) => guard,
            None => java_check_lock.lock().await,
        };
        return fall_back(app, need_requirement(profile, floor), &req, failure).await;
    }

    bail!(
        "No usable Java {} found for this instance",
        need_requirement(profile, floor).major
    )
}

/// After the Java `req` could not be installed for an instance that needs
/// `need`: a found or user-added Java that serves the instance, or the
/// failure.
async fn fall_back(
    app: &App,
    need: JavaRequirement,
    req: &JavaRequirement,
    failure: anyhow::Error,
) -> anyhow::Result<JavaComponent> {
    let reqs = [need, req.clone()];

    match fallback_java(app, &reqs).await {
        Ok(Some(java)) => {
            tracing::warn!(
                "Could not install Java {}: {failure:#}. Falling back to {} ({})",
                req.major,
                java.path,
                java.version.to_string()
            );
            Ok(java)
        }
        Ok(None) => Err(failure),
        // The install's failure is what the user needs to see.
        Err(e) => {
            tracing::warn!("could not look for a Java to fall back to: {e:#}");
            Err(failure)
        }
    }
}

/// Java install progress on the instance tile, named for the major.
struct JavaTileProgress {
    sender: watch::Sender<Step>,
    forwarder: JoinHandle<()>,
}

impl JavaTileProgress {
    fn start(t_subtasks: &TSubtasks, major: u8) -> Self {
        t_subtasks
            .t_download_java
            .update(|data| data.name = Translation::InstanceTaskLaunchDownloadJava { major });
        t_subtasks
            .t_extract_java
            .update(|data| data.name = Translation::InstanceTaskLaunchExtractJava { major });

        let (sender, receiver) = watch::channel(Step::Idle);
        let forwarder = tokio::spawn(forward_install_progress(receiver, Arc::clone(t_subtasks)));

        Self { sender, forwarder }
    }

    /// Whatever happened, the Java steps are over and must leave the tile.
    async fn finish(self) -> anyhow::Result<()> {
        let _ = self.sender.send(Step::Done);
        drop(self.sender);
        self.forwarder.await?;

        Ok(())
    }
}

/// Mirrors install progress onto the tile until every sender is dropped.
async fn forward_install_progress(mut progress: watch::Receiver<Step>, t_subtasks: TSubtasks) {
    let mut download_completed = false;

    while progress.changed().await.is_ok() {
        let step = progress.borrow_and_update().clone();

        match step {
            Step::Downloading(downloaded, total) => {
                t_subtasks
                    .t_download_java
                    .update_download(downloaded as u32, total as u32, true)
            }
            Step::Extracting(count, total) => {
                if !download_completed {
                    t_subtasks.t_download_java.complete_opaque();
                    download_completed = true;
                }

                t_subtasks
                    .t_extract_java
                    .update_items(count as u32, total as u32);
            }
            Step::Done => {
                t_subtasks.t_download_java.complete_opaque();
                t_subtasks.t_extract_java.complete_opaque();
            }
            Step::Idle => {}
        }
    }
}

// The fake `java` below is a shell script, which the real checker runs.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::domain::java::JavaVendor;
    use crate::managers::instance::run::modpack::TSubtasksInner;
    use crate::managers::java::build_version::JavaBuildVersion;
    use crate::managers::java::managed::{BuildSource, ManagedJavaBuild, ManagedJavaCatalog};
    use crate::managers::java::resolve::host_arch;
    use crate::setup_managers_for_test;
    use carbon_repos::repos::java::{self as java_repo, JavaRow};

    /// A `java` the real checker accepts: it prints what JavaCheck would.
    fn fake_java(dir: &Path, version: &str) -> String {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(dir).unwrap();
        let bin = dir.join("java");
        std::fs::write(
            &bin,
            format!(
                "#!/bin/sh\necho java.version={version}\necho os.arch={}\necho java.vendor=Eclipse Adoptium\n",
                std::env::consts::ARCH
            ),
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin.to_string_lossy().into_owned()
    }

    async fn add_java(app: &App, id: &str, ty: &str, major: i32, version: &str, path: &str) {
        java_repo::insert_java(
            &app.db,
            JavaRow {
                id: id.into(),
                path: path.into(),
                major,
                full_version: version.into(),
                r#type: ty.into(),
                os: "linux".into(),
                arch: host_arch().to_string(),
                vendor: "Eclipse Adoptium".into(),
                is_valid: true,
            },
        )
        .await
        .unwrap();
    }

    fn azul_build(id: &str, version: &[u32]) -> ManagedJavaBuild {
        ManagedJavaBuild {
            id: id.into(),
            name: format!("{id}.zip"),
            major: version[0] as u8,
            version: JavaBuildVersion::from_components(version),
            source: BuildSource::AzulZip {
                download_url: String::new(),
                sha256: None,
            },
            size_bytes: None,
            release_date: None,
            lts: false,
        }
    }

    fn delta_override() -> Option<JavaOverride> {
        Some(JavaOverride::Profile(Some(
            SystemJavaProfileName::Delta.to_string(),
        )))
    }

    #[tokio::test]
    async fn a_system_profile_override_without_a_usable_java_goes_through_automatic() {
        let app = setup_managers_for_test().await;
        // Nothing to download, so the download fails at once and falls back.
        app.java_manager()
            .managed_service
            .cache_azul_catalog_for_test(ManagedJavaCatalog::default())
            .await;
        let found = fake_java(&app.tmpdir.join("found21/bin"), "21.0.5");
        add_java(&app.app, "found21", "local", 21, "21.0.5", &found).await;

        // Fresh install: sync never links a found Java, so delta is empty.
        let (java, is_override) = launch_java(
            &app.app,
            SystemJavaProfileName::Gamma,
            SystemJavaProfileName::Gamma,
            delta_override(),
            None,
            &TSubtasksInner::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(java.path, found);
        assert!(is_override);

        // Linked to a Java that no longer passes: the same.
        add_java(
            &app.app,
            "gone21",
            "custom",
            21,
            "21.0.4",
            "/nowhere/bin/java",
        )
        .await;
        java_repo::set_java_validity(&app.db, "gone21", false)
            .await
            .unwrap();
        java_repo::set_profile_java(
            &app.db,
            &SystemJavaProfileName::Delta.to_string(),
            Some("gone21"),
        )
        .await
        .unwrap();
        let (java, _) = launch_java(
            &app.app,
            SystemJavaProfileName::Gamma,
            SystemJavaProfileName::Gamma,
            delta_override(),
            None,
            &TSubtasksInner::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(java.path, found);
    }

    #[tokio::test]
    async fn a_path_to_a_gone_gdlauncher_java_launches_on_automatic() {
        let app = setup_managers_for_test().await;
        app.java_manager()
            .managed_service
            .cache_azul_catalog_for_test(ManagedJavaCatalog::default())
            .await;
        let managed_root = app
            .settings_manager()
            .runtime_path
            .get_managed_javas()
            .to_path();
        let managed17 = fake_java(&managed_root.join("zulu17.0.13-a/bin"), "17.0.13");
        add_java(&app.app, "m17", "managed", 17, "17.0.13", &managed17).await;
        // An update deleted it while the instance wasn't loaded, so nothing
        // repointed the instance.
        let gone = managed_root
            .join("zulu17.0.12-old/bin/java")
            .to_string_lossy()
            .into_owned();
        let launch = |java_override: JavaOverride| {
            let app = app.app.clone();
            async move {
                launch_java(
                    &app,
                    SystemJavaProfileName::Gamma,
                    SystemJavaProfileName::Gamma,
                    Some(java_override),
                    None,
                    &TSubtasksInner::for_test(),
                )
                .await
            }
        };

        let (java, is_override) = launch(JavaOverride::Path(Some(gone))).await.unwrap();
        assert_eq!(java.path, managed17);
        assert!(!is_override);

        // A path of the user's, such as on a drive that isn't mounted, is
        // theirs to fix.
        assert!(
            launch(JavaOverride::Path(Some("/nowhere/bin/java".into())))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn waiting_for_another_install_lets_other_launches_through_and_uses_its_result() {
        let app = setup_managers_for_test().await;
        let b21 = azul_build("b21", &[21, 0, 5]);
        let manager = app.java_manager();
        let service = &manager.managed_service;
        service
            .cache_azul_catalog_for_test(ManagedJavaCatalog::from_builds(vec![b21.clone()]))
            .await;
        // The Add dialog is installing the build Java 21 needs.
        let installing = service
            .register_install(JavaVendor::Azul, &b21)
            .await
            .unwrap();
        let found17 = fake_java(&app.tmpdir.join("found17/bin"), "17.0.12");
        add_java(&app.app, "found17", "local", 17, "17.0.12", &found17).await;

        let waiting = tokio::spawn({
            let app = app.app.clone();
            async move {
                automatic_java(
                    &app,
                    SystemJavaProfileName::Delta,
                    None,
                    &TSubtasksInner::for_test(),
                )
                .await
            }
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Another instance launches meanwhile.
        let other = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            automatic_java(
                &app.app,
                SystemJavaProfileName::Gamma,
                None,
                &TSubtasksInner::for_test(),
            ),
        )
        .await
        .expect("other launches must not queue behind the wait")
        .unwrap();
        assert_eq!(other.path, found17);
        assert!(!waiting.is_finished());

        // The other install finishes with a managed Java 21.
        let managed21 = fake_java(&app.tmpdir.join("zulu21.0.5-b21/bin"), "21.0.5");
        add_java(&app.app, "m21", "managed", 21, "21.0.5", &managed21).await;
        drop(installing);

        let java = tokio::time::timeout(std::time::Duration::from_secs(10), waiting)
            .await
            .expect("the wait should end with the other install")
            .unwrap()
            .unwrap();
        assert_eq!(java.path, managed21);
    }
}
