use super::Set;
use super::keys::instance::*;
use super::router::router;
use super::settings::ModSources;
use super::translation::Translation;
use super::vtask::FETaskId;
use crate::api::keys;
use crate::domain::instance::{self as domain, InstanceModpackInfo};
use crate::error::{AxumError, FeError, FeErrorCode};
use crate::managers::account::gdl_account::{
    PaginatedShares, QuotaInfo, RegenerateShareCodeResponse, ShareInfo, SharePreview, SharedMod,
    UpdateShareBody, WaitForShareInstanceResponse,
};
use crate::managers::instance as manager;
use crate::managers::instance::InstanceMoveTarget;
use crate::managers::instance::export::{InstanceTooLargeError, ShareInstanceProgress};
use crate::managers::instance::importer::ImportShareCodeProgress;
use crate::managers::instance::log::{LogEntrySourceKind, SearchResult};
use crate::managers::{App, AppInner, instance::importer};
use anyhow::anyhow;
use axum::Json;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use carbon_platforms as mpdomain;
use chrono::{DateTime, Utc};
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;

pub(super) fn mount() -> RouterBuilder<App> {
    router! {
        query DEFAULT_GROUP[app, args: ()] {
            Ok(*app.instance_manager()
                .get_default_group()
                .await?)
        }

        query GET_GROUPS[app, args: ()] {
            Ok(app.instance_manager()
                .list_groups()
                .await?
                .into_iter()
                .map(ListGroup::from)
                .collect::<Vec<_>>())
        }

        query GET_ALL_INSTANCES[app, args: ()] {
            Ok(app.instance_manager()
                .list_groups()
                .await?
                .into_iter()
                .flat_map(|group| group.instances.into_iter().map(ListInstance::from))
                .collect::<Vec<_>>())
        }

        mutation CREATE_INSTANCE[app, details: CreateInstance] {
            if details.name.is_empty() {
                return Err(anyhow::anyhow!("instance name cannot be empty"));
            }

            let group: domain::GroupId = match details.group {
                Some(group) => group.into(),
                None => app.instance_manager()
                .get_default_group()
                .await?
            };

            let version = details.version.try_into()?;

            if let Some(icon_url) = details.icon_url {
                let icon = match app.instance_manager().download_icon(icon_url).await {
                    Ok(icon) => Some(icon),
                    Err(e) => {
                        tracing::warn!("Failed to download icon for new instance, using default: {e}");
                        None
                    }
                };

                app.instance_manager()
                    .create_instance_ext(
                        group,
                        details.name,
                        icon,
                        None,
                        None,
                        version,
                        details.notes,
                        |_| async { Ok(()) },
                    )
                    .await
                    .map(FEInstanceId::from)
            } else {
                app.instance_manager()
                    .create_instance(
                        group,
                        details.name,
                        details.use_loaded_icon,
                        version,
                        details.notes,
                    )
                    .await
                    .map(FEInstanceId::from)
            }
        }

        mutation CHANGE_MODPACK[app, details: ChangeModpack] {
            app.instance_manager()
                .change_modpack(
                    details.instance.into(),
                    details.modpack.into(),
                )
                .await
                .map(FETaskId::from)
        }

        mutation REPAIR_MODPACK[app, args: RepairModpack] {
            app.instance_manager()
                .repair_modpack(
                    args.instance.into(),
                    manager::modpack::RepairMarkerFile {
                        re_enable_disabled: args.re_enable_disabled,
                        cleanup_paths: args.cleanup_paths,
                    },
                )
                .await
                .map(FETaskId::from)
        }

        query GET_REPAIR_PREVIEW[app, args: RepairPreviewArgs] {
            app.instance_manager()
                .repair_preview(args.instance.into())
                .await
                .map(FERepairPreview::from)
        }

        mutation CHECK_PACK_ORIGIN[app, id: FEInstanceId] {
            app.instance_manager()
                .check_pack_origin(id.into())
                .await
                .map(FETaskId::from)
        }

        mutation LOAD_ICON_URL[app, url: String] {
            let icon = app.instance_manager()
                .download_icon(url)
                .await?;

            app.instance_manager().set_loaded_icon(icon).await;
            Ok(())
        }

        mutation DELETE_GROUP[app, id: FEGroupId] {
            app.instance_manager()
                .delete_group(id.into())
                .await
        }

        mutation DELETE_GROUP_WITH_INSTANCES[app, id: FEGroupId] {
            app.instance_manager()
                .delete_group_with_instances(id.into())
                .await
        }

        mutation RENAME_GROUP[app, rename: RenameGroup] {
            app.instance_manager()
                .rename_group(rename.group.into(), rename.name)
                .await
        }

        mutation DELETE_INSTANCE[app, id: FEInstanceId] {
            app.instance_manager()
                .delete_instance(id.into())
                .await
        }

        mutation MOVE_GROUP[app, move_data: MoveGroup] {
            let target = match move_data.target {
                MoveGroupTarget::BeforeGroup(id) => manager::GroupMoveTarget::BeforeGroup(id.into()),
                MoveGroupTarget::BeforeInstance(id) => manager::GroupMoveTarget::BeforeInstance(id.into()),
                MoveGroupTarget::EndOfLibrary => manager::GroupMoveTarget::EndOfLibrary,
            };
            app.instance_manager()
                .move_group(move_data.group.into(), target)
                .await
        }

        mutation MOVE_INSTANCE[app, move_instance: MoveInstance] {
            app.instance_manager()
                .move_instance(
                    move_instance.instance.into(),
                    match move_instance.target {
                        MoveInstanceTarget::BeforeInstance(instance)
                            => InstanceMoveTarget::Before(instance.into()),
                        MoveInstanceTarget::BeginningOfGroup(group)
                            => InstanceMoveTarget::BeginningOfGroup(group.into()),
                        MoveInstanceTarget::EndOfGroup(group)
                            => InstanceMoveTarget::EndOfGroup(group.into()),
                        MoveInstanceTarget::BeforeGroup(group)
                            => InstanceMoveTarget::BeforeGroup(group.into()),
                    }
                )
                .await
        }

        mutation CREATE_FOLDER_FROM_INSTANCES[app, data: CreateFolderFromInstances] {
            app.instance_manager()
                .create_folder_from_instances(
                    data.instances.into_iter().map(|id| id.into()).collect(),
                    data.target_instance_id.map(|id| id.into()),
                )
                .await
                .map(FEGroupId::from)
        }

        mutation ARRANGE_LIBRARY[app, sort_by: LibrarySortCriteria] {
            app.instance_manager()
                .arrange_library(sort_by.into())
                .await
        }

        mutation ARRANGE_GROUP[app, data: ArrangeGroup] {
            app.instance_manager()
                .arrange_group(data.group.into(), data.sort_by.into())
                .await
        }

        mutation DUPLICATE_INSTANCE[app, details: DuplicateInstance] {
            app.instance_manager()
                .duplicate_instance(
                    details.instance.into(),
                    details.new_name,
                )
                .await
                .map(FEInstanceId::from)
        }

        mutation UPDATE_INSTANCE[app, details: FEUpdateInstance] {
            app.instance_manager()
                .update_instance(details.try_into()?)
                .await
                .map(|task_id| task_id.map(super::vtask::FETaskId::from))
        }

        mutation SET_FAVORITE[app, favorite: SetFavorite] {
            app.instance_manager()
                .set_favorite(
                    favorite.instance.into(),
                    favorite.favorite,
                )
                .await
        }

        query INSTANCE_DETAILS[app, id: Option<FEInstanceId>] {
            let Some(id) = id else {
                return Ok(None);
            };

            let result = app.instance_manager()
                .instance_details(id.into())
                .await
                .map(InstanceDetails::from);

            Ok(Some(result?))
        }

        query GET_MODPACK_INFO[app, id: Option<FEInstanceId>] {
            let Some(id) = id else {
                return Ok(None);
            };

            let result = app.instance_manager()
                .get_modpack_info(id.into())
                .await?
                .map(FEInstanceModpackInfo::from);

            Ok(result)
        }

        query INSTANCE_MODS[app, instance_id: FEInstanceId] {
            let result = app.instance_manager()
                .list_mods(instance_id.into(), None)
                .await?
                .into_iter()
                .map(Into::into)
                .collect::<Vec<Mod>>();

            Ok(Some(result))
        }

        mutation PRIORITIZE_INSTANCE_CACHE[app, instance_id: Option<FEInstanceId>] {
            use crate::managers::metadata::cache::CacheEntityId;
            app.meta_cache_manager()
                .watch_and_prioritize(instance_id.map(|id| CacheEntityId::Instance(id.into())))
                .await;

            Ok(())
        }

        mutation PREPARE_INSTANCE[app, id: FEInstanceId] {
            let (_, vtask_id) = app.instance_manager()
                .prepare_game(id.into(), None, None, true)
                .await?;

            Ok(FETaskId::from(vtask_id))
        }

        mutation LAUNCH_INSTANCE[app, args: FELaunchInstanceArgs] {
            let account = app.account_manager()
                .get_active_account()
                .await?;

            let Some(account) = account else {
                return Err(anyhow::anyhow!("attempted to launch instance without an account"));
            };

            let memory_check_dismissed = crate::api::settings::is_memory_warning_dismissed(
                &app.db,
            ).await.unwrap_or(false);

            if !args.skip_memory_check && !memory_check_dismissed {
                let instance_id = crate::domain::instance::InstanceId(args.id.0);
                let (_xms, xmx) = app.instance_manager()
                    .get_effective_memory(instance_id)
                    .await?;

                let available_bytes = app.system_info_manager()
                    .get_available_ram()
                    .await;
                let available_mb: u64 = available_bytes / 1024 / 1024;

                // If we couldn't determine available RAM, skip the check
                // and let the user launch normally.
                if available_mb > 0 {
                    // JVM needs native memory beyond the heap for JIT compiler, thread
                    // stacks, metaspace, GC structures, direct buffers, etc.
                    // A 1500 MB buffer accounts for this overhead.
                    let total_estimated_mb: u64 = u64::from(xmx).saturating_add(1500);

                    if total_estimated_mb > available_mb {
                        return Err(crate::managers::instance::run::InsufficientMemoryError {
                            instance_id: args.id.0,
                            requested_mb: u64::from(xmx),
                            needed_mb: total_estimated_mb,
                            available_mb,
                        }.into());
                    }
                }
            }

            app.instance_manager()
                .prepare_game(args.id.into(), Some(account), None, false)
                .await?;

            Ok(())
        }

        mutation KILL_INSTANCE[app, id: FEInstanceId] {
            app.instance_manager()
                .kill_instance(id.into())
                .await
        }

        query GET_LOGS[app, id: FEInstanceId] {
            Ok(app.instance_manager()
               .get_logs(id.into())
               .await
               .into_iter()
               .map(GameLogEntry::from)
               .collect::<Vec<_>>())
        }

        query SEARCH_LOGS[app, query: SearchLogsQuery] {
            let res = app.instance_manager()
                .search_in_log(
                    query.log_id.into(),
                    &*query.query,
                    query.match_case,
                    query.match_whole_word,
                    query.use_regex,
                )
                .await?
                .into_iter()
                .map(FESearchResult::from)
                .collect::<Vec<_>>();

            Ok(res)
        }

        mutation DELETE_LOG[app, id: GameLogId] {
            app.instance_manager()
                .delete_log(id.into())
                .await
        }

        mutation OPEN_LOG_IN_FOLDER[app, req: OpenLogInFolder] {
            Ok(app.instance_manager()
                .get_log_file_path(req.instance_id.into(), req.log_id.into())
                .await?)
        }

        mutation ENABLE_MOD[app, imod: InstanceMod] {
            app.instance_manager()
                .enable_mod(
                    imod.instance_id.into(),
                    imod.mod_id,
                    true,
                )
                .await
        }

        mutation DISABLE_MOD[app, imod: InstanceMod] {
            app.instance_manager()
                .enable_mod(
                    imod.instance_id.into(),
                    imod.mod_id,
                    false,
                )
                .await
        }

        mutation DELETE_MOD[app, imod: InstanceMod] {
            app.instance_manager()
                .delete_mod(
                    imod.instance_id.into(),
                    imod.mod_id,
                )
                .await
        }

        mutation INSTALL_MOD[app, imod: InstallMod] {
            let task = match imod.mod_source {
                ModSource::Curseforge(cf_mod) => {
                    app.instance_manager()
                        .install_curseforge_mod(
                            imod.instance_id.into(),
                            cf_mod.project_id,
                            cf_mod.file_id,
                            imod.install_deps,
                            imod.replaces_mod,
                        )
                        .await?
                }
                ModSource::Modrinth(mdr_mod) => {
                    app.instance_manager()
                        .install_modrinth_mod(
                            imod.instance_id.into(),
                            mdr_mod.project_id,
                            mdr_mod.version_id,
                            imod.install_deps,
                            imod.replaces_mod,
                        )
                        .await?
                }
            };

            Ok(super::vtask::FETaskId::from(task))
        }

        mutation UPDATE_MOD[app, args: UpdateMod] {
            let task = app.instance_manager().update_mod(
                args.instance_id.into(),
                args.mod_id,
            ).await?;

            Ok(super::vtask::FETaskId::from(task))
        }

        // query FIND_MOD_UPDATE[app, args: UpdateMod] {
        //     app.instance_manager().find_mod_update(
        //         args.instance_id.into(),
        //         args.mod_id,
        //     ).await
        //     .map(|v| v.map(RemoteVersion::from))
        // }

        query GET_MOD_SOURCES[app, instance_id: FEInstanceId] {
            app.instance_manager()
                .get_instance_mod_sources(instance_id.into())
                .await
                .map(crate::api::settings::ModSources::from)
        }

        query CHECK_SHADER_REQUIREMENTS[app, instance_id: FEInstanceId] {
            app.instance_manager()
                .check_shader_requirements(instance_id.into())
                .await
        }

        mutation INSTALL_FABRIC_LOADER_DEFAULT[app, instance_id: FEInstanceId] {
            let task_id = app.instance_manager()
                .install_fabric_loader_default(instance_id.into())
                .await?;
            Ok(super::vtask::FETaskId::from(task_id))
        }

        mutation INSTALL_LATEST_MOD[app, imod: InstallLatestMod] {
            let task = match imod.mod_source {
                LatestModSource::Curseforge(cf_mod) => {
                    app.instance_manager()
                        .install_latest_curseforge_mod(
                            imod.instance_id.into(),
                            cf_mod,
                        )
                        .await?
                }
                LatestModSource::Modrinth(mdr_mod) => {
                    app.instance_manager()
                        .install_latest_modrinth_mod(
                            imod.instance_id.into(),
                            mdr_mod
                        )
                        .await?
                }
            };

            Ok(super::vtask::FETaskId::from(task))
        }

        mutation OPEN_INSTANCE_FOLDER[app, folder: OpenInstanceFolder] {
            app.instance_manager().open_folder(
                folder.instance_id.into(),
                folder.folder.into(),
            )
            .await
        }

        query VALIDATE_SHARE_CODE[app, share_code: String] {
            app.account_manager()
                .validate_share_code(share_code)
                .await
        }

        query GET_SHARE_PREVIEW[app, share_code: String] {
            let preview = app.account_manager()
                .get_share_preview(share_code)
                .await?;

            Ok(FESharePreview::from(preview))
        }

        query GET_IMPORTABLE_ENTITIES[_, _args: ()] {
            anyhow::Result::Ok(importer::Entity::list()
                .into_iter()
                .map(|(e, support, selection_type)| ImportEntityStatus {
                    entity: ImportEntity::from(e),
                    supported: support,
                    selection_type: ImportEntitySelectionType::from(selection_type),
                })
                .collect::<Vec<_>>())
        }

        query GET_IMPORT_ENTITY_DEFAULT_PATH[_, entity: ImportEntity] {
            importer::Entity::from(entity)
                .get_default_scan_path().await
        }

        mutation SET_IMPORT_SCAN_TARGET[app, target: (ImportEntity, String)] {
            app.instance_manager()
                .import_manager()
                .set_scan_target(Some((target.0.into(), PathBuf::from(target.1))))
        }

        mutation CANCEL_IMPORT_SCAN[app, args: ()] {
            app.instance_manager()
                .import_manager()
                .set_scan_target(None)
        }

        query GET_IMPORT_SCAN_STATUS[app, args: ()] {
            app.instance_manager()
                .import_manager()
                .scan_status()
                .await
                .map(FullImportScanStatus::from)
        }

        mutation IMPORT_INSTANCE[app, req: ImportRequest] {
            app.instance_manager()
                .import_manager()
                .begin_import(req.index, req.name)
                .await
                .map(FETaskId::from)
        }

        query EXPLORE[app, args: ExploreQuery] {
            app.instance_manager().explore_data(
                args.instance_id.into(),
                args.path,
            ).await
                .map(|entries| entries.into_iter().map(ExploreEntry::from).collect::<Vec<_>>())
        }

        mutation EXPORT[app, args: ExportArgs] {
            let task = app.instance_manager()
                .export_manager()
                .export_instance(
                    args.instance_id.into(),
                    args.target.into(),
                    args.save_path.into(),
                    args.self_contained_addons_bundling,
                    args.filter.into(),
                    args.version,
                ).await?;

            Ok(FETaskId::from(task))
        }

        query WAIT_FOR_SHARE_INSTANCE[app, args: FEWaitForShareInstanceArgs] {
            let instance_id = args.instance_id.map(|id| domain::InstanceId(id));

            let resp = app.instance_manager()
                .export_manager()
                .wait_for_share_instance(args.file_key, instance_id)
                .await?;

            Ok(FEWaitForInstanceShareResponse::from(resp))
        }

        query GET_USER_SHARES[app, args: FEGetUserSharesArgs] {
            let Some(gdl_account_uuid) = app
                .settings_manager()
                .get_settings()
                .await?
                .gdl_account_uuid
            else {
                anyhow::bail!("no gdl account found");
            };

            let shares = app
                .account_manager()
                .get_user_shares(
                    gdl_account_uuid,
                    args.limit.map(|l| l as i64),
                    args.offset.map(|o| o as i64),
                )
                .await?;

            Ok(FEPaginatedShares::from(shares))
        }

        query GET_USER_QUOTA[app, args: ()] {
            let Some(gdl_account_uuid) = app
                .settings_manager()
                .get_settings()
                .await?
                .gdl_account_uuid
            else {
                anyhow::bail!("no gdl account found");
            };

            let quota = app
                .account_manager()
                .get_quota(gdl_account_uuid)
                .await?;

            Ok(FEQuotaInfo::from(quota))
        }

        mutation DELETE_SHARE[app, share_code: String] {
            let Some(gdl_account_uuid) = app
                .settings_manager()
                .get_settings()
                .await?
                .gdl_account_uuid
            else {
                anyhow::bail!("no gdl account found");
            };

            app.account_manager()
                .delete_share(gdl_account_uuid, share_code)
                .await?;

            Ok(())
        }

        mutation UPDATE_SHARE[app, args: FEUpdateShareArgs] {
            let Some(gdl_account_uuid) = app
                .settings_manager()
                .get_settings()
                .await?
                .gdl_account_uuid
            else {
                anyhow::bail!("no gdl account found");
            };

            app.account_manager()
                .update_share(
                    gdl_account_uuid,
                    args.share_code,
                    args.title,
                    args.max_downloads,
                )
                .await?;

            Ok(())
        }

        mutation REPORT_SHARE[app, args: FEReportShareArgs] {
            let Some(gdl_account_uuid) = app
                .settings_manager()
                .get_settings()
                .await?
                .gdl_account_uuid
            else {
                anyhow::bail!("no gdl account found");
            };

            app.account_manager()
                .report_share(
                    gdl_account_uuid,
                    args.share_code,
                    args.report_type,
                    args.reason,
                )
                .await?;

            Ok(())
        }

        mutation REGENERATE_SHARE_CODE[app, share_code: String] {
            let Some(gdl_account_uuid) = app
                .settings_manager()
                .get_settings()
                .await?
                .gdl_account_uuid
            else {
                anyhow::bail!("no gdl account found");
            };

            let response = app
                .account_manager()
                .regenerate_share_code(gdl_account_uuid, share_code)
                .await?;

            Ok(FERegenerateShareCodeResponse::from(response))
        }
    }
}

pub(super) fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    #[derive(Deserialize)]
    struct InstanceIconQuery {
        id: i32,
        rev: Option<i32>,
    }

    #[derive(Deserialize)]
    struct ModpackIconQuery {
        instance_id: i32,
    }

    #[derive(Deserialize)]
    struct ModIconQuery {
        instance_id: i32,
        mod_id: String,
        platform: String,
    }

    #[derive(Deserialize)]
    struct IconPathQuery {
        path: String,
    }

    async fn instance_icon(
        State(app): State<Arc<AppInner>>,
        Query(query): Query<InstanceIconQuery>,
    ) -> Result<impl IntoResponse, impl IntoResponse> {
        let icon = app
            .instance_manager()
            .instance_icon(domain::InstanceId(query.id))
            .await
            .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

        let res = match icon {
            Some((name, icon)) => {
                let mut headers = HeaderMap::new();
                headers.insert(
                    "filename",
                    name.parse::<HeaderValue>()
                        .map_err(|e| FeError::from_anyhow(&anyhow!(e)).make_axum())?,
                );

                (StatusCode::OK, headers, icon)
            }
            None => (StatusCode::NO_CONTENT, HeaderMap::new(), Vec::new()),
        };

        Ok::<_, AxumError>(res)
    }

    async fn share_instance(
        State(app): State<Arc<AppInner>>,
        Query(query): Query<ShareInstanceQuery>,
    ) -> Result<impl IntoResponse, impl IntoResponse> {
        let cancel_token = tokio_util::sync::CancellationToken::new();

        let (mut rx, handle) = app
            .instance_manager()
            .export_manager()
            .share_instance(
                query.instance_id.into(),
                query.title,
                query.expiration_days,
                query.max_downloads,
                query.include_saves,
                cancel_token.clone(),
            )
            .await
            .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

        let abort_handle = handle.abort_handle();

        struct CancelGuard {
            token: tokio_util::sync::CancellationToken,
            abort_handle: tokio::task::AbortHandle,
            completed: bool,
        }

        impl Drop for CancelGuard {
            fn drop(&mut self) {
                if !self.completed {
                    tracing::info!("ShareInstance: client disconnected, cancelling task");
                    self.token.cancel();
                    self.abort_handle.abort();
                }
            }
        }

        let guard = CancelGuard {
            token: cancel_token,
            abort_handle,
            completed: false,
        };

        let response = axum::response::sse::Sse::new(async_stream::stream! {
            // Explicitly move guard into the stream so it lives until the stream is dropped
            let mut guard = guard;

            yield Ok::<_, Infallible>(axum::response::sse::Event::default()
                .json_data(FEShareInstanceProgress::Progress(0))
                .unwrap());

            let mut last_progress = ShareInstanceProgress::Progress(0);

            while let Some(progress) = rx.recv().await {
                if last_progress != progress {
                    last_progress = progress.clone();
                    yield Ok(axum::response::sse::Event::default()
                        .json_data(FEShareInstanceProgress::from(progress.clone()))
                        .unwrap());
                }
            }

            let result = match handle.await {
                Ok(result) => result,
                Err(err) => Err(anyhow::anyhow!(err)),
            };

            match result {
                Ok(result) => {
                    tracing::info!("Share instance finished with result: {}", result);
                    guard.completed = true;
                    yield Ok(axum::response::sse::Event::default()
                        .json_data(FEShareInstanceProgress::Finished(result))
                        .unwrap());
                }
                Err(err) => {
                    use crate::managers::account::gdl_account::InstanceShareError;
                    tracing::error!("Share instance failed with error: {}", err);
                    guard.completed = true;

                    let fe_error = if let Some(too_large) =
                        err.downcast_ref::<InstanceTooLargeError>()
                    {
                        FEShareInstanceProgress::Error {
                            code: "INSTANCE_TOO_LARGE".to_string(),
                            message: too_large.to_string(),
                            details: Some(FEShareErrorDetails::from(too_large)),
                        }
                    } else if let Some(share_err) = err.downcast_ref::<InstanceShareError>() {
                        FEShareInstanceProgress::Error {
                            code: share_err.error_code().to_string(),
                            message: share_err.to_string(),
                            details: None,
                        }
                    } else {
                        FEShareInstanceProgress::Error {
                            code: "UNKNOWN_ERROR".to_string(),
                            message: err.to_string(),
                            details: None,
                        }
                    };

                    // Delivered as a normal `message` event (not a named
                    // `error` event): the frontend's EventSource reserves the
                    // `error` event for its own connection-level failures, so
                    // an app error sent under that name collides with native
                    // errors and can be dropped. On the default stream
                    // `onmessage` always sees it.
                    yield Ok(axum::response::sse::Event::default()
                        .json_data(fe_error)
                        .unwrap());
                }
            };
        });

        Ok::<_, AxumError>(response)
    }

    async fn import_share_instance(
        State(app): State<Arc<AppInner>>,
        Query(query): Query<ImportShareCodeQuery>,
    ) -> Result<impl IntoResponse, impl IntoResponse> {
        let cancel_token = tokio_util::sync::CancellationToken::new();

        let (mut rx, handle) = app
            .instance_manager()
            .import_manager()
            .import_instance_share_code_with_progress(query.share_code, cancel_token.clone())
            .await
            .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

        let abort_handle = handle.abort_handle();

        struct CancelGuard {
            token: tokio_util::sync::CancellationToken,
            abort_handle: tokio::task::AbortHandle,
            completed: bool,
        }

        impl Drop for CancelGuard {
            fn drop(&mut self) {
                if !self.completed {
                    tracing::info!("ImportShareInstance: client disconnected, cancelling task");
                    self.token.cancel();
                    self.abort_handle.abort();
                }
            }
        }

        let guard = CancelGuard {
            token: cancel_token,
            abort_handle,
            completed: false,
        };

        let response = axum::response::sse::Sse::new(async_stream::stream! {
            let mut guard = guard;

            yield Ok::<_, Infallible>(axum::response::sse::Event::default()
                .json_data(FEImportShareCodeProgress::Progress(0))
                .unwrap());

            let mut last_progress = ImportShareCodeProgress::Progress(0);

            while let Some(progress) = rx.recv().await {
                if last_progress != progress {
                    last_progress = progress.clone();
                    yield Ok(axum::response::sse::Event::default()
                        .json_data(FEImportShareCodeProgress::from(progress.clone()))
                        .unwrap());
                }
            }

            let result = match handle.await {
                Ok(result) => result,
                Err(err) => Err(anyhow::anyhow!(err)),
            };

            match result {
                Ok(()) => {
                    tracing::info!("Import share instance finished successfully");
                    guard.completed = true;
                    yield Ok(axum::response::sse::Event::default()
                        .json_data(FEImportShareCodeProgress::Finished("ok".into()))
                        .unwrap());
                }
                Err(err) => {
                    use crate::managers::account::gdl_account::InstanceShareError;
                    tracing::error!("Import share instance failed with error: {}", err);
                    guard.completed = true;

                    let (code, message) = err
                        .downcast_ref::<InstanceShareError>()
                        .map(|e| (e.error_code().to_string(), e.to_string()))
                        .unwrap_or_else(|| ("UNKNOWN_ERROR".to_string(), err.to_string()));

                    yield Ok(axum::response::sse::Event::default()
                        .event("error")
                        .json_data(FEImportShareCodeProgress::Error { code, message })
                        .unwrap());
                }
            };
        });

        Ok::<_, AxumError>(response)
    }

    axum::Router::new()
        .route(
            "/instanceIcon",
            axum::routing::get(instance_icon),
        )
        .route(
            "/modIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<ModIconQuery>| async move {
                    let platformid = match &query.platform as &str {
                        "metadata" => 0,
                        "curseforge" => 1,
                        "modrinth" => 2,
                        _ => return Err(FeError::from_anyhow(&anyhow::anyhow!("unsupported platform")).make_axum()),
                    };

                    let icon = app.instance_manager()
                        .get_mod_icon(domain::InstanceId(query.instance_id), query.mod_id, platformid)
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

                    let res = match icon {
                        Some(icon) => (StatusCode::OK, icon),
                        None => (StatusCode::NO_CONTENT, Vec::new()),
                    };

                    Ok(res)
                }
            )
        )
        .route(
            "/modpackIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<ModpackIconQuery>| async move {
                    let icon = app.instance_manager()
                        .get_modpack_icon(domain::InstanceId(query.instance_id))
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;


                        let res = match icon {
                            Some(icon) => {
                                (StatusCode::OK, icon)
                            }
                            None => (StatusCode::NO_CONTENT, Vec::new()),
                        };

                        Ok::<_, AxumError>(res)
                }
            )
        )
        .route(
            "/loadIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<IconPathQuery>| async move {
                    let icon = app.instance_manager()
                        .load_icon(PathBuf::from(query.path))
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

                    app.instance_manager().set_loaded_icon(icon.clone()).await;

                    let icon_bytes = icon.1;
                    Ok::<_, AxumError>(icon_bytes)
                }
            )
        )
        .route("/log", axum::routing::get(log::log_handler))
        .route("/shareInstance", axum::routing::get(share_instance))
        .route("/importShareInstance", axum::routing::get(import_share_instance))
}

#[derive(Type, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct FEGroupId(i32);

#[derive(Type, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct FEInstanceId(i32);

impl From<domain::GroupId> for FEGroupId {
    fn from(value: domain::GroupId) -> Self {
        Self(*value)
    }
}

impl From<domain::InstanceId> for FEInstanceId {
    fn from(value: domain::InstanceId) -> Self {
        Self(*value)
    }
}

impl From<FEGroupId> for domain::GroupId {
    fn from(value: FEGroupId) -> Self {
        Self(value.0)
    }
}

impl From<FEInstanceId> for domain::InstanceId {
    fn from(value: FEInstanceId) -> Self {
        Self(value.0)
    }
}

#[derive(Type, Debug, Serialize)]
struct ListGroup {
    id: FEGroupId,
    name: String,
    library_position: Option<i32>,
}

#[derive(Type, Debug, Serialize)]
struct ListInstance {
    id: FEInstanceId,
    group_id: FEGroupId,
    index: i32,
    library_position: Option<i32>,
    name: String,
    favorite: bool,
    status: ListInstanceStatus,
    icon_revision: Option<u32>,
    last_played: Option<DateTime<Utc>>,
    date_created: DateTime<Utc>,
    date_updated: DateTime<Utc>,
    seconds_played: u32,
    locked: bool,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status", content = "value")]
enum ListInstanceStatus {
    Valid(ValidListInstance),
    Invalid(InvalidListInstance),
}

#[derive(Type, Debug, Serialize)]
struct ValidListInstance {
    mc_version: Option<String>,
    modloader: Option<FEInstanceModloaderType>,
    modloader_version: Option<String>,
    modpack: Option<Modpack>,
    state: LaunchState,
}

#[derive(Type, Debug, Serialize)]
enum InvalidListInstance {
    JsonMissing,
    JsonError(ConfigurationParseError),
    Other(String),
}

#[derive(Type, Debug, Serialize)]
struct ConfigurationParseError {
    type_: ConfigurationParseErrorType,
    message: String,
    line: u32,
    config_text: String,
}

#[derive(Type, Debug, Serialize)]
enum ConfigurationParseErrorType {
    Syntax,
    Data,
    Eof,
    Unknown,
}

#[derive(Type, Debug, Deserialize)]
struct CreateInstance {
    #[specta(optional)]
    group: Option<FEGroupId>,
    name: String,
    use_loaded_icon: bool,
    version: CreateInstanceVersion,
    notes: String,
    #[specta(optional)]
    icon_url: Option<String>,
}

#[derive(Type, Debug, Deserialize)]
struct ChangeModpack {
    instance: FEInstanceId,
    modpack: Modpack,
}

#[derive(Type, Debug, Deserialize)]
struct RepairModpack {
    instance: FEInstanceId,
    /// User-ticked untracked paths from the repair preview
    /// (`RepairModpack/index.tsx`'s `ticked()` set) — packinfo-style keys,
    /// empty when the user ticked nothing.
    cleanup_paths: Vec<String>,
    re_enable_disabled: bool,
}

#[derive(Type, Debug, Deserialize)]
struct RepairPreviewArgs {
    instance: FEInstanceId,
}

/// Result of [`GET_REPAIR_PREVIEW`](keys::instance::GET_REPAIR_PREVIEW): what
/// [`manager::modpack::ManagerRef::repair_preview`] would do, computed
/// read-only against the recorded `packinfo.json` — never the network. See
/// that function's own docs for the preview/execution asymmetry this
/// implies. `with_re_enable`/`without_re_enable` carry both
/// `re_enable_disabled` outcomes over the one disk scan the query performs —
/// `RepairModpack/index.tsx` picks between them client-side as its checkbox
/// is toggled, so a query keyed only on `instance` never needs to re-fetch
/// for that.
#[derive(Type, Debug, Serialize)]
pub struct FERepairPreview {
    pub has_packinfo: bool,
    pub with_re_enable: FERepairPlanVariant,
    pub without_re_enable: FERepairPlanVariant,
    pub untracked: Vec<FEUntrackedFile>,
    pub duplicates: Vec<FEDuplicateGroup>,
}

impl From<manager::modpack::RepairPreview> for FERepairPreview {
    fn from(value: manager::modpack::RepairPreview) -> Self {
        Self {
            has_packinfo: value.has_packinfo,
            with_re_enable: value.with_re_enable.into(),
            without_re_enable: value.without_re_enable.into(),
            untracked: value
                .untracked
                .into_iter()
                .map(FEUntrackedFile::from)
                .collect(),
            duplicates: value
                .duplicates
                .into_iter()
                .map(FEDuplicateGroup::from)
                .collect(),
        }
    }
}

/// One `re_enable_disabled` setting's worth of [`FERepairPreview`] — see
/// [`manager::modpack::RepairPlanVariant`].
#[derive(Type, Debug, Serialize)]
pub struct FERepairPlanVariant {
    /// Full expandable list, path-sorted.
    pub entries: Vec<FERepairEntry>,
    pub counts: FERepairCounts,
}

impl From<manager::modpack::RepairPlanVariant> for FERepairPlanVariant {
    fn from(value: manager::modpack::RepairPlanVariant) -> Self {
        Self {
            entries: value.entries.into_iter().map(FERepairEntry::from).collect(),
            counts: value.counts.into(),
        }
    }
}

#[derive(Type, Debug, Serialize)]
pub struct FERepairEntry {
    pub path: String,
    pub action: FERepairAction,
    pub reason: FERepairReason,
}

impl From<manager::modpack::apply_plan::PlanEntry> for FERepairEntry {
    fn from(value: manager::modpack::apply_plan::PlanEntry) -> Self {
        Self {
            path: value.path,
            action: value.action.into(),
            reason: value.reason.into(),
        }
    }
}

#[derive(Type, Debug, Serialize)]
pub enum FERepairAction {
    Replace,
    Create,
    Delete,
    Keep,
    ReplaceDisabled,
    ReEnable,
}

impl From<manager::modpack::apply_plan::PlanAction> for FERepairAction {
    fn from(value: manager::modpack::apply_plan::PlanAction) -> Self {
        use manager::modpack::apply_plan::PlanAction;
        match value {
            PlanAction::Replace => Self::Replace,
            PlanAction::Create => Self::Create,
            PlanAction::Delete => Self::Delete,
            PlanAction::Keep => Self::Keep,
            PlanAction::ReplaceDisabled => Self::ReplaceDisabled,
            PlanAction::ReEnable => Self::ReEnable,
        }
    }
}

/// Mirrors `apply_plan::PlanReason` but without its per-variant hash payload
/// — the preview surfaces only which bucket a path fell into, not the raw
/// md5 diff.
#[derive(Type, Debug, Serialize)]
pub enum FERepairReason {
    PackUpdate,
    Unchanged,
    ModifiedByUser,
    DeletedByUser,
    DisabledByUser,
    InSaveFolder,
    PackDropped,
    DroppedButModified,
    PreservedExisting,
    RepairOverwrote,
    RepairRestored,
    ReEnabled,
    CaseAliasedByTarget,
    /// VersionChange-only — repair (the only mode this FE type is ever
    /// actually populated from) never produces it, but `PlanReason` is one
    /// enum shared by both modes, so this arm still has to exist.
    DisabledReplaceResumed,
}

impl From<manager::modpack::apply_plan::PlanReason> for FERepairReason {
    fn from(value: manager::modpack::apply_plan::PlanReason) -> Self {
        use manager::modpack::apply_plan::PlanReason;
        match value {
            PlanReason::PackUpdate => Self::PackUpdate,
            PlanReason::Unchanged => Self::Unchanged,
            PlanReason::ModifiedByUser { .. } => Self::ModifiedByUser,
            PlanReason::DeletedByUser => Self::DeletedByUser,
            PlanReason::DisabledByUser => Self::DisabledByUser,
            PlanReason::InSaveFolder => Self::InSaveFolder,
            PlanReason::PackDropped => Self::PackDropped,
            PlanReason::DroppedButModified { .. } => Self::DroppedButModified,
            PlanReason::PreservedExisting => Self::PreservedExisting,
            PlanReason::RepairOverwrote { .. } => Self::RepairOverwrote,
            PlanReason::RepairRestored => Self::RepairRestored,
            PlanReason::ReEnabled => Self::ReEnabled,
            PlanReason::CaseAliasedByTarget { .. } => Self::CaseAliasedByTarget,
            PlanReason::DisabledReplaceResumed => Self::DisabledReplaceResumed,
        }
    }
}

#[derive(Type, Debug, Serialize)]
pub struct FERepairCounts {
    pub restore_modified: u32,
    pub restore_deleted: u32,
    pub unchanged: u32,
    pub disabled_kept: u32,
    pub re_enabled: u32,
    pub stale_dropped: u32,
    pub saves_skipped: u32,
}

impl From<manager::modpack::RepairCounts> for FERepairCounts {
    fn from(value: manager::modpack::RepairCounts) -> Self {
        Self {
            restore_modified: value.restore_modified,
            restore_deleted: value.restore_deleted,
            unchanged: value.unchanged,
            disabled_kept: value.disabled_kept,
            re_enabled: value.re_enabled,
            stale_dropped: value.stale_dropped,
            saves_skipped: value.saves_skipped,
        }
    }
}

#[derive(Type, Debug, Serialize)]
pub struct FEUntrackedFile {
    pub path: String,
    /// `u64` doesn't map cleanly through specta — same `f64` pattern
    /// `Mod::file_size` uses above.
    pub size: f64,
    pub label: FEUntrackedLabel,
    /// Whether ticking this path for cleanup would actually remove it —
    /// see `manager::modpack::UntrackedFile::deletable`. Always `true` for
    /// `Unknown`; for `DisabledPackFile` this depends on whether the
    /// tracked path's enabled copy still coexists on disk.
    pub deletable: bool,
    pub origin: Option<FEOriginVerdict>,
}

impl From<manager::modpack::UntrackedFile> for FEUntrackedFile {
    fn from(value: manager::modpack::UntrackedFile) -> Self {
        Self {
            path: value.path,
            size: value.size as f64,
            label: value.label.into(),
            deletable: value.deletable,
            origin: value.origin.map(FEOriginVerdict::from),
        }
    }
}

#[derive(Type, Debug, Serialize)]
pub enum FEUntrackedLabel {
    Unknown,
    DisabledPackFile,
}

impl From<manager::modpack::UntrackedLabel> for FEUntrackedLabel {
    fn from(value: manager::modpack::UntrackedLabel) -> Self {
        match value {
            manager::modpack::UntrackedLabel::Unknown => Self::Unknown,
            manager::modpack::UntrackedLabel::DisabledPackFile => Self::DisabledPackFile,
        }
    }
}

/// Populated once `instance.checkPackOrigin` has completed a run for the
/// instance — `None` on an untracked file until then, or if that run never
/// assigned this exact path a verdict. See `manager::modpack::origin_verdict_for`.
#[derive(Type, Debug, Serialize)]
pub enum FEOriginVerdict {
    ShippedIn {
        version_name: String,
        version_id: String,
    },
    CurrentVersion,
    Unknown,
}

impl From<manager::modpack::OriginVerdict> for FEOriginVerdict {
    fn from(value: manager::modpack::OriginVerdict) -> Self {
        match value {
            manager::modpack::OriginVerdict::ShippedIn {
                version_name,
                version_id,
            } => Self::ShippedIn {
                version_name,
                version_id,
            },
            manager::modpack::OriginVerdict::CurrentVersion => Self::CurrentVersion,
            manager::modpack::OriginVerdict::Unknown => Self::Unknown,
        }
    }
}

#[derive(Type, Debug, Serialize)]
pub struct FEDuplicateGroup {
    pub modid: String,
    pub files: Vec<FEDuplicateSide>,
}

impl From<manager::modpack::DuplicateGroup> for FEDuplicateGroup {
    fn from(value: manager::modpack::DuplicateGroup) -> Self {
        Self {
            modid: value.modid,
            files: value.files.into_iter().map(FEDuplicateSide::from).collect(),
        }
    }
}

#[derive(Type, Debug, Serialize)]
pub struct FEDuplicateSide {
    pub path: String,
    pub pack_owned: bool,
    pub enabled: bool,
}

impl From<manager::modpack::DuplicateSide> for FEDuplicateSide {
    fn from(value: manager::modpack::DuplicateSide) -> Self {
        Self {
            path: value.path,
            pack_owned: value.pack_owned,
            enabled: value.enabled,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
enum FEJavaOverride {
    Profile(Option<String>),
    Path(Option<String>),
}

impl From<domain::info::JavaOverride> for FEJavaOverride {
    fn from(value: domain::info::JavaOverride) -> Self {
        match value {
            domain::info::JavaOverride::Profile(p) => Self::Profile(p),
            domain::info::JavaOverride::Path(p) => Self::Path(p),
        }
    }
}

impl From<FEJavaOverride> for domain::info::JavaOverride {
    fn from(value: FEJavaOverride) -> Self {
        match value {
            FEJavaOverride::Profile(p) => Self::Profile(p),
            FEJavaOverride::Path(p) => Self::Path(p),
        }
    }
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FEUpdateInstance {
    instance: FEInstanceId,
    #[specta(optional)]
    name: Option<Set<String>>,
    #[specta(optional)]
    use_loaded_icon: Option<Set<bool>>,
    #[specta(optional)]
    notes: Option<Set<String>>,
    #[specta(optional)]
    version: Option<Set<String>>,
    #[specta(optional)]
    modloader: Option<Set<Option<ModLoader>>>,
    #[specta(optional)]
    java_override: Option<Set<Option<FEJavaOverride>>>,
    #[specta(optional)]
    global_java_args: Option<Set<bool>>,
    #[specta(optional)]
    extra_java_args: Option<Set<Option<String>>>,
    #[specta(optional)]
    memory: Option<Set<Option<MemoryRange>>>,
    #[specta(optional)]
    pre_launch_hook: Option<Set<Option<String>>>,
    #[specta(optional)]
    post_exit_hook: Option<Set<Option<String>>>,
    #[specta(optional)]
    wrapper_command: Option<Set<Option<String>>>,
    #[specta(optional)]
    game_resolution: Option<Set<Option<GameResolution>>>,
    #[specta(optional)]
    mod_sources: Option<Set<Option<ModSources>>>,
    #[specta(optional)]
    modpack_locked: Option<Set<Option<bool>>>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FELaunchInstanceArgs {
    id: FEInstanceId,
    #[serde(default)]
    skip_memory_check: bool,
}

#[derive(Type, Debug, Deserialize)]
struct DuplicateInstance {
    instance: FEInstanceId,
    new_name: String,
}

#[derive(Type, Debug, Deserialize)]
struct SetFavorite {
    instance: FEInstanceId,
    favorite: bool,
}

#[derive(Type, Debug, Deserialize)]
struct InstanceMod {
    instance_id: FEInstanceId,
    mod_id: String,
}

#[derive(Type, Debug, Deserialize)]
enum ModSource {
    Curseforge(CurseforgeMod),
    Modrinth(ModrinthMod),
}

#[derive(Type, Debug, Deserialize)]
enum ModSourceType {
    Curseforge,
    Modrinth,
}

#[derive(Type, Debug, Deserialize)]
struct CurseforgeMod {
    project_id: u32,
    file_id: u32,
}

#[derive(Type, Debug, Deserialize)]
struct ModrinthMod {
    project_id: String,
    version_id: String,
}

#[derive(Type, Debug, Deserialize)]
struct InstallMod {
    instance_id: FEInstanceId,
    mod_source: ModSource,
    install_deps: bool,
    replaces_mod: Option<String>,
}

#[derive(Type, Debug, Deserialize)]
struct UpdateMod {
    instance_id: FEInstanceId,
    mod_id: String,
}

#[derive(Type, Debug, Deserialize)]
struct InstallLatestMod {
    instance_id: FEInstanceId,
    mod_source: LatestModSource,
}

#[derive(Type, Debug, Deserialize)]
enum LatestModSource {
    Curseforge(u32),
    Modrinth(String),
}

#[derive(Type, Debug, Serialize, Deserialize)]
struct GameLogId(i32);

#[derive(Type, Debug, Serialize)]
struct GameLogEntry {
    id: GameLogId,
    instance_id: FEInstanceId,
    active: bool,
    timestamp: String,
    file_size: Option<f64>,
}

#[derive(Type, Debug, Deserialize)]
enum CreateInstanceVersion {
    Version(GameVersion),
    Modpack(Modpack),
}

#[derive(Type, Debug, Deserialize)]
enum GameVersion {
    Standard(StandardVersion),
    // Custom(json)
}

#[derive(Type, Debug, Serialize, Deserialize)]
struct ModpackInfo {
    modpack: Modpack,
    locked: bool,
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "value")]
enum Modpack {
    Curseforge(CurseforgeModpack),
    Modrinth(ModrinthModpack),
}

#[derive(Type, Debug, Serialize, Deserialize)]
struct CurseforgeModpack {
    project_id: u32,
    file_id: u32,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct ModrinthModpack {
    pub project_id: String,
    pub version_id: String,
}

#[derive(Type, Debug, Deserialize)]
struct StandardVersion {
    release: String,
    modloaders: HashSet<ModLoader>,
}

#[derive(Type, Debug, Deserialize)]
enum MoveGroupTarget {
    BeforeGroup(FEGroupId),
    BeforeInstance(FEInstanceId), // Instance must be in default group (ungrouped)
    EndOfLibrary,
}

#[derive(Type, Debug, Deserialize)]
struct MoveGroup {
    group: FEGroupId,
    target: MoveGroupTarget,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateFolderFromInstances {
    instances: Vec<FEInstanceId>,
    #[specta(optional)]
    target_instance_id: Option<FEInstanceId>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum LibrarySortCriteria {
    Name,
    LastPlayed,
    MostPlayed,
    DateCreated,
}

impl From<LibrarySortCriteria> for manager::LibrarySortCriteria {
    fn from(value: LibrarySortCriteria) -> Self {
        match value {
            LibrarySortCriteria::Name => Self::Name,
            LibrarySortCriteria::LastPlayed => Self::LastPlayed,
            LibrarySortCriteria::MostPlayed => Self::MostPlayed,
            LibrarySortCriteria::DateCreated => Self::DateCreated,
        }
    }
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArrangeGroup {
    group: FEGroupId,
    sort_by: LibrarySortCriteria,
}

#[derive(Type, Debug, Deserialize)]
struct RenameGroup {
    group: FEGroupId,
    name: String,
}

#[derive(Type, Debug, Deserialize)]
struct MoveInstance {
    instance: FEInstanceId,
    target: MoveInstanceTarget,
}

#[derive(Type, Debug, Deserialize)]
enum MoveInstanceTarget {
    BeforeInstance(FEInstanceId),
    BeginningOfGroup(FEGroupId),
    EndOfGroup(FEGroupId),
    BeforeGroup(FEGroupId), // Position instance before a folder (at library root level)
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum GameResolution {
    Standard(u16, u16),
    Custom(u16, u16),
}

impl From<domain::info::GameResolution> for GameResolution {
    fn from(value: domain::info::GameResolution) -> Self {
        match value {
            domain::info::GameResolution::Standard(w, h) => Self::Standard(w, h),
            domain::info::GameResolution::Custom(w, h) => Self::Custom(w, h),
        }
    }
}

impl From<GameResolution> for domain::info::GameResolution {
    fn from(value: GameResolution) -> Self {
        match value {
            GameResolution::Standard(w, h) => Self::Standard(w, h),
            GameResolution::Custom(w, h) => Self::Custom(w, h),
        }
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstanceDetails {
    id: FEInstanceId,
    name: String,
    favorite: bool,
    version: Option<String>,
    // is_being_cached: bool,
    modpack: Option<ModpackInfo>,
    global_java_args: bool,
    extra_java_args: Option<String>,
    memory: Option<MemoryRange>,
    game_resolution: Option<GameResolution>,
    last_played: Option<DateTime<Utc>>,
    seconds_played: u32,
    modloaders: Vec<ModLoader>,
    java_override: Option<FEJavaOverride>,
    required_java_profile: Option<String>,
    java_override_mismatch: bool,
    pre_launch_hook: Option<String>,
    post_exit_hook: Option<String>,
    wrapper_command: Option<String>,
    notes: String,
    state: LaunchState,
    icon_revision: Option<u32>,
    has_pack_update: bool,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct FEInstanceModpackInfo {
    pub name: String,
    pub version_name: String,
    pub url_slug: String,
    pub has_image: bool,
}

impl From<InstanceModpackInfo> for FEInstanceModpackInfo {
    fn from(value: InstanceModpackInfo) -> Self {
        Self {
            name: value.name,
            version_name: value.version_name,
            url_slug: value.url_slug,
            has_image: value.has_image,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct MemoryRange {
    pub min_mb: u16,
    pub max_mb: u16,
}

#[derive(Type, Debug, Deserialize)]
struct OpenInstanceFolder {
    instance_id: FEInstanceId,
    folder: InstanceFolder,
}

#[derive(Type, Debug, Deserialize)]
struct OpenLogInFolder {
    instance_id: FEInstanceId,
    log_id: GameLogId,
}

#[derive(Type, Debug, Deserialize)]
enum InstanceFolder {
    Root,
    Data,
    Mods,
    Configs,
    Screenshots,
    Saves,
    Logs,
    CrashReports,
    ResourcePacks,
    TexturePacks,
    ShaderPacks,
}

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct ModLoader {
    type_: FEInstanceModloaderType,
    version: String,
}

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
enum FEInstanceModloaderType {
    Neoforge,
    Forge,
    Fabric,
    Quilt,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "state", content = "value")]
enum LaunchState {
    Inactive {
        failed_task: Option<FETaskId>,
    },
    Queued(FETaskId),
    Preparing(FETaskId),
    Running {
        start_time: DateTime<Utc>,
        /// `null` for an adopted session — see `adopted`.
        log_id: Option<i32>,
        /// Launched by a previous launcher session: this core does not own the
        /// process, so there is no live log to open and no exit code to
        /// report. Stop still works, through the pid.
        adopted: bool,
    },
    Deleting,
}

#[derive(Type, Debug, Serialize)]
struct Mod {
    id: String,
    filename: String,
    enabled: bool,
    addon_type: domain::AddonType,
    metadata: Option<ModFileMetadata>,
    curseforge: Option<CurseForgeModMetadata>,
    modrinth: Option<ModrinthModMetadata>,
    has_update: bool,
    is_duplicate: bool,
    file_size: f64,
}

#[derive(Type, Debug, Serialize)]
struct ModFileMetadata {
    id: String,
    modid: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    authors: Option<String>,
    modloaders: Vec<FEInstanceModloaderType>,
    sha_1: String,
    sha_512: String,
    murmur_2: String,
    has_image: bool,
}

#[derive(Type, Serialize, Debug)]
struct CurseForgeModMetadata {
    project_id: u32,
    file_id: u32,
    name: String,
    version: String,
    urlslug: String,
    summary: String,
    authors: String,
    has_image: bool,
}

#[derive(Type, Serialize, Debug)]
struct ModrinthModMetadata {
    project_id: String,
    version_id: String,
    title: String,
    version: String,
    urlslug: String,
    description: String,
    authors: String,
    has_image: bool,
}

#[derive(Type, Deserialize, Debug)]
struct ExploreQuery {
    instance_id: FEInstanceId,
    path: Vec<String>,
}

#[derive(Type, Serialize, Debug)]
struct ExploreEntry {
    name: String,
    #[serde(rename = "type")]
    type_: ExploreEntryType,
}

#[derive(Type, Serialize, Debug)]
enum ExploreEntryType {
    File { size: u32 },
    Directory,
}

#[derive(Type, Deserialize, Debug)]
struct ExportEntry {
    //#[serde(flatten)]
    entries: HashMap<String, Option<ExportEntry>>,
}

#[derive(Type, Deserialize, Debug)]
enum ExportTarget {
    Curseforge,
    Modrinth,
    Gdlauncher,
}

#[derive(Type, Deserialize, Debug)]
struct ExportArgs {
    instance_id: FEInstanceId,
    target: ExportTarget,
    save_path: String,
    self_contained_addons_bundling: bool,
    filter: ExportEntry,
    version: String,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub enum ImportEntity {
    LegacyGDLauncher,
    MRPack,
    Modrinth,
    CurseForgeZip,
    CurseForge,
    ATLauncher,
    Technic,
    FTB,
    MultiMC,
    PrismLauncher,
    GDLPack,
}

#[derive(Type, Debug, Serialize)]
struct ImportableInstance {
    filename: String,
    instance_name: String,
}

#[derive(Type, Debug, Serialize)]
struct InvalidImportEntry {
    name: String,
    reason: Translation,
}

#[derive(Type, Debug, Serialize)]
enum ImportEntry {
    Valid(ImportableInstance),
    Invalid(InvalidImportEntry),
}

#[derive(Type, Debug, Serialize)]
enum ImportScanStatus {
    NoResults,
    SingleResult(ImportEntry),
    MultiResult(Vec<ImportEntry>),
}

#[derive(Type, Debug, Serialize)]
struct FullImportScanStatus {
    scanning: bool,
    status: ImportScanStatus,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum ImportEntitySelectionType {
    File,
    Directory,
}

impl From<importer::SelectionType> for ImportEntitySelectionType {
    fn from(value: importer::SelectionType) -> Self {
        match value {
            importer::SelectionType::File => Self::File,
            importer::SelectionType::Directory => Self::Directory,
        }
    }
}

#[derive(Type, Debug, Serialize)]
struct ImportEntityStatus {
    entity: ImportEntity,
    supported: bool,
    selection_type: ImportEntitySelectionType,
}

#[derive(Type, Debug, Deserialize)]
struct ImportRequest {
    index: u32,
    name: Option<String>,
}

impl From<domain::InstanceDetails> for InstanceDetails {
    fn from(value: domain::InstanceDetails) -> Self {
        Self {
            id: value.id.into(),
            favorite: value.favorite,
            name: value.name,
            version: value.version,
            // is_being_cached: value.is_being_cached,
            modpack: value.modpack.map(Into::into),
            global_java_args: value.global_java_args,
            extra_java_args: value.extra_java_args,
            memory: value.memory.map(Into::into),
            game_resolution: value.game_resolution.map(Into::into),
            last_played: value.last_played,
            seconds_played: value.seconds_played,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
            java_override: value.java_override.map(Into::into),
            required_java_profile: value.required_java_profile,
            java_override_mismatch: value.java_override_mismatch,
            notes: value.notes,
            state: value.state.into(),
            icon_revision: value.icon_revision,
            has_pack_update: value.has_pack_update,
            pre_launch_hook: value.pre_launch_hook,
            post_exit_hook: value.post_exit_hook,
            wrapper_command: value.wrapper_command,
        }
    }
}

impl From<domain::info::ModLoader> for ModLoader {
    fn from(value: domain::info::ModLoader) -> Self {
        Self {
            type_: value.type_.into(),
            version: value.version,
        }
    }
}

impl From<domain::info::ModLoaderType> for FEInstanceModloaderType {
    fn from(value: domain::info::ModLoaderType) -> Self {
        use domain::info::ModLoaderType as domain;

        match value {
            domain::Neoforge => Self::Neoforge,
            domain::Forge => Self::Forge,
            domain::Fabric => Self::Fabric,
            domain::Quilt => Self::Quilt,
        }
    }
}

impl TryFrom<CreateInstanceVersion> for manager::InstanceVersionSource {
    type Error = anyhow::Error;

    fn try_from(value: CreateInstanceVersion) -> anyhow::Result<Self> {
        Ok(match value {
            CreateInstanceVersion::Version(v) => Self::Version(v.try_into()?),
            CreateInstanceVersion::Modpack(m) => Self::Modpack(m.into(), true),
        })
    }
}

impl TryFrom<GameVersion> for domain::info::GameVersion {
    type Error = anyhow::Error;

    fn try_from(value: GameVersion) -> anyhow::Result<Self> {
        match value {
            GameVersion::Standard(v) => Ok(Self::Standard(v.try_into()?)),
        }
    }
}

impl From<ModpackInfo> for domain::info::ModpackInfo {
    fn from(value: ModpackInfo) -> Self {
        Self {
            modpack: value.modpack.into(),
            locked: value.locked,
        }
    }
}

impl From<Modpack> for domain::info::Modpack {
    fn from(value: Modpack) -> Self {
        match value {
            Modpack::Curseforge(m) => Self::Curseforge(m.into()),
            Modpack::Modrinth(m) => Self::Modrinth(m.into()),
        }
    }
}

impl From<CurseforgeModpack> for domain::info::CurseforgeModpack {
    fn from(value: CurseforgeModpack) -> Self {
        Self {
            project_id: value.project_id,
            file_id: value.file_id,
        }
    }
}

impl From<ModrinthModpack> for domain::info::ModrinthModpack {
    fn from(value: ModrinthModpack) -> Self {
        Self {
            project_id: value.project_id,
            version_id: value.version_id,
        }
    }
}

impl From<domain::info::ModpackInfo> for ModpackInfo {
    fn from(value: domain::info::ModpackInfo) -> Self {
        Self {
            modpack: value.modpack.into(),
            locked: value.locked,
        }
    }
}

impl From<domain::info::Modpack> for Modpack {
    fn from(value: domain::info::Modpack) -> Self {
        match value {
            domain::info::Modpack::Curseforge(m) => Self::Curseforge(m.into()),
            domain::info::Modpack::Modrinth(m) => Self::Modrinth(m.into()),
        }
    }
}

impl From<domain::info::CurseforgeModpack> for CurseforgeModpack {
    fn from(value: domain::info::CurseforgeModpack) -> Self {
        Self {
            project_id: value.project_id,
            file_id: value.file_id,
        }
    }
}

impl From<domain::info::ModrinthModpack> for ModrinthModpack {
    fn from(value: domain::info::ModrinthModpack) -> Self {
        Self {
            project_id: value.project_id,
            version_id: value.version_id,
        }
    }
}

impl TryFrom<StandardVersion> for domain::info::StandardVersion {
    type Error = anyhow::Error;

    fn try_from(value: StandardVersion) -> anyhow::Result<Self> {
        let mut modloaders = HashSet::new();

        for modloader in value.modloaders {
            modloaders.insert(modloader.try_into()?);
        }

        Ok(Self {
            release: value.release,
            modloaders,
        })
    }
}

impl TryFrom<ModLoader> for domain::info::ModLoader {
    type Error = anyhow::Error;

    fn try_from(value: ModLoader) -> anyhow::Result<Self> {
        if value.version.is_empty() {
            return Err(anyhow!("modloader version cannot be empty"));
        }

        Ok(Self {
            type_: value.type_.into(),
            version: value.version,
        })
    }
}

impl From<FEInstanceModloaderType> for domain::info::ModLoaderType {
    fn from(value: FEInstanceModloaderType) -> Self {
        match value {
            FEInstanceModloaderType::Neoforge => Self::Neoforge,
            FEInstanceModloaderType::Forge => Self::Forge,
            FEInstanceModloaderType::Fabric => Self::Fabric,
            FEInstanceModloaderType::Quilt => Self::Quilt,
        }
    }
}

impl From<manager::ListGroup> for ListGroup {
    fn from(value: manager::ListGroup) -> Self {
        Self {
            id: value.id.into(),
            name: value.name,
            library_position: value.library_position,
        }
    }
}

impl From<manager::ListInstance> for ListInstance {
    fn from(value: manager::ListInstance) -> Self {
        Self {
            id: value.id.into(),
            group_id: value.group_id.into(),
            index: value.index,
            library_position: value.library_position,
            name: value.name,
            favorite: value.favorite,
            status: value.status.into(),
            icon_revision: value.icon_revision,
            last_played: value.last_played,
            date_created: value.date_created,
            date_updated: value.date_updated,
            seconds_played: value.seconds_played,
            locked: value.locked,
        }
    }
}

impl From<manager::ListInstanceStatus> for ListInstanceStatus {
    fn from(value: manager::ListInstanceStatus) -> Self {
        match value {
            manager::ListInstanceStatus::Valid(status) => Self::Valid(status.into()),
            manager::ListInstanceStatus::Invalid(status) => Self::Invalid(status.into()),
        }
    }
}

impl From<manager::ValidListInstance> for ValidListInstance {
    fn from(value: manager::ValidListInstance) -> Self {
        Self {
            mc_version: value.mc_version,
            modloader: value.modloader.map(Into::into),
            modloader_version: value.modloader_version,
            modpack: value.modpack.map(Into::into),
            state: value.state.into(),
        }
    }
}

impl From<manager::InvalidListInstance> for InvalidListInstance {
    fn from(value: manager::InvalidListInstance) -> Self {
        use manager::InvalidListInstance as manager;

        match value {
            manager::JsonMissing => Self::JsonMissing,
            manager::JsonError(e) => Self::JsonError(e.into()),
            manager::Other(e) => Self::Other(e),
        }
    }
}

impl From<manager::ConfigurationParseError> for ConfigurationParseError {
    fn from(value: manager::ConfigurationParseError) -> Self {
        Self {
            type_: value.type_.into(),
            message: value.message,
            line: value.line,
            config_text: value.config_text,
        }
    }
}

impl From<manager::ConfigurationParseErrorType> for ConfigurationParseErrorType {
    fn from(value: manager::ConfigurationParseErrorType) -> Self {
        use manager::ConfigurationParseErrorType as manager;

        match value {
            manager::Syntax => Self::Syntax,
            manager::Data => Self::Data,
            manager::Eof => Self::Eof,
            manager::Unknown => Self::Unknown,
        }
    }
}

impl From<domain::LaunchState> for LaunchState {
    fn from(value: domain::LaunchState) -> Self {
        use domain::LaunchState as domain;

        match value {
            domain::Inactive { failed_task } => Self::Inactive {
                failed_task: failed_task.map(Into::into),
            },
            domain::Queued(task) => Self::Queued(task.into()),
            domain::Preparing(task) => Self::Preparing(task.into()),
            domain::Running {
                start_time,
                log_id,
                adopted,
            } => Self::Running {
                start_time,
                log_id: log_id.map(|id| id.0),
                adopted,
            },
            domain::Deleting => Self::Deleting,
        }
    }
}

impl From<domain::Mod> for Mod {
    fn from(value: domain::Mod) -> Self {
        Self {
            id: value.id,
            filename: value.filename,
            enabled: value.enabled,
            addon_type: value.addon_type,
            metadata: value.metadata.map(Into::into),
            curseforge: value.curseforge.map(Into::into),
            modrinth: value.modrinth.map(Into::into),
            has_update: value.has_update,
            is_duplicate: value.is_duplicate,
            file_size: value.file_size as f64,
        }
    }
}

impl From<domain::ModFileMetadata> for ModFileMetadata {
    fn from(value: domain::ModFileMetadata) -> Self {
        Self {
            id: value.id,
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
            sha_1: hex::encode(value.sha_1),
            sha_512: hex::encode(value.sha_512),
            murmur_2: value.murmur_2.to_string(),
            has_image: value.has_image,
        }
    }
}

impl From<domain::CurseForgeModMetadata> for CurseForgeModMetadata {
    fn from(value: domain::CurseForgeModMetadata) -> Self {
        Self {
            project_id: value.project_id,
            file_id: value.file_id,
            name: value.name,
            version: value.version,
            urlslug: value.urlslug,
            summary: value.summary,
            authors: value.authors,
            has_image: value.has_image,
        }
    }
}

impl From<domain::ModrinthModMetadata> for ModrinthModMetadata {
    fn from(value: domain::ModrinthModMetadata) -> Self {
        Self {
            project_id: value.project_id,
            version_id: value.version_id,
            title: value.title,
            version: value.version,
            urlslug: value.urlslug,
            description: value.description,
            authors: value.authors,
            has_image: value.has_image,
        }
    }
}

impl From<domain::GameLogId> for GameLogId {
    fn from(value: domain::GameLogId) -> Self {
        Self(value.0)
    }
}

impl From<GameLogId> for domain::GameLogId {
    fn from(value: GameLogId) -> Self {
        Self(value.0)
    }
}

impl From<domain::GameLogEntry> for GameLogEntry {
    fn from(value: domain::GameLogEntry) -> Self {
        Self {
            id: value.id.into(),
            instance_id: value.instance_id.into(),
            active: value.active,
            timestamp: value.datetime.timestamp_millis().to_string(),
            file_size: value.file_size.map(|s| s as f64),
        }
    }
}

impl From<InstanceFolder> for domain::InstanceFolder {
    fn from(value: InstanceFolder) -> Self {
        match value {
            InstanceFolder::Root => Self::Root,
            InstanceFolder::Data => Self::Data,
            InstanceFolder::Mods => Self::Mods,
            InstanceFolder::Configs => Self::Configs,
            InstanceFolder::Screenshots => Self::Screenshots,
            InstanceFolder::Saves => Self::Saves,
            InstanceFolder::Logs => Self::Logs,
            InstanceFolder::CrashReports => Self::CrashReports,
            InstanceFolder::ResourcePacks => Self::ResourcePacks,
            InstanceFolder::TexturePacks => Self::TexturePacks,
            InstanceFolder::ShaderPacks => Self::ShaderPacks,
        }
    }
}

impl From<(u16, u16)> for MemoryRange {
    fn from(value: (u16, u16)) -> Self {
        Self {
            min_mb: value.0,
            max_mb: value.1,
        }
    }
}

impl From<MemoryRange> for (u16, u16) {
    fn from(value: MemoryRange) -> Self {
        (value.min_mb, value.max_mb)
    }
}

impl TryFrom<FEUpdateInstance> for domain::InstanceSettingsUpdate {
    type Error = anyhow::Error;

    fn try_from(value: FEUpdateInstance) -> anyhow::Result<Self> {
        Ok(Self {
            instance_id: value.instance.into(),
            name: value.name.map(|x| x.inner()),
            use_loaded_icon: value.use_loaded_icon.map(|x| x.inner()),
            notes: value.notes.map(|x| x.inner()),
            version: value.version.map(|x| x.inner()),
            modloader: value
                .modloader
                .map(|x| x.inner().and_then(|v| v.try_into().ok())),
            java_override: value.java_override.map(|x| x.inner().map(Into::into)),
            global_java_args: value.global_java_args.map(|x| x.inner()),
            extra_java_args: value.extra_java_args.map(|x| x.inner()),
            memory: value.memory.map(|x| x.inner().map(Into::into)),
            pre_launch_hook: value.pre_launch_hook.map(|x| x.inner()),
            post_exit_hook: value.post_exit_hook.map(|x| x.inner()),
            wrapper_command: value.wrapper_command.map(|x| x.inner()),
            game_resolution: value.game_resolution.map(|x| x.inner().map(Into::into)),
            mod_sources: value.mod_sources.map(|x| x.inner().map(Into::into)),
            modpack_locked: value.modpack_locked.map(|x| x.inner()),
        })
    }
}

impl From<domain::ExploreEntry> for ExploreEntry {
    fn from(value: domain::ExploreEntry) -> Self {
        Self {
            name: value.name,
            type_: value.type_.into(),
        }
    }
}

impl From<ImportEntity> for importer::Entity {
    fn from(entity: ImportEntity) -> Self {
        match entity {
            ImportEntity::LegacyGDLauncher => Self::LegacyGDLauncher,
            ImportEntity::MRPack => Self::MRPack,
            ImportEntity::Modrinth => Self::Modrinth,
            ImportEntity::CurseForgeZip => Self::CurseForgeZip,
            ImportEntity::CurseForge => Self::CurseForge,
            ImportEntity::ATLauncher => Self::ATLauncher,
            ImportEntity::Technic => Self::Technic,
            ImportEntity::FTB => Self::FTB,
            ImportEntity::MultiMC => Self::MultiMC,
            ImportEntity::PrismLauncher => Self::PrismLauncher,
            ImportEntity::GDLPack => Self::GDLPack,
        }
    }
}

impl From<importer::Entity> for ImportEntity {
    fn from(entity: importer::Entity) -> Self {
        use importer::Entity as backend;

        match entity {
            backend::LegacyGDLauncher => Self::LegacyGDLauncher,
            backend::MRPack => Self::MRPack,
            backend::Modrinth => Self::Modrinth,
            backend::CurseForgeZip => Self::CurseForgeZip,
            backend::CurseForge => Self::CurseForge,
            backend::ATLauncher => Self::ATLauncher,
            backend::Technic => Self::Technic,
            backend::FTB => Self::FTB,
            backend::MultiMC => Self::MultiMC,
            backend::PrismLauncher => Self::PrismLauncher,
            backend::GDLPack => Self::GDLPack,
        }
    }
}

impl From<importer::ImportableInstance> for ImportableInstance {
    fn from(value: importer::ImportableInstance) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.instance_name,
        }
    }
}

impl From<importer::InvalidImportEntry> for InvalidImportEntry {
    fn from(value: importer::InvalidImportEntry) -> Self {
        Self {
            name: value.name,
            reason: value.reason,
        }
    }
}

impl From<domain::ExploreEntryType> for ExploreEntryType {
    fn from(value: domain::ExploreEntryType) -> Self {
        match value {
            domain::ExploreEntryType::File { size } => Self::File { size },
            domain::ExploreEntryType::Directory => Self::Directory,
        }
    }
}

impl From<importer::ImportEntry> for ImportEntry {
    fn from(value: importer::ImportEntry) -> Self {
        match value {
            importer::ImportEntry::Valid(v) => Self::Valid(v.into()),
            importer::ImportEntry::Invalid(v) => Self::Invalid(v.into()),
        }
    }
}

impl From<ExportTarget> for domain::ExportTarget {
    fn from(value: ExportTarget) -> Self {
        match value {
            ExportTarget::Curseforge => Self::Curseforge,
            ExportTarget::Modrinth => Self::Modrinth,
            ExportTarget::Gdlauncher => Self::Gdlauncher,
        }
    }
}

impl From<importer::ImportScanStatus> for ImportScanStatus {
    fn from(value: importer::ImportScanStatus) -> Self {
        use importer::ImportScanStatus as domain;

        match value {
            domain::NoResults => Self::NoResults,
            domain::SingleResult(r) => Self::SingleResult(r.into()),
            domain::MultiResult(r) => Self::MultiResult(r.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<ExportEntry> for domain::ExportEntry {
    fn from(value: ExportEntry) -> Self {
        Self(
            value
                .entries
                .into_iter()
                .map(|(k, v)| (k, v.map(Into::into)))
                .collect(),
        )
    }
}

impl From<importer::FullImportScanStatus> for FullImportScanStatus {
    fn from(value: importer::FullImportScanStatus) -> Self {
        Self {
            scanning: value.scanning,
            status: value.status.into(),
        }
    }
}

mod log {
    use axum::extract::{WebSocketUpgrade, ws::Message};
    use tracing::{error, trace};

    use super::*;

    #[derive(Debug, Deserialize)]
    pub struct LogQuery {
        id: i32,
    }

    #[tracing::instrument(skip(app))]
    pub async fn log_handler(
        Query(query): Query<LogQuery>,
        req: WebSocketUpgrade,
        State(app): State<App>,
    ) -> impl IntoResponse {
        req.on_upgrade(move |mut socket| async move {
            let log_rx = app
                .instance_manager()
                .get_log(domain::GameLogId(query.id))
                .await;

            let Ok(mut log_rx) = log_rx else {
                tracing::warn!("log entry not found");

                socket.send(Message::Text(r#"{"init":"notfound"}"#.to_string()));
                return;
            };

            socket.send(Message::Text(r#"{"init":"found"}"#.to_string()));

            tracing::trace!("starting log stream");

            let mut last_idx = 0;

            loop {
                tracing::trace!("waiting for log data to come in");

                let new_lines = {
                    let log = log_rx.borrow();

                    let new_lines = log
                        .get_span(last_idx..)
                        .into_iter()
                        .map(|entry| entry.clone())
                        .collect::<Vec<_>>();

                    last_idx = log.len();

                    new_lines
                };

                if let Err(e) = socket
                    .send(Message::Text(
                        serde_json::to_string(&new_lines)
                            .expect("serialization of a log entry should be infallible"),
                    ))
                    .await
                {
                    error!(?e, "Failed to send log entry");
                }

                if let Err(_) = log_rx.changed().await {
                    trace!("`log_rx` was closed, killing log stream");

                    return;
                }
            }
        })
    }
}

#[derive(Debug, Deserialize, Type)]
pub struct SearchLogsQuery {
    log_id: i32,
    query: String,
    match_case: bool,
    match_whole_word: bool,
    use_regex: bool,
}

#[derive(Debug, Type, Serialize)]
struct FESearchResult {
    pub entry_index: u32,
    pub pos: u32,
    pub len: u32,
}

impl From<SearchResult> for FESearchResult {
    fn from(value: SearchResult) -> Self {
        Self {
            entry_index: value.entry_index as u32,
            pos: value.pos as u32,
            len: value.len as u32,
        }
    }
}

impl From<FESearchResult> for SearchResult {
    fn from(value: FESearchResult) -> Self {
        Self {
            entry_index: value.entry_index as usize,
            pos: value.pos as usize,
            len: value.len as usize,
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
struct ShareInstanceQuery {
    instance_id: FEInstanceId,
    title: Option<String>,
    expiration_days: Option<i32>,
    max_downloads: Option<i32>,
    #[serde(default)]
    include_saves: bool,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
enum FEShareInstanceProgress {
    Progress(i32),
    Finished(String),
    Error {
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<FEShareErrorDetails>,
    },
}

/// Extra machine-readable context for a share failure. Currently only set for
/// `INSTANCE_TOO_LARGE`, so the UI can show the size, the cap, and which
/// folders are responsible.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEShareErrorDetails {
    total_bytes: f64,
    limit_bytes: f64,
    largest_folders: Vec<FEShareFolderSize>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEShareFolderSize {
    name: String,
    bytes: f64,
}

impl From<&InstanceTooLargeError> for FEShareErrorDetails {
    fn from(err: &InstanceTooLargeError) -> Self {
        Self {
            total_bytes: err.total_bytes as f64,
            limit_bytes: err.limit_bytes as f64,
            largest_folders: err
                .largest_folders
                .iter()
                .map(|f| FEShareFolderSize {
                    name: f.name.clone(),
                    bytes: f.bytes as f64,
                })
                .collect(),
        }
    }
}

impl From<ShareInstanceProgress> for FEShareInstanceProgress {
    fn from(value: ShareInstanceProgress) -> Self {
        match value {
            ShareInstanceProgress::Progress(p) => Self::Progress(p),
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
struct ImportShareCodeQuery {
    share_code: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
enum FEImportShareCodeProgress {
    Progress(i32),
    Finished(String),
    Error { code: String, message: String },
}

impl From<ImportShareCodeProgress> for FEImportShareCodeProgress {
    fn from(value: ImportShareCodeProgress) -> Self {
        match value {
            ImportShareCodeProgress::Progress(p) => Self::Progress(p),
        }
    }
}

#[derive(Debug, Serialize, Type)]
struct FEWaitForInstanceShareResponse {
    share_code: String,
    expires_at: DateTime<Utc>,
}

impl From<WaitForShareInstanceResponse> for FEWaitForInstanceShareResponse {
    fn from(value: WaitForShareInstanceResponse) -> Self {
        Self {
            share_code: value.share_code,
            expires_at: value.expires_at,
        }
    }
}

// Individual mod data for share preview
#[derive(Debug, Serialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
struct FESharedMod {
    name: String,
    curseforge_project_id: Option<i32>,
    curseforge_file_id: Option<i32>,
    curseforge_slug: Option<String>,
    modrinth_project_id: Option<String>,
    modrinth_version_id: Option<String>,
    modrinth_slug: Option<String>,
}

impl From<SharedMod> for FESharedMod {
    fn from(value: SharedMod) -> Self {
        Self {
            name: value.name,
            curseforge_project_id: value.curseforge_project_id,
            curseforge_file_id: value.curseforge_file_id,
            curseforge_slug: value.curseforge_slug,
            modrinth_project_id: value.modrinth_project_id,
            modrinth_version_id: value.modrinth_version_id,
            modrinth_slug: value.modrinth_slug,
        }
    }
}

// Share preview for public preview endpoint (no auth required)
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct FESharePreview {
    share_code: String,
    title: Option<String>,
    minecraft_version: Option<String>,
    modloader_type: Option<String>,
    modloader_version: Option<String>,
    mods: Vec<FESharedMod>,
    size_kilobytes: i32,
    background_url: Option<String>,
    expires_at: DateTime<Utc>,
    download_count: i32,
    max_downloads: Option<i32>,
    sharer_display_name: String,
    sharer_friend_code: String,
}

impl From<SharePreview> for FESharePreview {
    fn from(value: SharePreview) -> Self {
        Self {
            share_code: value.share_code,
            title: value.title,
            minecraft_version: value.minecraft_version,
            modloader_type: value.modloader_type,
            modloader_version: value.modloader_version,
            mods: value.mods.into_iter().map(FESharedMod::from).collect(),
            size_kilobytes: value.size_kilobytes,
            background_url: value.background_url,
            expires_at: value.expires_at,
            download_count: value.download_count,
            max_downloads: value.max_downloads,
            sharer_display_name: value.sharer_display_name,
            sharer_friend_code: value.sharer_friend_code,
        }
    }
}

// Args for WAIT_FOR_SHARE_INSTANCE query
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEWaitForShareInstanceArgs {
    file_key: String,
    /// Optional instance_id to upload instance background after share completes
    #[specta(optional)]
    instance_id: Option<i32>,
}

// Args for GET_USER_SHARES query
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEGetUserSharesArgs {
    limit: Option<i32>,
    offset: Option<i32>,
}

// Individual share info
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEShareInfo {
    share_code: String,
    title: Option<String>,
    download_count: i32,
    max_downloads: Option<i32>,
    expires_at: DateTime<Utc>,
    size_kilobytes: i32,
    created_at: DateTime<Utc>,
    is_expired: bool,
}

impl From<ShareInfo> for FEShareInfo {
    fn from(value: ShareInfo) -> Self {
        Self {
            share_code: value.share_code,
            title: value.title,
            download_count: value.download_count,
            max_downloads: value.max_downloads,
            expires_at: value.expires_at,
            size_kilobytes: value.size_kilobytes,
            created_at: value.created_at,
            is_expired: value.is_expired,
        }
    }
}

// Paginated shares response
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEPaginatedShares {
    items: Vec<FEShareInfo>,
    total_count: i32,
    limit: i32,
    offset: i32,
}

impl From<PaginatedShares> for FEPaginatedShares {
    fn from(value: PaginatedShares) -> Self {
        Self {
            items: value.items.into_iter().map(FEShareInfo::from).collect(),
            total_count: value.total_count as i32,
            limit: value.limit as i32,
            offset: value.offset as i32,
        }
    }
}

// Quota info
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEQuotaInfo {
    used_kilobytes: i32,
    total_kilobytes: i32,
}

impl From<QuotaInfo> for FEQuotaInfo {
    fn from(value: QuotaInfo) -> Self {
        Self {
            used_kilobytes: value.used_kilobytes as i32,
            total_kilobytes: value.total_kilobytes as i32,
        }
    }
}

// Args for UPDATE_SHARE mutation
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEUpdateShareArgs {
    share_code: String,
    #[specta(optional)]
    title: Option<String>,
    #[specta(optional)]
    max_downloads: Option<Option<i32>>,
}

// Args for REPORT_SHARE mutation
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
struct FEReportShareArgs {
    share_code: String,
    /// One of: "share_background", "share_title", "share_content"
    report_type: String,
    #[specta(optional)]
    reason: Option<String>,
}

// Response for REGENERATE_SHARE_CODE mutation
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct FERegenerateShareCodeResponse {
    new_share_code: String,
}

impl From<RegenerateShareCodeResponse> for FERegenerateShareCodeResponse {
    fn from(value: RegenerateShareCodeResponse) -> Self {
        Self {
            new_share_code: value.new_share_code,
        }
    }
}
