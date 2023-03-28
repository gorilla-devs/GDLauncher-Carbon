use std::sync::Arc;

use tokio::sync::{watch, RwLock};

struct VisualTaskManager {
    tasks: RwLock<Vec<VisualTask>>,
}

impl VisualTaskManager {
    fn new() -> Self {
        Self {
            tasks: RwLock::new(Vec::new()),
        }
    }
}

struct VisualTask {
    pub name: String,
    /// the indeterminate flag hides the progress bar before tasks have decided
    /// their respective weights.
    pub indeterminate: bool,
    notify_rx: watch::Receiver<()>,
    notify_tx: Arc<watch::Sender<()>>,
    subtasks: Vec<watch::Receiver<SubtaskData>>,
}

impl VisualTask {
    pub fn new(name: String) -> Self {
        let (notify_tx, notify_rx) = watch::channel(());

        Self {
            name,
            indeterminate: true,
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

    // Get the current task progress as a float from 0.0 to 1.0
    pub fn get_progress_float(&self) -> f32 {
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
            .sum::<f32>()
    }
}

struct Subtask {
    notify: Arc<watch::Sender<()>>,
    data: watch::Sender<SubtaskData>,
}

impl Subtask {
    pub async fn update(&self, f: impl FnOnce(&mut SubtaskData)) {
        self.data.send_modify(f);
        let _ = self.notify.send(());
    }

    // convenience functions

    pub async fn start_task(&self) {
        self.update(|data| data.started = true).await;
    }

    pub async fn update_progress(&self, progress: Progress) {
        self.update(|data| data.progress = progress).await;
    }

    pub async fn update_download(&self, downloaded: u32, total: u32) {
        self.update_progress(Progress::Download { downloaded, total })
            .await;
    }

    pub async fn update_items(&self, current: u32, total: u32) {
        self.update_progress(Progress::Item { current, total })
            .await;
    }

    pub async fn complete_opaque(&self) {
        self.update_progress(Progress::Opaque(true)).await;
    }

    pub async fn set_weight(&self, weight: f32) {
        self.update(|data| data.weight = weight).await;
    }
}

async fn test(st: Subtask) {
    st.update(|task| {
        task.progress = Progress::Download {
            downloaded: 0,
            total: 0,
        }
    })
    .await;
}

struct SubtaskData {
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
