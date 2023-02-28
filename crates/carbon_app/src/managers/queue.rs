use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU32, Ordering},
};

use rspc::Type;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct TaskHandle(u32);

impl TaskHandle {
    pub fn new() -> Self {
        static TASK_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

        Self(TASK_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Type, Serialize, Clone)]
pub struct TaskStatus {
    /// additional status details
    subtext: Option<String>,
    /// current task progress where None means indeterminate
    progress: Option<TaskProgress>,
}

#[derive(Type, Serialize, Clone)]
pub struct TaskProgress {
    /// current progress in `unit`s
    current: u64,
    /// total progress in `unit`s. None if unknown.
    total: Option<u64>,
    unit: ProgressUnit,
}

#[derive(Type, Serialize, Clone)]
pub enum ProgressUnit {
    SizeBytes,
    Count,
}

pub struct QueuedTask {
    handle: TaskHandle,
    name: String,
    start: Box<dyn FnOnce(TaskHandle) + Send + Sync>,
    // this can be changed if and when we need any resource type other than
    // a download slot.
    requires_download_slot: bool,
    /// Prerequisites must be queued in the executor BEFORE this task or
    /// they may be skipped.
    prerequisites: Vec<TaskHandle>,
}

pub struct ActiveTask {
    handle: TaskHandle,
    name: String,
    requires_download_slot: bool,
    status: TaskStatus,
}

pub struct TaskQueue {
    queue: RwLock<VecDeque<QueuedTask>>,
    active: RwLock<VecDeque<ActiveTask>>,
    download_slots: RwLock<DownloadSlots>,
}

struct DownloadSlots {
    used: usize,
    // this allows changing download slot count at runtime
    total: usize,
}

#[derive(Type, Serialize)]
pub struct TaskListEntry {
    handle: TaskHandle,
    name: String,
    state: TaskEntryState,
}

#[derive(Type, Serialize)]
pub enum TaskEntryState {
    Active,
    Queued,
}

impl TaskQueue {
    pub fn new(download_slots: usize) -> Self {
        Self {
            queue: RwLock::new(VecDeque::new()),
            active: RwLock::new(VecDeque::new()),
            download_slots: RwLock::new(DownloadSlots {
                used: 0,
                total: download_slots,
            }),
        }
    }

    async fn can_start_task(
        &self,
        task: &QueuedTask,
        queue: &VecDeque<QueuedTask>,
        active: &VecDeque<ActiveTask>,
    ) -> bool {
        if !task.requires_download_slot || {
            let slots = self.download_slots.read().await;
            slots.used < slots.total
        } {
            for &prerequisite in &task.prerequisites {
                if queue.iter().any(|task| task.handle == prerequisite)
                    || active.iter().any(|task| task.handle == prerequisite)
                {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    /// Queue a task or run it immediately if possible
    pub async fn queue(&mut self, task: QueuedTask) {
        if self
            .can_start_task(&task, &*self.queue.read().await, &*self.active.read().await)
            .await
        {
            self.start_task(task).await;
        } else {
            self.queue.write().await.push_back(task);
        }
    }

    async fn start_task(&self, task: QueuedTask) {
        if task.requires_download_slot {
            self.download_slots.write().await.used += 1;
        }

        self.active.write().await.push_back(ActiveTask {
            handle: task.handle,
            name: task.name,
            requires_download_slot: task.requires_download_slot,
            status: TaskStatus {
                subtext: None,
                progress: None,
            },
        });

        (task.start)(task.handle);
    }

    /// Start all tasks that can be started
    async fn start_tasks(&self) {
        let mut queue = self.queue.write().await;
        let mut i = 0;
        while let Some(queued) = queue.get(i) {
            if self
                .can_start_task(queued, &*queue, &*self.active.read().await)
                .await
            {
                let task = queue.remove(i).unwrap();
                // not a deadlock, start_task uses self.active
                self.start_task(task).await;
            } else {
                i += 1;
            }
        }
    }

    fn get_active_index(active: &VecDeque<ActiveTask>, task: TaskHandle) -> Option<usize> {
        active
            .iter()
            .enumerate()
            .find(|(_, t)| t.handle == task)
            .map(|(i, _)| i)
    }

    pub async fn complete(&self, task: TaskHandle) {
        let task = {
            let mut active = self.active.write().await;
            let Some(idx) = Self::get_active_index(&*active, task) else { return };
            let Some(task) = active.remove(idx) else { return };

            task
        };

        if task.requires_download_slot {
            self.download_slots.write().await.used -= 1;
        }

        self.start_tasks().await;
    }

    pub async fn update(&self, task: TaskHandle, status: TaskStatus) {
        let mut active = self.active.write().await;
        let Some(idx) = Self::get_active_index(&*active, task) else { return };
        let Some(task) = active.get_mut(idx) else { return };

        task.status = status;
    }

    pub async fn get_tasks(&self) -> Vec<TaskListEntry> {
        let mut entries = Vec::<TaskListEntry>::new();

        entries.extend(self.active.read().await.iter().map(|task| TaskListEntry {
            handle: task.handle,
            name: task.name.clone(),
            state: TaskEntryState::Active,
        }));

        entries.extend(self.queue.read().await.iter().map(|task| TaskListEntry {
            handle: task.handle,
            name: task.name.clone(),
            state: TaskEntryState::Queued,
        }));

        entries
    }

    pub async fn get_task_status(&self, task: TaskHandle) -> Option<TaskStatus> {
        // only active tasks can have a status
        self.active
            .read()
            .await
            .iter()
            .find(|active| active.handle == task)
            .map(|task| task.status.clone())
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use ntest::timeout;

    use super::{QueuedTask, TaskHandle, TaskQueue};

    #[tokio::test]
    #[timeout(1)]
    async fn download_concurrency() {
        let mut queue = TaskQueue::new(2);

        let task = || {
            let trigger = Arc::new(Mutex::new(false));
            let trigger2 = trigger.clone();

            let task = QueuedTask {
                handle: TaskHandle::new(),
                name: String::new(),
                start: Box::new(move |_| *trigger.lock().unwrap() = true),
                requires_download_slot: true,
                prerequisites: Vec::new(),
            };

            (task.handle, task, trigger2)
        };

        let (a_handle, a_task, a_trigger) = task();
        let (_, b_task, b_trigger) = task();
        let (_, c_task, c_trigger) = task();

        queue.queue(a_task).await;
        assert_eq!(*a_trigger.lock().unwrap(), true);
        queue.queue(b_task).await;
        assert_eq!(*b_trigger.lock().unwrap(), true);
        queue.queue(c_task).await;
        assert_eq!(*c_trigger.lock().unwrap(), false);
        queue.complete(a_handle).await;
        assert_eq!(*c_trigger.lock().unwrap(), true);
    }

    #[tokio::test]
    #[timeout(1)]
    async fn task_dependence() {
        let mut queue = TaskQueue::new(0);

        let a_handle = TaskHandle::new();
        let b_handle = TaskHandle::new();

        let b_trigger = Arc::new(Mutex::new(false));

        let a_task = QueuedTask {
            handle: a_handle,
            name: String::new(),
            start: Box::new(|_| {}),
            requires_download_slot: false,
            prerequisites: Vec::new(),
        };

        let b_task = QueuedTask {
            handle: b_handle,
            name: String::new(),
            start: Box::new({
                let trigger = b_trigger.clone();

                move |_| *trigger.lock().unwrap() = true
            }),
            requires_download_slot: false,
            prerequisites: vec![a_handle],
        };

        queue.queue(a_task).await;
        queue.queue(b_task).await;
        assert_eq!(*b_trigger.lock().unwrap(), false);
        queue.complete(a_handle).await;
        assert_eq!(*b_trigger.lock().unwrap(), true);
    }
}
