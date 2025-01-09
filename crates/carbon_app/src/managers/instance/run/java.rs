use super::modpack::TSubtasks;
use crate::api::translation::Translation;
use crate::domain::instance::info::{
    self, Instance, JavaOverride, Modpack, ModpackInfo, StandardVersion,
};
use crate::domain::instance::{self as domain, GameLogId, InstanceId};
use crate::domain::java::{JavaComponent, JavaComponentType, SystemJavaProfileName};
use crate::domain::metrics::GDLMetricsEvent;
use crate::domain::vtask::VisualTaskId;
use crate::managers::instance::log::{
    format_message_as_log4j_event, GameLog, LogEntry, LogEntrySourceKind,
};
use crate::managers::instance::modpack::{packinfo, PackVersionFile};
use crate::managers::instance::schema::make_instance_config;
use crate::managers::java::java_checker::{JavaChecker, RealJavaChecker};
use crate::managers::java::managed::Step;
use crate::managers::minecraft::assets::get_assets_dir;
use crate::managers::minecraft::minecraft::get_lwjgl_meta;
use crate::managers::minecraft::modrinth;
use crate::managers::minecraft::{curseforge, UpdateValue};
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::managers::modplatforms::modrinth::convert_mr_version_to_standard_version;
use crate::managers::vtask::Subtask;
use crate::managers::AppInner;
use crate::util::NormalizedWalkdir;
use crate::{
    domain::instance::info::{GameVersion, ModLoader, ModLoaderType},
    managers::{
        self,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
        ManagerRef,
    },
};
use anyhow::{anyhow, bail, Context, Error};
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
use tokio::sync::{watch, Mutex, Semaphore};
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
    log: &watch::Sender<GameLog>,
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

    let msg = format!("Suggested Java Profile: {java_profile:?}");

    log.send_modify(|log| log.add_entry(LogEntry::system_message(msg.clone())));

    if let Some(ref mut file) = file {
        file.write_all(format_message_as_log4j_event(&msg).as_bytes())
            .await?;
    }

    let mut required_java_system_profile = SystemJavaProfileName::try_from(java_profile)
        .with_context(|| anyhow!("System java version unsupported"))?;

    tracing::debug!(
        "This instance requires system profile: {:?}",
        required_java_system_profile
    );

    let java_component_override = match java_override {
        Some(path) => match path {
            JavaOverride::Path(value) => {
                if let Some(value) = value {
                    trace!("Java path override: {:?}", value);
                    Some(
                        RealJavaChecker::get_bin_info(
                            &RealJavaChecker,
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
                if let Some(value) = value {
                    if let Ok(all_profiles) = app.java_manager().get_java_profiles().await {
                        if let Ok(all_javas) = app.java_manager().get_available_javas().await {
                            let mut result = None;
                            for profile in all_profiles.iter() {
                                if profile.name == value {
                                    if auto_manage_java_system_profiles
                                        && profile.is_system
                                        && value == required_java_system_profile.to_string()
                                    {
                                        // !! THIS IS USED TO EFFECTIVELY TREAT IT AS A NON-OVERRIDE, SO THAT THE LAUNCHER MANAGES THE PROFILE NORMALLY !!
                                        trace!(
                                            "Java profile override BUT ignoring: {:?}",
                                            profile.name
                                        );
                                        result = None;
                                    } else {
                                        if let Some(java_id) = profile.java_id.as_ref() {
                                            let java_component =
                                                all_javas.iter().find_map(|javas| {
                                                    javas.1.iter().find_map(|java| {
                                                        if &java.id == java_id {
                                                            Some(java.component.clone())
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                });

                                            if let Some(java_component) = java_component {
                                                trace!(
                                                    "Java profile override: {:?}",
                                                    java_component
                                                );
                                                result = Some(Ok(java_component));
                                            } else {
                                                result = Some(anyhow::bail!(
                                                    "Java profile has associated java id that does not exist"
                                                ));
                                            }
                                        } else {
                                            result = Some(anyhow::bail!(
                                                "Java profile has no associated java id"
                                            ));
                                        }
                                    }
                                }
                            }

                            result
                        } else {
                            Some(anyhow::bail!("Could not get available javas"))
                        }
                    } else {
                        Some(anyhow::bail!("Could not get java profiles"))
                    }
                } else {
                    Some(anyhow::bail!("Java profile is not set"))
                }
            }
        },
        None => None,
    };

    let java = {
        if let Some(java_component_override) = java_component_override {
            java_component_override?
        } else {
            // Forge 1.16.5 requires an older java 8 version so we inject the legacy fixed 1 profile
            if &version.release == "1.16.5"
                && *&version
                    .modloaders
                    .iter()
                    .find(|v| v.type_ == ModLoaderType::Forge)
                    .is_some()
            {
                required_java_system_profile = SystemJavaProfileName::LegacyFixed1;
            }

            let instance_manager = app.instance_manager();
            let _guard = instance_manager
                .persistence_manager
                .java_check_lock
                .lock()
                .await;

            let usable_java = app
                .java_manager()
                .get_usable_java_for_profile_name(required_java_system_profile)
                .await?;

            tracing::debug!("Usable java: {:?}", usable_java);

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
                                Step::Downloading(downloaded, total) => t_subtasks_clone
                                    .t_download_java
                                    .update_download(downloaded as u32, total as u32, true),
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
                            required_java_system_profile,
                            true,
                            Some(progress_watch_tx),
                        )
                        .await?;

                    completion.await?;

                    match path {
                        Some(path) => path,
                        None => bail!("No usable java found after installation attempt"),
                    }
                }
            }
        }
    };

    let msg = format!("Using Java: {java:#?}");

    log.send_modify(|log| log.add_entry(LogEntry::system_message(msg.clone())));

    if let Some(file) = file {
        file.write_all(format_message_as_log4j_event(&msg).as_bytes())
            .await?;
    }

    Ok(java)
}
