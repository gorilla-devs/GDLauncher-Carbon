use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::api::keys::vtask::*;
use crate::error::FeError;

use crate::domain::vtask as domain;
use crate::managers::App;

use super::router::router;
use super::translation::Translation;

pub(super) fn mount() -> RouterBuilder<App> {
    router! {
        query GET_TASKS[app, args: ()] {
            Ok(app.task_manager().get_tasks().await
               .into_iter()
               .map(FETask::from)
               .collect::<Vec<_>>())
        }

        query GET_TASK[app, task: Option<FETaskId>] {
            let Some(task) = task else {
                return Ok(None);
            };

            let result = app.task_manager()
               .get_task(task.into())
               .await
               .map(FETask::from);

            Ok(result)
        }

        mutation DISMISS_TASK[app, task: FETaskId] {
            app.task_manager().dismiss_task(task.into()).await
        }

        query GET_TASK_OUTCOME[app, task: Option<FETaskId>] {
            Ok(match task {
                Some(task) => FETaskOutcome::from(app.task_manager().get_task_outcome(task.into()).await),
                None => FETaskOutcome::Unavailable,
            })
        }

        // Warnings of succeeded tasks the UI has not shown yet.
        query GET_TASK_WARNINGS[app, args: ()] {
            Ok(app.task_manager().get_task_warnings().await
               .into_iter()
               .map(FETaskWarnings::from)
               .collect::<Vec<_>>())
        }

        mutation ACKNOWLEDGE_TASK_WARNINGS[app, task: FETaskId] {
            app.task_manager().acknowledge_task_warnings(task.into()).await;
            Ok(())
        }
    }
}

#[derive(Type, Serialize)]
pub struct FETaskWarnings {
    id: FETaskId,
    name: Translation,
    /// What the warnings are about (an addon's project title), when the task
    /// name does not say it plainly.
    subject: Option<String>,
    warnings: Vec<FETaskWarning>,
}

#[derive(Type, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum FETaskWarning {
    SkippedDependency {
        /// Unknown when the platform no longer serves the project.
        name: Option<String>,
        platform: FEDependencyPlatform,
        /// A project id, or the file name of a dependency declared by file only.
        project_id: String,
        reason: FESkippedDependencyReason,
    },
    /// Installed at another release than the addon pins, because the pinned
    /// one is not for this Minecraft version and loader.
    ReplacedPinnedVersion {
        name: String,
        project_id: String,
        pinned: String,
        installed: String,
    },
    /// Two addons in one installation ask for different versions of a
    /// dependency; `installed` is the one that was installed.
    ConflictingVersions {
        name: String,
        platform: FEDependencyPlatform,
        project_id: String,
        installed: String,
    },
    /// A modpack's files under a link in the server folder were not
    /// installed; `path` is the link, `/`-separated, inside the server folder.
    PackFilesSkippedAtLink { path: String },
    /// A world was installed as the new save `saved_as`, because a save
    /// named `name` already exists; that save was not changed.
    WorldNameTaken { name: String, saved_as: String },
}

#[derive(Type, Serialize)]
pub enum FEDependencyPlatform {
    Curseforge,
    Modrinth,
}

#[derive(Type, Serialize)]
pub enum FESkippedDependencyReason {
    NoCompatibleVersion,
    NotDownloadable,
    Unavailable,
    NotForServers,
    RemovedDuringInstall,
    /// Installed but disabled: enabling it is up to the user.
    Disabled,
}

#[derive(Type, Serialize)]
pub enum FEPreviousPackFilesReason {
}

impl From<domain::DependencyPlatform> for FEDependencyPlatform {
    fn from(value: domain::DependencyPlatform) -> Self {
        match value {
            domain::DependencyPlatform::Curseforge => Self::Curseforge,
            domain::DependencyPlatform::Modrinth => Self::Modrinth,
        }
    }
}

impl From<domain::TaskWarnings> for FETaskWarnings {
    fn from(value: domain::TaskWarnings) -> Self {
        Self {
            id: value.id.into(),
            name: value.name,
            subject: value.subject,
            warnings: value.warnings.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<domain::TaskWarning> for FETaskWarning {
    fn from(value: domain::TaskWarning) -> Self {
        match value {
            domain::TaskWarning::SkippedDependency {
                name,
                platform,
                project_id,
                reason,
            } => Self::SkippedDependency {
                name,
                platform: platform.into(),
                project_id,
                reason: match reason {
                    domain::SkippedDependencyReason::NoCompatibleVersion => {
                        FESkippedDependencyReason::NoCompatibleVersion
                    }
                    domain::SkippedDependencyReason::NotDownloadable => {
                        FESkippedDependencyReason::NotDownloadable
                    }
                    domain::SkippedDependencyReason::Unavailable => {
                        FESkippedDependencyReason::Unavailable
                    }
                    domain::SkippedDependencyReason::NotForServers => {
                        FESkippedDependencyReason::NotForServers
                    }
                    domain::SkippedDependencyReason::RemovedDuringInstall => {
                        FESkippedDependencyReason::RemovedDuringInstall
                    }
                    domain::SkippedDependencyReason::Disabled => {
                        FESkippedDependencyReason::Disabled
                    }
                },
            },
            domain::TaskWarning::ReplacedPinnedVersion {
                name,
                project_id,
                pinned,
                installed,
            } => Self::ReplacedPinnedVersion {
                name,
                project_id,
                pinned,
                installed,
            },
            domain::TaskWarning::ConflictingVersions {
                name,
                platform,
                project_id,
                installed,
            } => Self::ConflictingVersions {
                name,
                platform: platform.into(),
                project_id,
                installed,
            },
            domain::TaskWarning::PackFilesSkippedAtLink { path } => {
                Self::PackFilesSkippedAtLink { path }
            }
            domain::TaskWarning::WorldNameTaken { name, saved_as } => {
                Self::WorldNameTaken { name, saved_as }
            }
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct FETaskId(pub i32);

#[derive(Type, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum FETaskOutcome {
    Running(FETask),
    Failed(FETask),
    Succeeded,
    Dismissed,
    Unavailable,
}

impl From<crate::managers::vtask::TaskOutcome> for FETaskOutcome {
    fn from(value: crate::managers::vtask::TaskOutcome) -> Self {
        use crate::managers::vtask::TaskOutcome;
        match value {
            TaskOutcome::Running(task) => Self::Running(task.into()),
            TaskOutcome::Failed(task) => Self::Failed(task.into()),
            TaskOutcome::Succeeded => Self::Succeeded,
            TaskOutcome::Dismissed => Self::Dismissed,
            TaskOutcome::Unavailable => Self::Unavailable,
        }
    }
}

impl From<domain::VisualTaskId> for FETaskId {
    fn from(value: domain::VisualTaskId) -> Self {
        Self(value.0)
    }
}

impl From<FETaskId> for domain::VisualTaskId {
    fn from(value: FETaskId) -> Self {
        Self(value.0)
    }
}

#[derive(Type, Serialize)]
pub struct FETask {
    id: FETaskId,
    name: Translation,
    progress: Progress,
    downloaded: u32,
    download_total: u32,
    active_subtasks: Vec<FESubtask>,
}

#[derive(Type, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Progress {
    Indeterminate,
    Known(f32),
    Failed(FeError),
}

#[derive(Type, Serialize)]
pub struct FESubtask {
    name: Translation,
    progress: FESubtaskProgress,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FESubtaskProgress {
    Download { downloaded: u32, total: u32 },
    Item { current: u32, total: u32 },
    Opaque,
}

impl From<domain::Task> for FETask {
    fn from(value: domain::Task) -> Self {
        Self {
            id: value.id.into(),
            name: value.name.into(),
            progress: value.progress.into(),
            downloaded: value.downloaded,
            download_total: value.download_total,
            active_subtasks: value
                .active_subtasks
                .into_iter()
                .map(|task| task.into())
                .collect(),
        }
    }
}

impl From<domain::Progress> for Progress {
    fn from(value: domain::Progress) -> Self {
        match value {
            domain::Progress::Indeterminate => Self::Indeterminate,
            domain::Progress::Known(x) => Self::Known(x),
            domain::Progress::Failed(err) => Self::Failed(FeError::from_anyhow(&*err)),
        }
    }
}

impl From<domain::Subtask> for FESubtask {
    fn from(value: domain::Subtask) -> Self {
        Self {
            name: value.name.into(),
            progress: value.progress.into(),
        }
    }
}

impl From<domain::SubtaskProgress> for FESubtaskProgress {
    fn from(value: domain::SubtaskProgress) -> Self {
        match value {
            domain::SubtaskProgress::Download { downloaded, total } => {
                Self::Download { downloaded, total }
            }
            domain::SubtaskProgress::Item { current, total } => Self::Item { current, total },
            domain::SubtaskProgress::Opaque => Self::Opaque,
        }
    }
}
