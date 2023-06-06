use crate::api::{keys::vtask::*, translation::Translation};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    },
};

use anyhow::anyhow;

use thiserror::Error;
use tokio::sync::{watch, RwLock};

use super::ManagerRef;

use crate::domain::vtask as domain;
use domain::VisualTaskId;

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
            while notify.changed().await.is_ok() {
                if let NotifyState::Drop = *notify.borrow() {
                    break;
                }

                app.invalidate(GET_TASKS, None);
                app.invalidate(GET_TASK, Some(id.0.into()));
            }

            app.task_manager().tasks.write().await.remove(&id);
            app.invalidate(GET_TASKS, None);
            app.invalidate(GET_TASK, Some(id.0.into()));
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

    pub async fn dismiss_task(self, task_id: VisualTaskId) -> anyhow::Result<()> {
        let mut tasklist = self.tasks.write().await;
        let task = tasklist.get(&task_id).ok_or(InvalidTaskIdError)?;

        let data = task.data.read().await;
        if let TaskState::Failed(_) = &data.state {
            drop(data);
            tasklist.remove(&task_id);

            self.app.invalidate(GET_TASKS, None);
            self.app.invalidate(GET_TASK, Some(task_id.0.into()));

            Ok(())
        } else {
            Err(anyhow!(NonFailedDismissError))
        }
    }

    #[cfg(test)]
    pub async fn wait_with_log(self, task_id: VisualTaskId) -> anyhow::Result<()> {
        let tasklist = self.tasks.read().await;
        let Some(task) = tasklist.get(&task_id) else {
            println!("task already exited");
            return Ok(())
        };

        let mut notify = task.notify_rx.clone();

        while notify.changed().await.is_ok() {
            if let NotifyState::Drop = *notify.borrow() {
                break;
            }

            let domain = task.make_domain_task().await;

            let progress = match &domain.progress {
                domain::Progress::Indeterminate => String::from("unk"),
                domain::Progress::Known(p) => format!("{}%", p * 100.0),
                domain::Progress::Failed(_) => String::from("fail"),
            };

            println!(" -- Task Update ({progress}): {:?}", domain.name);

            for task in domain.active_subtasks {
                let progress = match task.progress {
                    domain::SubtaskProgress::Opaque => String::from("opaque"),
                    domain::SubtaskProgress::Download { downloaded, total } => format!(
                        "{}kb / {}kb",
                        downloaded as f32 * 0.001,
                        total as f32 * 0.001
                    ),
                    domain::SubtaskProgress::Item { current, total } => {
                        format!("{current} / {total}")
                    }
                };

                println!("Subtask ({progress}): {:?}", task.name);
            }

            if let domain::Progress::Failed(e) = &domain.progress {
                println!("Failure: {e:?}");
                break;
            }
        }

        Ok(())
    }
}

pub struct VisualTask {
    data: Arc<RwLock<TaskData>>,
    notify_rx: watch::Receiver<NotifyState>,
    notify_tx: Arc<watch::Sender<NotifyState>>,
    subtasks: Arc<RwLock<Vec<watch::Receiver<SubtaskData>>>>,
    owner: bool,
}

impl Clone for VisualTask {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            notify_tx: self.notify_tx.clone(),
            notify_rx: self.notify_rx.clone(),
            subtasks: self.subtasks.clone(),
            owner: false,
        }
    }
}

enum NotifyState {
    Update,
    Drop,
}

impl Drop for VisualTask {
    fn drop(&mut self) {
        if self.owner {
            let _ = self.notify_tx.send(NotifyState::Drop);
        }
    }
}

impl VisualTask {
    pub fn new(name: Translation) -> Self {
        let (notify_tx, notify_rx) = watch::channel(NotifyState::Update);

        Self {
            data: Arc::new(RwLock::new(TaskData {
                name,
                state: TaskState::Indeterminate,
            })),
            notify_rx,
            notify_tx: Arc::new(notify_tx),
            subtasks: Arc::new(RwLock::new(Vec::new())),
            owner: true,
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

    pub async fn fail(mut self, error: anyhow::Error) {
        self.edit(|data| data.state = TaskState::Failed(Arc::new(error)))
            .await;

        // disown and drop self, leaving it in the task list
        self.owner = false;

        let _ = self.notify_tx.send(NotifyState::Update);
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
                Progress::Download {
                    downloaded, total, ..
                } => (downloaded, total),
                _ => (0, 0),
            })
            .fold((0, 0), |(ad, at), (d, t)| (ad + d, at + t))
    }

    pub async fn make_domain_task(&self) -> domain::Task {
        let (name, state) = {
            let data = self.data.read().await;
            (data.name.clone(), data.state.clone())
        };

        let (downloaded, download_total) = self.downloaded_bytes().await;

        domain::Task {
            name: name.into(),
            progress: match state {
                TaskState::Indeterminate => domain::Progress::Indeterminate,
                TaskState::KnownProgress => domain::Progress::Known(self.progress_float().await),
                TaskState::Failed(error) => domain::Progress::Failed(error),
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

    pub fn update_progress(&self, progress: Progress) {
        self.update(|data| {
            data.started = true;
            data.progress = progress;
        });
    }

    pub fn update_download(&self, downloaded: u32, total: u32) {
        self.update_progress(Progress::Download {
            downloaded,
            total,
            complete: false,
        });
    }

    pub fn update_items(&self, current: u32, total: u32) {
        self.update_progress(Progress::Item { current, total });
    }

    pub fn start_opaque(&self) {
        self.update_progress(Progress::Opaque(false));
    }

    pub fn complete_opaque(&self) {
        self.update_progress(Progress::Opaque(true));
    }

    pub fn complete_download(&self) {
        self.update(|data| {
            data.progress = match data.progress {
                Progress::Download {
                    downloaded, total, ..
                } => Progress::Download {
                    downloaded,
                    total,
                    complete: true,
                },
                _ => Progress::Opaque(true),
            }
        });
    }

    pub fn set_weight(&self, weight: f32) {
        self.update(|data| data.weight = weight);
    }
}

pub struct TaskData {
    pub name: Translation,
    pub state: TaskState,
}

#[derive(Clone)]
pub enum TaskState {
    Indeterminate,
    KnownProgress,
    Failed(Arc<anyhow::Error>),
}

impl TaskState {
    fn from_indeterminate(indeterminate: bool) -> Self {
        match indeterminate {
            true => Self::Indeterminate,
            false => Self::KnownProgress,
        }
    }
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
    Download {
        downloaded: u32,
        total: u32,
        complete: bool,
    },

    // Item progress numbers are shown after the subtask text as `(<current>/<total>)`.
    Item {
        current: u32,
        total: u32,
    },

    // There isn't a reasonable way to represent the progress of this task, so progress is
    // represented as an opaque "is it done" boolean. `Opaque(false)` can also represent
    // `Download` or `Item` states that aren't calculated yet.
    Opaque(bool),
}

impl Progress {
    pub fn as_float(self) -> f32 {
        match self {
            Self::Download {
                downloaded, total, ..
            } => downloaded as f32 / total as f32,
            Self::Item { current, total } => current as f32 / total as f32,
            Self::Opaque(false) => 0.0,
            Self::Opaque(true) => 1.0,
        }
    }

    pub fn is_complete(self) -> bool {
        match self {
            Self::Download { complete, .. } => complete,
            Self::Item { current, total } => current >= total,
            Self::Opaque(complete) => complete,
        }
    }
}

impl From<Progress> for domain::SubtaskProgress {
    fn from(value: Progress) -> Self {
        match value {
            Progress::Download {
                downloaded, total, ..
            } => Self::Download { downloaded, total },
            Progress::Item { current, total } => Self::Item { current, total },
            Progress::Opaque(_) => Self::Opaque,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::api::translation::Translation;
    use crate::domain::vtask as domain;
    use crate::managers::vtask::{TaskState, VisualTask};

    #[tokio::test]
    async fn test() {
        let app = crate::setup_managers_for_test().await;

        let task = VisualTask::new(Translation::Test);
        let id = app.task_manager().spawn_task(&task).await;

        let subtask = task.subtask(Translation::Test).await;

        subtask.start_opaque();

        let mut tasks = vec![domain::Task {
            name: Translation::Test,
            progress: domain::Progress::Indeterminate,
            downloaded: 0,
            download_total: 0,
            active_subtasks: vec![domain::Subtask {
                name: Translation::Test,
                progress: domain::SubtaskProgress::Opaque,
            }],
        }];

        assert_eq!(tasks, app.task_manager().get_tasks().await);
        assert_eq!(
            Some(&tasks[0]),
            app.task_manager().get_task(id).await.as_ref()
        );

        task.edit(|data| data.state = TaskState::KnownProgress)
            .await;
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

#[derive(Error, Debug)]
#[error("task id does not refer to a valid task")]
pub struct InvalidTaskIdError;

#[derive(Error, Debug)]
#[error("tasks that are not in a failed state cannot be dismissed")]
pub struct NonFailedDismissError;
