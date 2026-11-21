use super::modpack::TSubtasks;
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
use crate::managers::java::java_checker::{JavaChecker, RealJavaChecker};
use crate::managers::java::managed::Step;
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
    auto_manage_java_system_profiles: bool,
    log: Option<&watch::Sender<GameLog>>,
    mut file: Option<&mut File>,
    cancel: &tokio_util::sync::CancellationToken,
    turn: &mut super::turn::InstallTurn,
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

    let msg = format!("Suggested Java Profile: {java_profile:?}");

    if let Some(ref mut file) = file {
        if let Some(log) = log {
            log.send_modify(|log| log.add_entry(LogEntry::system_message(msg.clone())));
        }
        file.write_all(format_message_as_log4j_event(&msg).as_bytes())
            .await?;
    }

    // The original metadata profile. The override-ignore comparison below matches against
    // this, because it is the system-profile name the frontend stores for a system override.
    let required_java_system_profile = SystemJavaProfileName::try_from(java_profile)
        .with_context(|| anyhow!("System java version unsupported"))?;

    // The profile actually installed and validated. Forge 1.16.5 requires an older Java 8 build
    // (the legacy-fixed-1 profile), but that patched value must NOT leak into the comparison
    // above, or a system override on a 1.16.5 Forge instance would stop being auto-managed.
    let mut effective_java_system_profile = required_java_system_profile;
    if &version.release == "1.16.5"
        && version
            .modloaders
            .iter()
            .any(|v| v.type_ == ModLoaderType::Forge)
    {
        effective_java_system_profile = SystemJavaProfileName::LegacyFixed1;
    }

    tracing::debug!(
        "This instance requires system profile: {:?}",
        effective_java_system_profile
    );

    // The built-in profile the launcher looks after for this launch: it uses
    // that profile's Java, or installs one. It is the one the instance needs
    // unless the instance is set to another built-in profile.
    let mut managed_java_system_profile = effective_java_system_profile;

    let java_component_override = match java_override {
        Some(path) => match path {
            JavaOverride::Path(value) => {
                if let Some(value) = value {
                    trace!("Java path override: {:?}", value);
                    Some(
                        RealJavaChecker::get_bin_info(
                            &RealJavaChecker::new(app.runtime_lifecycle.clone()),
                            &PathBuf::from(value),
                            JavaComponentType::Custom,
                        )
                        .await,
                    )
                } else {
                    Some(anyhow::bail!("Java path is not set"))
                }
            }
            JavaOverride::Profile(value) => {
                let Some(value) = value else {
                    anyhow::bail!("Java profile is not set");
                };
                let Ok(all_profiles) = app.java_manager().get_java_profiles().await else {
                    anyhow::bail!("Could not get java profiles");
                };
                match profile_choice(
                    &all_profiles,
                    &value,
                    required_java_system_profile,
                    effective_java_system_profile,
                    auto_manage_java_system_profiles,
                )? {
                    ProfileChoice::Managed(profile) => {
                        trace!("Java profile override, managed: {profile:?}");
                        managed_java_system_profile = profile;
                        None
                    }
                    ProfileChoice::Unknown => None,
                    ProfileChoice::Linked(java_id) => {
                        let Ok(all_javas) = app.java_manager().get_available_javas().await else {
                            anyhow::bail!("Could not get available javas");
                        };
                        let java_component = all_javas.iter().find_map(|javas| {
                            javas
                                .1
                                .iter()
                                .find(|java| java.id == java_id)
                                .map(|java| java.component.clone())
                        });
                        let Some(java_component) = java_component else {
                            anyhow::bail!(java_removed(&value));
                        };
                        trace!("Java profile override: {:?}", java_component);
                        Some(Ok(java_component))
                    }
                }
            }
        },
        None => None,
    };

    let java = {
        if let Some(java_component_override) = java_component_override {
            java_component_override?
        } else {
            let instance_manager = app.instance_manager();
            let (usable_java, _guard) = loop {
                let guard = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => anyhow::bail!("Java preparation cancelled while queued"),
                    guard = instance_manager.persistence_manager.java_check_lock.lock() => guard,
                };

                let usable_java = app
                    .java_manager()
                    .get_usable_java_for_profile_name(managed_java_system_profile)
                    .await?;

                tracing::debug!("Usable java: {:?}", usable_java);

                // Installing Java needs the turn. A run that has it waits
                // for this lock, so the turn is never waited for under it:
                // the lock is given back first, and Java looked for again
                // once the turn is this run's.
                if usable_java.is_none() && auto_manage_java_system_profiles && !turn.is_held() {
                    drop(guard);
                    turn.take().await?;
                    continue;
                }
                break (usable_java, guard);
            };

            match usable_java {
                Some(path) => path,
                None => {
                    if !auto_manage_java_system_profiles {
                        bail!("No usable java found and auto manage java is disabled");
                    }

                    let (progress_watch_tx, mut progress_watch_rx) = watch::channel(Step::Idle);

                    let t_subtasks_clone = Arc::clone(t_subtasks);

                    // dropped when the sender is dropped
                    let completion = tokio::spawn(async move {
                        let mut started = false;
                        let mut dl_completed = false;

                        while progress_watch_rx.changed().await.is_ok() {
                            let step = progress_watch_rx.borrow();

                            if !started && !matches!(*step, Step::Idle) {
                                started = true;
                            }

                            match *step {
                                Step::Downloading(..) => {
                                    forward_java_download(&t_subtasks_clone.t_download_java, &step)
                                }
                                Step::Extracting(count, total) => {
                                    if !dl_completed {
                                        t_subtasks_clone.t_download_java.complete_opaque();
                                        dl_completed = true;
                                    }

                                    t_subtasks_clone
                                        .t_extract_java
                                        .update_items(count as u32, total as u32);
                                }

                                Step::Done => {
                                    t_subtasks_clone.t_download_java.complete_opaque();
                                    t_subtasks_clone.t_extract_java.complete_opaque();
                                }

                                Step::Idle => {}
                            }

                            // this is already debounced in setup_managed
                        }
                    });

                    let path = app
                        .java_manager()
                        .require_java_install(
                            managed_java_system_profile,
                            true,
                            Some(progress_watch_tx),
                        )
                        .await;

                    completion.await?;

                    match path? {
                        Some(path) => path,
                        None => bail!("No usable java found after installation attempt"),
                    }
                }
            }
        }
    };

    // A user-set or imported override is used verbatim. Warn (without blocking, since the
    // override is a deliberate escape hatch) when its major version does not match the
    // profile this instance expects, so the user has a clear reason if the game fails to
    // start. The auto-managed path already only selects a compatible Java, so this is a
    // no-op there.
    if !effective_java_system_profile.is_java_version_compatible(&java.version) {
        let warn_msg = format!(
            "Warning: the selected Java (major {}) does not match the {:?} profile this \
             instance expects. Minecraft may fail to start; if it does, select a matching \
             Java or enable automatic Java management.",
            java.version.major, effective_java_system_profile
        );
        tracing::warn!("{warn_msg}");
        if let Some(ref mut file) = file {
            if let Some(log) = log {
                log.send_modify(|log| log.add_entry(LogEntry::system_message(warn_msg.clone())));
            }
            // Best-effort: this warning must not block the launch (the override is a deliberate
            // escape hatch), so a failure to write it to the log is logged, not propagated.
            if let Err(e) = file
                .write_all(format_message_as_log4j_event(&warn_msg).as_bytes())
                .await
            {
                tracing::warn!({ error = ?e }, "Failed to write Java mismatch warning to instance log");
            }
        }
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

/// What a Java profile chosen for an instance means when the game starts.
#[derive(Debug, PartialEq, Eq)]
enum ProfileChoice {
    /// A built-in profile the launcher looks after: it uses the profile's
    /// Java when that works, and installs one when it does not.
    Managed(SystemJavaProfileName),
    /// The Java, by id, that the profile was given by hand.
    Linked(String),
    /// No profile has that name. The instance starts as if none was chosen.
    Unknown,
}

/// Reads the profile named `chosen` for an instance that needs `required`
/// (`effective` is what is installed for it, which differs for Forge 1.16.5).
///
/// While the launcher manages the built-in profiles, choosing one means
/// "this Java version, kept by the launcher", whichever it is: the one the
/// instance needs or another. Only the first used to be honoured; any other
/// that had no Java yet failed the launch instead of getting one installed.
/// A profile the user looks after is used as it is set, and refused in words
/// that say where to set it when it has no Java.
fn profile_choice(
    profiles: &[crate::domain::java::JavaProfile],
    chosen: &str,
    required: SystemJavaProfileName,
    effective: SystemJavaProfileName,
    auto_manage: bool,
) -> anyhow::Result<ProfileChoice> {
    let Some(profile) = profiles.iter().find(|profile| profile.name == chosen) else {
        return Ok(ProfileChoice::Unknown);
    };
    if auto_manage && profile.is_system {
        if chosen == required.to_string() {
            return Ok(ProfileChoice::Managed(effective));
        }
        if let Ok(built_in) = SystemJavaProfileName::try_from(chosen) {
            return Ok(ProfileChoice::Managed(built_in));
        }
    }
    match &profile.java_id {
        Some(java_id) => Ok(ProfileChoice::Linked(java_id.clone())),
        None if profile.is_system => bail!(
            "The Java profile \"{chosen}\" has no Java set. Choose one for it in Settings > \
             Java, or turn on automatic Java management there."
        ),
        None => bail!(
            "The Java profile \"{chosen}\" has no Java set. Choose one for it in Settings > Java."
        ),
    }
}

/// Why a launch stops when the Java a profile points at is gone.
fn java_removed(profile: &str) -> String {
    format!(
        "The Java set for the profile \"{profile}\" is no longer installed. Choose another for \
         it in Settings > Java."
    )
}

fn forward_java_download(task: &Subtask, step: &Step) {
    if let Step::Downloading(current, total) = step {
        task.update_download(*current, *total, true);
    }
}
#[cfg(test)]
mod java_progress_tests {
    use super::*;
    #[tokio::test]
    async fn java_progress_retains_u64_step_bytes() {
        let task = VisualTask::new(Translation::Test);
        let subtask = task.subtask(Translation::Test);
        forward_java_download(&subtask, &Step::Downloading(4_294_967_296, 5_368_709_120));
        assert_eq!(
            task.downloaded_bytes().await,
            (4_294_967_296, 5_368_709_120)
        );
        assert!((task.progress_float().await - 0.8).abs() < 0.00001);
        forward_java_download(&subtask, &Step::Downloading(5_368_709_120, 5_368_709_120));
        assert_eq!(task.progress_float().await, 1.0);
    }
}

#[cfg(test)]
mod profile_choice_tests {
    use super::*;
    use crate::domain::java::JavaProfile;
    use SystemJavaProfileName::{Delta, Gamma, Legacy, LegacyFixed1};

    fn built_in(name: SystemJavaProfileName, java_id: Option<&str>) -> JavaProfile {
        JavaProfile {
            name: name.to_string(),
            java_id: java_id.map(str::to_owned),
            is_system: true,
        }
    }

    fn own(name: &str, java_id: Option<&str>) -> JavaProfile {
        JavaProfile {
            name: name.to_owned(),
            java_id: java_id.map(str::to_owned),
            is_system: false,
        }
    }

    fn profiles() -> Vec<JavaProfile> {
        vec![
            built_in(Legacy, None),
            built_in(Gamma, Some("zulu17")),
            built_in(Delta, None),
            own("My Java 21", Some("temurin21")),
            own("Empty", None),
        ]
    }

    fn choice(chosen: &str, auto_manage: bool) -> anyhow::Result<ProfileChoice> {
        profile_choice(&profiles(), chosen, Gamma, Gamma, auto_manage)
    }

    #[test]
    fn a_built_in_profile_without_java_gets_one_installed_instead_of_failing_the_launch() {
        // The instance needs Gamma (Java 17) and is set to Delta (Java 21),
        // which nothing was ever installed for. This used to end the launch
        // with "Java profile has no associated java id" on every Play.
        assert_eq!(
            choice(&Delta.to_string(), true).unwrap(),
            ProfileChoice::Managed(Delta)
        );
        assert_eq!(
            choice(&Legacy.to_string(), true).unwrap(),
            ProfileChoice::Managed(Legacy)
        );
    }

    #[test]
    fn a_built_in_profile_with_java_is_looked_after_too_so_a_broken_one_is_replaced() {
        // Managed, not Linked: the managed path checks the Java before using
        // it, where a linked one is taken as recorded.
        let mut all = profiles();
        all[2].java_id = Some("zulu21".into());
        assert_eq!(
            profile_choice(&all, &Delta.to_string(), Gamma, Gamma, true).unwrap(),
            ProfileChoice::Managed(Delta)
        );
    }

    #[test]
    fn the_profile_the_instance_needs_stays_the_one_installed_for_it() {
        assert_eq!(
            choice(&Gamma.to_string(), true).unwrap(),
            ProfileChoice::Managed(Gamma)
        );
        // Forge 1.16.5: the instance needs Legacy, and an older Java 8 is
        // what gets installed for it. Choosing Legacy must not lose that.
        assert_eq!(
            profile_choice(&profiles(), &Legacy.to_string(), Legacy, LegacyFixed1, true).unwrap(),
            ProfileChoice::Managed(LegacyFixed1)
        );
    }

    #[test]
    fn a_profile_of_ones_own_is_used_as_it_is_set_whatever_the_launcher_manages() {
        for auto_manage in [true, false] {
            assert_eq!(
                choice("My Java 21", auto_manage).unwrap(),
                ProfileChoice::Linked("temurin21".into())
            );
            assert_eq!(
                choice("Empty", auto_manage).unwrap_err().to_string(),
                "The Java profile \"Empty\" has no Java set. Choose one for it in Settings > Java."
            );
        }
    }

    #[test]
    fn with_automatic_java_off_a_built_in_profile_is_used_as_set_and_never_installed() {
        assert_eq!(
            choice(&Gamma.to_string(), false).unwrap(),
            ProfileChoice::Linked("zulu17".into())
        );
        let refused = choice(&Delta.to_string(), false).unwrap_err().to_string();
        assert_eq!(
            refused,
            format!(
                "The Java profile \"{}\" has no Java set. Choose one for it in Settings > Java, \
                 or turn on automatic Java management there.",
                Delta.to_string()
            )
        );
    }

    #[test]
    fn a_name_no_profile_has_starts_the_instance_as_without_a_choice() {
        assert_eq!(
            choice("Deleted profile", true).unwrap(),
            ProfileChoice::Unknown
        );
        assert_eq!(
            choice("Deleted profile", false).unwrap(),
            ProfileChoice::Unknown
        );
    }

    #[test]
    fn a_profile_whose_java_is_gone_says_where_to_choose_another() {
        assert_eq!(
            java_removed("My Java 21"),
            "The Java set for the profile \"My Java 21\" is no longer installed. Choose another \
             for it in Settings > Java."
        );
    }
}
