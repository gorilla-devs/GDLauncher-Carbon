#[derive(Debug, PartialEq)]
pub struct Task {
    pub name: String,
    pub progress: Progress,
    pub downloaded: u32,
    pub download_total: u32,
    pub active_subtasks: Vec<Subtask>,
}

#[derive(Debug, PartialEq)]
pub enum Progress {
    Indeterminate,
    Known(f32),
}

#[derive(Debug, PartialEq)]
pub struct Subtask {
    pub name: String,
    pub progress: SubtaskProgress,
}

#[derive(Debug, PartialEq)]
pub enum SubtaskProgress {
    Download { downloaded: u32, total: u32 },
    Item { current: u32, total: u32 },
    Opaque,
}
