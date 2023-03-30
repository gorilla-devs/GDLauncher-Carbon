use crate::api::keys::queue::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::sync::{watch, RwLock};

use super::ManagerRef;

use carbon_domain::vqueue as domain;

pub struct VisualTaskManager {
    tasks: RwLock<HashMap<usize, VisualTask>>,
}

impl VisualTaskManager {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
        }
    }
}

impl ManagerRef<'_, VisualTaskManager> {
    pub async fn spawn_task(self, task: VisualTask) {
        static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);

        // Note: the id also keeps tasks in order.
        let id = ATOMIC_ID.fetch_add(1, Ordering::Relaxed);

        let mut notify = task.notify_rx.clone();

        self.tasks.write().await.insert(id, task);
        self.app.invalidate(GET_TASKS, None);

        let app = self.app.clone();
        tokio::task::spawn(async move {
            // Invalidate when changed until dropped.
            // On drop remove the task from the list.
            while let Ok(_) = notify.changed().await {
                app.invalidate(GET_TASKS, None);
            }

            app.task_manager().tasks.write().await.remove(&id);
            app.invalidate(GET_TASKS, None);
        });
    }

    pub async fn get_tasks(self) -> Vec<domain::Task> {
        let tasklist = self.tasks.read().await;
        let mut tasks = tasklist
            .iter()
            .map(|(i, task)| (i, task.make_domain_task()))
            .collect::<Vec<_>>();
        tasks.sort_by(|(a, _), (b, _)| Ord::cmp(a, b));

        let mut ret = Vec::<domain::Task>::with_capacity(tasks.len());

        for (_, task) in tasks {
            ret.push(task.await);
        }

        ret
    }
}

pub struct VisualTask {
    data: RwLock<TaskData>,
    notify_rx: watch::Receiver<()>,
    notify_tx: Arc<watch::Sender<()>>,
    subtasks: Vec<watch::Receiver<SubtaskData>>,
}

impl VisualTask {
    pub fn new(name: String) -> Self {
        let (notify_tx, notify_rx) = watch::channel(());

        Self {
            data: RwLock::new(TaskData {
                name,
                indeterminate: true,
            }),
            notify_rx,
            notify_tx: Arc::new(notify_tx),
            subtasks: Vec::new(),
        }
    }

    pub fn subtask(&mut self, name: String) -> Subtask {
        let (watch_tx, watch_rx) = watch::channel(SubtaskData {
            name,
            weight: 1.0,
            started: false,
            progress: Progress::Opaque(false),
        });

        self.subtasks.push(watch_rx);

        Subtask {
            notify: self.notify_tx.clone(),
            data: watch_tx,
        }
    }

    pub async fn edit(&self, f: impl FnOnce(&mut TaskData)) {
        f(&mut *self.data.write().await);
    }

    // Get the current task progress as a float from 0.0 to 1.0
    pub fn progress_float(&self) -> f32 {
        let total_weight = self
            .subtasks
            .iter()
            .map(|task| task.borrow().weight)
            .sum::<f32>();

        self.subtasks
            .iter()
            .map(|task| {
                let task = task.borrow();
                let mul = task.weight / total_weight;
                task.progress.as_float() * mul
            })
            .sum()
    }

    pub fn downloaded_bytes(&self) -> (u32, u32) {
        self.subtasks
            .iter()
            .map(|task| match task.borrow().progress {
                Progress::Download { downloaded, total } => (downloaded, total),
                _ => (0, 0),
            })
            .fold((0, 0), |(ad, at), (d, t)| (ad + d, at + t))
    }

    pub async fn make_domain_task(&self) -> domain::Task {
        let (name, indeterminate) = {
            let data = self.data.read().await;
            (data.name.clone(), data.indeterminate)
        };

        let (downloaded, download_total) = self.downloaded_bytes();

        domain::Task {
            name,
            progress: match indeterminate {
                true => domain::Progress::Indeterminate,
                false => domain::Progress::Known(self.progress_float()),
            },
            downloaded,
            download_total,
            active_subtasks: self
                .subtasks
                .iter()
                .map(|t| t.borrow())
                .filter(|t| t.started)
                .filter(|t| !t.progress.is_complete())
                .map(|t| domain::Subtask {
                    name: t.name.clone(),
                    progress: t.progress.into(),
                })
                .collect(),
        }
    }
}

pub struct Subtask {
    notify: Arc<watch::Sender<()>>,
    data: watch::Sender<SubtaskData>,
}

impl Subtask {
    pub fn update(&self, f: impl FnOnce(&mut SubtaskData)) {
        self.data.send_modify(f);
        let _ = self.notify.send(());
    }

    // convenience functions

    pub fn start_task(&self) {
        self.update(|data| data.started = true);
    }

    pub fn update_progress(&self, progress: Progress) {
        self.update(|data| data.progress = progress);
    }

    pub fn update_download(&self, downloaded: u32, total: u32) {
        self.update_progress(Progress::Download { downloaded, total });
    }

    pub fn update_items(&self, current: u32, total: u32) {
        self.update_progress(Progress::Item { current, total });
    }

    pub fn complete_opaque(&self) {
        self.update_progress(Progress::Opaque(true));
    }

    pub fn set_weight(&self, weight: f32) {
        self.update(|data| data.weight = weight);
    }
}

pub struct TaskData {
    pub name: String,
    /// the indeterminate flag hides the progress bar before tasks have decided
    /// their respective weights.
    pub indeterminate: bool,
}

pub struct SubtaskData {
    /// The subtask's name. Shows as subtext under the main task name.
    pub name: String,
    /// Relative amount of space on the task progress bar this subtask takes.
    pub weight: f32,
    /// Started tasks show in the task list if they are not also complete.
    pub started: bool,
    pub progress: Progress,
}

#[derive(Copy, Clone)]
pub enum Progress {
    // Download progress numbers are added to the overall task downloaded number,
    // shown after the subtask text as `(<downloaded>/<total>)` in mb.
    Download { downloaded: u32, total: u32 },

    // Item progress numbers are shown after the subtask text as `(<current>/<total>)`.
    Item { current: u32, total: u32 },

    // There isn't a reasonable way to represent the progress of this task, so progress is
    // represented as an opaque "is it done" boolean. `Opaque(false)` can also represent
    // `Download` or `Item` states that aren't calculated yet.
    Opaque(bool),
}

impl Progress {
    pub fn as_float(self) -> f32 {
        match self {
            Self::Download { downloaded, total } => downloaded as f32 / total as f32,
            Self::Item { current, total } => current as f32 / total as f32,
            Self::Opaque(false) => 0.0,
            Self::Opaque(true) => 1.0,
        }
    }

    pub fn is_complete(self) -> bool {
        match self {
            Self::Download { downloaded, total } => downloaded >= total,
            Self::Item { current, total } => current >= total,
            Self::Opaque(complete) => complete,
        }
    }
}

impl From<Progress> for domain::SubtaskProgress {
    fn from(value: Progress) -> Self {
        match value {
            Progress::Download { downloaded, total } => Self::Download { downloaded, total },
            Progress::Item { current, total } => Self::Item { current, total },
            Progress::Opaque(_) => Self::Opaque,
        }
    }
}
