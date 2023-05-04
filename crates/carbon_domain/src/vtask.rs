use std::sync::Arc;

use crate::translation::Translation;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct VisualTaskId(pub i32);

#[derive(Debug, PartialEq)]
pub struct Task {
    pub name: Translation,
    pub progress: Progress,
    pub downloaded: u32,
    pub download_total: u32,
    pub active_subtasks: Vec<Subtask>,
}

#[derive(Debug)]
pub enum Progress {
    Indeterminate,
    Known(f32),
    Failed(Arc<anyhow::Error>),
}

#[derive(Debug, PartialEq)]
pub struct Subtask {
    pub name: Translation,
    pub progress: SubtaskProgress,
}

#[derive(Debug, PartialEq)]
pub enum SubtaskProgress {
    Download { downloaded: u32, total: u32 },
    Item { current: u32, total: u32 },
    Opaque,
}

impl PartialEq for Progress {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Progress::Indeterminate, Progress::Indeterminate) => true,
            (Progress::Known(a), Progress::Known(b)) if a == b => true,
            (Progress::Failed(_), Progress::Failed(_)) => true,
            _ => false,
        }
    }
}
