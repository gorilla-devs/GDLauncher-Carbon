use crate::{api::keys::vtask::*, translation::Translation};
use std::{
    collections::HashMap,
    sync::{
        atomic::{Ordering, AtomicI32},
        Arc,
    },
};

use tokio::sync::{watch, RwLock};

use super::ManagerRef;

use carbon_domain::vtask as domain;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct VisualTaskId(pub i32);

pub struct VisualTaskManager {
    tasks: RwLock<HashMap<VisualTaskId, VisualTask>>,
}

impl VisualTaskManager {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
        }
    }
}

impl ManagerRef<'_, VisualTaskManager> {
    pub async fn spawn_task(self, task: &VisualTask) -> VisualTaskId {
        let task = task.clone();
        static ATOMIC_ID: AtomicI32 = AtomicI32::new(0);

        // Note: the id also keeps tasks in order.
        let id = VisualTaskId(ATOMIC_ID.fetch_add(1, Ordering::Relaxed));

        let mut notify = task.notify_rx.clone();

        self.tasks.write().await.insert(id, task);
        self.app.invalidate(GET_TASKS, None);

        let app = self.app.clone();
        tokio::task::spawn(async move {
            // Invalidate when changed until dropped.
            // On drop remove the task from the list.
            while let Ok(()) = notify.changed().await {
                if let NotifyState::Drop = *notify.borrow() {
                    break;
                }
                app.invalidate(GET_TASKS, None);
            }

            app.task_manager().tasks.write().await.remove(&id);
            app.invalidate(GET_TASKS, None);
        });

        id
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

    pub async fn get_task(self, task_id: VisualTaskId) -> Option<domain::Task> {
        let tasklist = self.tasks.read().await;
        let task = tasklist.get(&task_id);

        match task {
            Some(task) => Some(task.make_domain_task().await),
            None => None,
        }
    }
}

#[derive(Clone)]
pub struct VisualTask {
    data: Arc<RwLock<TaskData>>,
    notify_rx: watch::Receiver<NotifyState>,
    notify_tx: Arc<watch::Sender<NotifyState>>,
    subtasks: Arc<RwLock<Vec<watch::Receiver<SubtaskData>>>>,
}

enum NotifyState {
    Update,
    Drop,
}

impl Drop for VisualTask {
    fn drop(&mut self) {
        let _ = self.notify_tx.send(NotifyState::Drop);
    }
}

impl VisualTask {
    pub fn new(name: Translation) -> Self {
        let (notify_tx, notify_rx) = watch::channel(NotifyState::Update);

        Self {
            data: Arc::new(RwLock::new(TaskData {
                name,
                indeterminate: true,
            })),
            notify_rx,
            notify_tx: Arc::new(notify_tx),
            subtasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn subtask(&self, name: Translation) -> Subtask {
        let (watch_tx, watch_rx) = watch::channel(SubtaskData {
            name,
            weight: 1.0,
            started: false,
            progress: Progress::Opaque(false),
        });

        self.subtasks.write().await.push(watch_rx);

        Subtask {
            notify: self.notify_tx.clone(),
            data: watch_tx,
        }
    }

    pub async fn edit(&self, f: impl FnOnce(&mut TaskData)) {
        f(&mut *self.data.write().await);
    }

    // Get the current task progress as a float from 0.0 to 1.0
    pub async fn progress_float(&self) -> f32 {
        let subtasks = self.subtasks.read().await;
        let total_weight = subtasks
            .iter()
            .map(|task| task.borrow().weight)
            .sum::<f32>();

        subtasks
            .iter()
            .map(|task| {
                let task = task.borrow();
                let mul = task.weight / total_weight;
                task.progress.as_float() * mul
            })
            .sum()
    }

    pub async fn downloaded_bytes(&self) -> (u32, u32) {
        self.subtasks
            .read()
            .await
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

        let (downloaded, download_total) = self.downloaded_bytes().await;

        domain::Task {
            name: name.into(),
            progress: match indeterminate {
                true => domain::Progress::Indeterminate,
                false => domain::Progress::Known(self.progress_float().await),
            },
            downloaded,
            download_total,
            active_subtasks: self
                .subtasks
                .read()
                .await
                .iter()
                .map(|t| t.borrow())
                .filter(|t| t.started)
                .filter(|t| !t.progress.is_complete())
                .map(|t| domain::Subtask {
                    name: t.name.clone().into(),
                    progress: t.progress.into(),
                })
                .collect(),
        }
    }
}

pub struct Subtask {
    notify: Arc<watch::Sender<NotifyState>>,
    data: watch::Sender<SubtaskData>,
}

impl Subtask {
    pub fn update(&self, f: impl FnOnce(&mut SubtaskData)) {
        self.data.send_modify(f);
        let _ = self.notify.send(NotifyState::Update);
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
    pub name: Translation,
    /// the indeterminate flag hides the progress bar before tasks have decided
    /// their respective weights.
    pub indeterminate: bool,
}

pub struct SubtaskData {
    /// The subtask's name. Shows as subtext under the main task name.
    pub name: Translation,
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

#[cfg(test)]
mod test {
    use crate::{managers::vtask::VisualTask, translation::translate};
    use carbon_domain::vtask as domain;

    #[tokio::test]
    async fn test() {
        let app = crate::setup_managers_for_test().await;

        let task = VisualTask::new(translate!("test"));
        app.task_manager().spawn_task(&task).await;

        let subtask = task.subtask(translate!("subtask")).await;

        subtask.start_task();

        let mut tasks = vec![domain::Task {
            name: translate!("test"),
            progress: domain::Progress::Indeterminate,
            downloaded: 0,
            download_total: 0,
            active_subtasks: vec![domain::Subtask {
                name: translate!("subtask"),
                progress: domain::SubtaskProgress::Opaque,
            }],
        }];

        assert_eq!(tasks, app.task_manager().get_tasks().await);

        task.edit(|data| data.indeterminate = false).await;
        tasks[0].progress = domain::Progress::Known(0.0);
        assert_eq!(tasks, app.task_manager().get_tasks().await);

        subtask.update_items(1, 2);
        tasks[0].progress = domain::Progress::Known(0.5);
        tasks[0].active_subtasks[0].progress = domain::SubtaskProgress::Item {
            current: 1,
            total: 2,
        };
        assert_eq!(tasks, app.task_manager().get_tasks().await);

        drop(task);
        tasks.clear();
        // give the queue time to poll
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(tasks, app.task_manager().get_tasks().await);
    }
}
