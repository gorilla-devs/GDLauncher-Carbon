use rspc::{RouterBuilderLike, Type};
use serde::Serialize;

use crate::api::keys::queue::*;
use crate::managers::App;
use carbon_domain::vtask as domain;

use super::router::router;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_TASKS[app, _: ()] {
            Ok(app.task_manager().get_tasks().await
               .into_iter()
               .map(|task| Task::from(task))
               .collect::<Vec<_>>())
        }
    }
}

#[derive(Type, Serialize)]
pub struct Task {
    name: String,
    progress: Progress,
    downloaded: u32,
    download_total: u32,
    active_subtasks: Vec<Subtask>,
}

#[derive(Type, Serialize)]
pub enum Progress {
    Indeterminate,
    Known(f32),
}

#[derive(Type, Serialize)]
pub struct Subtask {
    name: String,
    progress: SubtaskProgress,
}

#[derive(Type, Serialize)]
pub enum SubtaskProgress {
    Download { downloaded: u32, total: u32 },
    Item { current: u32, total: u32 },
    Opaque,
}

impl From<domain::Task> for Task {
    fn from(value: domain::Task) -> Self {
        Self {
            name: value.name,
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
        }
    }
}

impl From<domain::Subtask> for Subtask {
    fn from(value: domain::Subtask) -> Self {
        Self {
            name: value.name,
            progress: value.progress.into(),
        }
    }
}

impl From<domain::SubtaskProgress> for SubtaskProgress {
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
