pub struct Task {
    pub name: String,
    pub progress: Progress,
    pub downloaded: u32,
    pub download_total: u32,
    pub active_subtasks: Vec<Subtask>,
}

pub enum Progress {
    Indeterminate,
    Known(f32),
}

pub struct Subtask {
    pub name: String,
    pub progress: SubtaskProgress,
}

pub enum SubtaskProgress {
    Download { downloaded: u32, total: u32 },
    Item { current: u32, total: u32 },
    Opaque,
}
