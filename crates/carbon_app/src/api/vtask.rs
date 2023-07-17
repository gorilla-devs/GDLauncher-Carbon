use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

use crate::api::keys::vtask::*;
use crate::error::FeError;

use crate::domain::vtask as domain;
use crate::managers::App;

use super::router::router;
use super::translation::Translation;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_TASKS[app, args: ()] {
            Ok(app.task_manager().get_tasks().await
               .into_iter()
               .map(FETask::from)
               .collect::<Vec<_>>())
        }

        query GET_TASK[app, task: FETaskId] {
            Ok(app.task_manager()
               .get_task(task.into())
               .await
               .map(FETask::from))
        }

        mutation DISMISS_TASK[app, task: FETaskId] {
            app.task_manager().dismiss_task(task.into()).await
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct FETaskId(pub i32);

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
    name: Translation,
    progress: Progress,
    downloaded: u32,
    download_total: u32,
    active_subtasks: Vec<FESubtask>,
}

#[derive(Type, Serialize)]
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
