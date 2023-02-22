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

pub enum TaskProgress {
    Known(f32),
    Indeterminate,
}

pub struct QueuedTask {
    handle: TaskHandle,
    start: Box<dyn FnOnce() -> ActiveTask>,
    // this can be changed if and when we need any resource type other than
    // a download slot.
    requires_download_slot: bool,
    /// Prerequisites must be queued in the executor BEFORE this task or
    /// they may be skipped.
    prerequisites: Vec<TaskHandle>,
}

pub struct ActiveTask {
    handle: TaskHandle,
    requires_download_slot: bool,
    pause_fn: Option<Box<dyn FnOnce() -> QueuedTask>>,
    progress: TaskProgress,
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
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            active: VecDeque::new(),
            paused: VecDeque::new(),
            used_download_slots: 0,
            total_download_slots: 0,
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

        self.active.push_back((task.start)());
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

    pub fn update(&mut self, task: TaskHandle, progress: TaskProgress) {
        let Some(idx) = self.get_active_index(task) else { return };
        let Some(task) = self.active.get_mut(idx) else { return };

        task.progress = progress;
    }
}
