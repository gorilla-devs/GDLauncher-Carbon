pub struct Task {
    name: String,
    progress: Progress,
    downloaded: u32,
    active_subtasks: Vec<Subtask>,
}

pub enum Progress {
    Indeterminate,
    Known(f32),
}

pub struct Subtask {
    name: String,
    progress: SubtaskProgress,
}

pub enum SubtaskProgress {
    Download { downloaded: u32, total: u32 },
    Item { current: u32, total: u32 },
    Opaque,
}
