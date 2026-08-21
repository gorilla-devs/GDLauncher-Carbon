use crate::api::{keys::vtask::*, translation::Translation};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicI32, Ordering},
    },
};

use anyhow::anyhow;

use thiserror::Error;
use tokio::sync::{RwLock, watch};
use tracing::error;

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
        static ATOMIC_ID: AtomicI32 = AtomicI32::new(1);

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

                // Cap invalidations at ~5/s per task. `notify` is a watch
                // channel, so updates arriving during this pause collapse onto
                // one slot: the first update still invalidates immediately, and
                // whatever the progress settled on is always delivered, because
                // the next `changed()` returns straight away for anything that
                // landed while sleeping. Invalidations carry no payload — the
                // frontend re-reads current state — so dropping the ticks in
                // between loses nothing. Without this, a fast scan emits one
                // pair of invalidations per file processed.
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
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
            .map(|(i, task)| (i, task.make_domain_task(*i)))
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
            Some(task) => Some(task.make_domain_task(task_id).await),
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
        use tracing::info;

        let mut notify = {
            let tasklist = self.tasks.read().await;
            let Some(task) = tasklist.get(&task_id) else {
                info!("task already exited");
                return Ok(());
            };

            let notify = task.notify_rx.clone();
            notify
        }; // tasklist is dropped here, releasing the strong reference to the task

        while notify.changed().await.is_ok() {
            if let NotifyState::Drop = *notify.borrow() {
                info!("Received Drop notification, exiting wait_with_log");
                break;
            }

            // For logging, we need to get the task again, but only temporarily
            let domain = {
                let tasklist = self.tasks.read().await;
                if let Some(task) = tasklist.get(&task_id) {
                    task.make_domain_task(task_id).await
                } else {
                    // Task was removed from the list, exit
                    break;
                }
            };

            let progress = match &domain.progress {
                domain::Progress::Indeterminate => String::from("unk"),
                domain::Progress::Known(p) => format!("{}%", p * 100.0),
                domain::Progress::Failed(_) => String::from("fail"),
            };

            info!(" -- Task Update ({progress}): {:?}", domain.name);

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

                info!("Subtask ({progress}): {:?}", task.name);
            }

            if let domain::Progress::Failed(e) = &domain.progress {
                error!("Failure: {e:?}");
                break;
            }
        }

        info!("task exited");

        Ok(())
    }
}

pub struct VisualTask {
    data: Arc<RwLock<TaskData>>,
    notify_rx: watch::Receiver<NotifyState>,
    notify_tx: Arc<watch::Sender<NotifyState>>,
    // TODO: this should probably be replaced with a channel of some kind, instead of a sync mutex
    subtasks: Arc<Mutex<Vec<watch::Receiver<SubtaskData>>>>,
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
            tracing::info!("VisualTask owner dropped, sending Drop notification");
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
                checked_progress: 0.0,
            })),
            notify_rx,
            notify_tx: Arc::new(notify_tx),
            subtasks: Arc::new(Mutex::new(Vec::new())),
            owner: true,
        }
    }

    pub fn subtask(&self, name: Translation) -> Subtask {
        let (watch_tx, watch_rx) = watch::channel(SubtaskData {
            name,
            weight: 1.0,
            started: false,
            progress: Progress::Opaque(false),
            checked_progress: Mutex::new(0.0),
        });

        self.subtasks
            .lock()
            .expect("this mutex can never witness a panic")
            .push(watch_rx);

        Subtask {
            notify: self.notify_tx.clone(),
            data: watch_tx,
        }
    }

    pub async fn edit(&self, f: impl FnOnce(&mut TaskData)) {
        f(&mut *self.data.write().await);
    }

    pub async fn fail(mut self, error: anyhow::Error) {
        error!({ error = ?error }, "task failed: {name:?}", name = self.data.read().await.name);

        self.edit(|data| data.state = TaskState::Failed(Arc::new(error)))
            .await;

        // disown and drop self, leaving it in the task list
        self.owner = false;

        let _ = self.notify_tx.send(NotifyState::Update);
    }

    /// Get the current task progress as a float from 0.0 to 1.0.
    /// Finalizes checked progress in the main progress bar.
    pub async fn progress_float(&self) -> f32 {
        // The math here is wrong, disabled due to time constraints.
        /*let additional_progress = {
            let subtasks = self.subtasks.lock().expect("this mutex can never witness a panic");

            let remaining_weight = subtasks
                .iter()
                .map(|subtask| {
                    let subtask = subtask.borrow();
                    // as this is the only function that interacts with this value, it should never have to pause
                    let progress = subtask.checked_progress.lock().expect("this mutex can never witness a panic");

                    subtask.weight * (1.0 - *progress)
                })
                .sum::<f32>();

            let additional_progress = subtasks
                .iter()
                .map(|subtask| {
                    let subtask = subtask.borrow();
                    let mut checked_progress = subtask.checked_progress.lock().expect("this mutex can never witness a panic");
                    let progress = subtask.progress.as_float();
                    let new_progress = progress - *checked_progress;
                    let weight = subtask.weight * (1.0 - *checked_progress);

                    *checked_progress = progress;
                    new_progress * (subtask.weight * remaining_weight)
                })
                .sum::<f32>();

            additional_progress
        };

        let mut data = self.data.write().await;
        data.checked_progress += additional_progress;
        data.checked_progress*/

        let subtasks = self
            .subtasks
            .lock()
            .expect("this mutex can never witness a panic");
        let total_weight = subtasks
            .iter()
            .map(|subtask| subtask.borrow().weight)
            .sum::<f32>();

        subtasks
            .iter()
            .map(|subtask| {
                let subtask = subtask.borrow();
                let mul = subtask.weight / total_weight;
                subtask.progress.as_float() * mul
            })
            .sum()
    }

    pub async fn downloaded_bytes(&self) -> (u32, u32) {
        self.subtasks
            .lock()
            .expect("this mutex can never witness a panic")
            .iter()
            .map(|task| match task.borrow().progress {
                Progress::Download {
                    downloaded, total, ..
                } => (downloaded, total),
                _ => (0, 0),
            })
            .fold((0, 0), |(ad, at), (d, t)| (ad + d, at + t))
    }

    pub async fn make_domain_task(&self, id: VisualTaskId) -> domain::Task {
        let (name, state) = {
            let data = self.data.read().await;
            (data.name.clone(), data.state.clone())
        };

        let (downloaded, download_total) = self.downloaded_bytes().await;

        domain::Task {
            id,
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
                .lock()
                .expect("this mutex can never witness a panic")
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
            if data.progress != progress {
                data.progress = progress;
            }
        });
    }

    // complete_on_match is an explicit parameter instead of a following call to make sure
    // a conscious decision is made on a case by case basis.
    pub fn update_download(&self, downloaded: u32, total: u32, complete_on_match: bool) {
        self.update_progress(Progress::Download {
            downloaded,
            total,
            complete: complete_on_match && downloaded >= total,
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

    pub fn complete_items(&self) {
        self.update(|data| {
            data.progress = match data.progress {
                Progress::Item { total, .. } => Progress::Item {
                    current: total,
                    total,
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
    checked_progress: f32,
}

#[derive(Clone, Debug)]
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
    /// Progress that has been checked into the main progress bar
    checked_progress: Mutex<f32>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
        // A completed subtask must read 1.0 even with a zero/unknown total, or its
        // weight share would permanently drag the aggregate below 100%.
        if self.is_complete() {
            return 1.0;
        }

        // Clamped so a miscounted total (or a 0 total before the first update) can never
        // surface as NaN or >100% in the UI.
        match self {
            Self::Download {
                downloaded, total, ..
            } if total > 0 => (downloaded as f32 / total as f32).min(1.0),
            Self::Item { current, total } if total > 0 => (current as f32 / total as f32).min(1.0),
            _ => 0.0,
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
    #[tracing_test::traced_test]
    async fn test() {
        let app = crate::setup_managers_for_test().await;

        let task = VisualTask::new(Translation::Test);
        let id = app.task_manager().spawn_task(&task).await;

        let subtask = task.subtask(Translation::Test);

        subtask.start_opaque();

        let mut tasks = vec![domain::Task {
            id,
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

    /// The watcher loop caps invalidations at ~5/s per task, but the first
    /// update must still invalidate immediately and the value the task settles
    /// on must always be delivered — otherwise the UI would sit on a stale
    /// progress figure until something unrelated woke it.
    #[tokio::test]
    async fn progress_invalidations_are_throttled_but_keep_first_and_last() {
        let app = crate::setup_managers_for_test().await;
        let mut events = app.invalidation_channel.subscribe();

        let task = VisualTask::new(Translation::Test);
        let _id = app.task_manager().spawn_task(&task).await;
        let subtask = task.subtask(Translation::Test);
        task.edit(|data| data.state = TaskState::KnownProgress)
            .await;

        // A burst standing in for a fast scan: one progress update per file,
        // as fast as the files are processed.
        // Spaced like a real scan: hashing a file takes milliseconds, which is
        // slow enough for the watcher to observe every update individually. A
        // tight loop instead lets the watch channel coalesce the whole burst on
        // its own, which would exercise nothing.
        const N: u32 = 100;
        for i in 1..=N {
            subtask.update_items(i, N);
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }

        // Long enough for several throttle windows to elapse.
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        let mut count = 0usize;
        loop {
            match events.try_recv() {
                Ok(_) => count += 1,
                // The unthrottled path can overrun the broadcast buffer; those
                // dropped events still count as events that were emitted.
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => count += n as usize,
                Err(_) => break,
            }
        }

        println!("invalidations emitted for {N} spaced updates: {count}");

        assert!(
            count >= 2,
            "expected the leading invalidation plus a trailing one after the \
             burst, got {count}"
        );
        assert!(
            count <= 30,
            "{N} rapid updates must collapse into a handful of invalidations, \
             got {count}"
        );
        assert_eq!(
            app.task_manager().get_tasks().await[0].progress,
            domain::Progress::Known(1.0),
            "the settled progress value must be what a refetch would read"
        );
    }
}

#[derive(Error, Debug)]
#[error("task id does not refer to a valid task")]
pub struct InvalidTaskIdError;

#[derive(Error, Debug)]
#[error("tasks that are not in a failed state cannot be dismissed")]
pub struct NonFailedDismissError;
