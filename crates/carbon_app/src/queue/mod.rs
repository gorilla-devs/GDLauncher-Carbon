use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Copy, Clone, PartialEq)]
pub struct TaskHandle(usize);

impl TaskHandle {
    pub fn new() -> Self {
        static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

        Self(TASK_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct TaskStatus {
    /// additional status details
    subtext: Option<String>,
    /// current task progress where None means indeterminate
    progress: Option<TaskProgress>,
}

pub struct TaskProgress {
    /// current progress in `unit`s
    current: u64,
    /// total progress in `unit`s. None if unknown.
    total: Option<u64>,
    unit: ProgressUnit,
}

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
    queue: VecDeque<QueuedTask>,
    active: VecDeque<ActiveTask>,
    paused: VecDeque<QueuedTask>,
    // this allows changing download slot count at runtime
    used_download_slots: usize,
    total_download_slots: usize,
}

impl TaskQueue {
    pub fn new(download_slots: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            active: VecDeque::new(),
            paused: VecDeque::new(),
            used_download_slots: 0,
            total_download_slots: download_slots,
        }
    }

    fn can_start_task(&self, task: &QueuedTask) -> bool {
        if !task.requires_download_slot || self.used_download_slots < self.total_download_slots {
            for &prereqisite in &task.prerequisites {
                if self.queue.iter().any(|task| task.handle == prereqisite)
                    || self.active.iter().any(|task| task.handle == prereqisite)
                    || self.paused.iter().any(|task| task.handle == prereqisite)
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
    pub fn queue(&mut self, task: QueuedTask) {
        if self.can_start_task(&task) {
            self.start_task(task);
        } else {
            self.queue.push_back(task);
        }
    }

    fn start_task(&mut self, task: QueuedTask) {
        if task.requires_download_slot {
            self.used_download_slots += 1;
        }

        self.active.push_back(ActiveTask {
            handle: task.handle,
            name: task.name,
            requires_download_slot: task.requires_download_slot,
            status: TaskStatus { subtext: None, progress: None },
        });

        (task.start)(task.handle);
    }

    /// Start all tasks that can be started
    fn start_tasks(&mut self) {
        let mut i = 0;
        while let Some(queued) = self.queue.get(i) {
            if self.can_start_task(queued) {
                let task = self.queue.remove(i).unwrap();
                self.start_task(task);
            } else {
                i += 1;
            }
        }
    }

    fn get_active_index(&self, task: TaskHandle) -> Option<usize> {
        self.active
            .iter()
            .enumerate()
            .find(|(_, t)| t.handle == task)
            .map(|(i, _)| i)
    }

    pub fn complete(&mut self, task: TaskHandle) {
        let Some(idx) = self.get_active_index(task) else { return };
        let Some(task) = self.active.remove(idx) else { return };

        if task.requires_download_slot {
            self.used_download_slots -= 1;
        }

        self.start_tasks();
    }

    pub fn update(&mut self, task: TaskHandle, status: TaskStatus) {
        let Some(idx) = self.get_active_index(task) else { return };
        let Some(task) = self.active.get_mut(idx) else { return };

        task.status = status;
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use super::{ActiveTask, QueuedTask, TaskHandle, TaskProgress, TaskQueue};

    #[test]
    fn download_concurrency() {
        let mut queue = TaskQueue::new(2);

        let task = || {
            let trigger = Arc::new(Mutex::new(false));
            let trigger2 = trigger.clone();

            let task = QueuedTask {
                handle: TaskHandle::new(),
                start: Box::new(move |_| *trigger.lock().unwrap() = true),
                requires_download_slot: true,
                prerequisites: Vec::new(),
            };

            (task.handle, task, trigger2)
        };

        let (a_handle, a_task, a_trigger) = task();
        let (_, b_task, b_trigger) = task();
        let (_, c_task, c_trigger) = task();

        queue.queue(a_task);
        assert_eq!(*a_trigger.lock().unwrap(), true);
        queue.queue(b_task);
        assert_eq!(*b_trigger.lock().unwrap(), true);
        queue.queue(c_task);
        assert_eq!(*c_trigger.lock().unwrap(), false);
        queue.complete(a_handle);
        assert_eq!(*c_trigger.lock().unwrap(), true);
    }

    #[test]
    fn task_dependence() {
        let mut queue = TaskQueue::new(0);

        let a_handle = TaskHandle::new();
        let b_handle = TaskHandle::new();

        let b_trigger = Arc::new(Mutex::new(false));

        let a_task = QueuedTask {
            handle: a_handle,
            start: Box::new(|_| {}),
            requires_download_slot: false,
            prerequisites: Vec::new(),
        };

        let b_task = QueuedTask {
            handle: b_handle,
            start: Box::new({
                let trigger = b_trigger.clone();

                move |_| *trigger.lock().unwrap() = true
            }),
            requires_download_slot: false,
            prerequisites: vec![a_handle],
        };

        queue.queue(a_task);
        queue.queue(b_task);
        assert_eq!(*b_trigger.lock().unwrap(), false);
        queue.complete(a_handle);
        assert_eq!(*b_trigger.lock().unwrap(), true);
    }
}
