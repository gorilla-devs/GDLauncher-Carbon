// lib.rs

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weight {
    Lowest = 1,
    Low = 2,
    Medium = 3,
    High = 5,
    Highest = 10,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProgressUnit {
    Bytes,
    Items,
    Percentage,
    Custom(String),
}

#[derive(Clone, Debug)]
pub struct Progress {
    current: f64,
    total: f64,
    unit: ProgressUnit,
}

impl Progress {
    pub fn new(current: f64, total: f64, unit: ProgressUnit) -> Self {
        Self {
            current,
            total,
            unit,
        }
    }

    pub fn fraction(&self) -> f64 {
        if self.total == 0.0 {
            0.0
        } else {
            self.current / self.total
        }
    }
}

#[derive(Clone, Debug)]
pub enum SubtaskState {
    Inactive,
    Active(Progress),
    Completed,
}

pub struct SubtaskInfo<N: Clone> {
    name: N,
    state: SubtaskState,
    weight: Weight,
}

pub struct VTask<N: Clone> {
    name: N,
    subtasks: HashMap<N, Arc<RwLock<SubtaskInfo<N>>>>,
    total_weight: u32,
    progress_tx: mpsc::UnboundedSender<f64>,
}

impl<N: Clone + std::hash::Hash + Eq> VTask<N> {
    pub fn new(name: N, subtasks: Vec<(N, Weight)>) -> (Self, mpsc::UnboundedReceiver<f64>) {
        let mut task_subtasks = HashMap::new();
        let mut total_weight = 0;

        for (subtask_name, weight) in subtasks {
            let info = SubtaskInfo {
                name: subtask_name.clone(),
                state: SubtaskState::Inactive,
                weight,
            };
            task_subtasks.insert(subtask_name.clone(), Arc::new(RwLock::new(info)));
            total_weight += weight as u32;
        }

        let (progress_tx, progress_rx) = mpsc::unbounded_channel();

        (
            VTask {
                name,
                subtasks: task_subtasks,
                total_weight,
                progress_tx,
            },
            progress_rx,
        )
    }

    pub async fn start_subtask(&self, name: &N) -> Result<()> {
        let subtask = self
            .subtasks
            .get(name)
            .ok_or_else(|| anyhow!("Subtask not found"))?;

        let mut info = subtask.write().await;
        match info.state {
            SubtaskState::Inactive => {
                info.state =
                    SubtaskState::Active(Progress::new(0.0, 100.0, ProgressUnit::Percentage));
                drop(info);
                self.update_progress().await;
                Ok(())
            }
            SubtaskState::Active(_) => Err(anyhow!("Subtask is already active")),
            SubtaskState::Completed => Err(anyhow!("Subtask is already completed")),
        }
    }

    pub async fn set_progress(&self, name: &N, progress: Progress) -> Result<()> {
        let subtask = self
            .subtasks
            .get(name)
            .ok_or_else(|| anyhow!("Subtask not found"))?;

        let mut info = subtask.write().await;
        match &mut info.state {
            SubtaskState::Active(_) => {
                info.state = SubtaskState::Active(progress);
                drop(info);
                self.update_progress().await;
                Ok(())
            }
            SubtaskState::Inactive => Err(anyhow!("Subtask is not active")),
            SubtaskState::Completed => Err(anyhow!("Subtask is already completed")),
        }
    }

    pub async fn complete_subtask(&self, name: &N) -> Result<()> {
        let subtask = self
            .subtasks
            .get(name)
            .ok_or_else(|| anyhow!("Subtask not found"))?;

        let mut info = subtask.write().await;
        match info.state {
            SubtaskState::Completed => Err(anyhow!("Subtask is already completed")),
            _ => {
                info.state = SubtaskState::Completed;
                drop(info);
                self.update_progress().await;
                Ok(())
            }
        }
    }

    async fn update_progress(&self) {
        let progress = self.calculate_progress().await;
        let _ = self.progress_tx.send(progress);
    }

    async fn calculate_progress(&self) -> f64 {
        let mut completed_weight = 0.0;
        let total_weight = self.total_weight as f64;

        for subtask in self.subtasks.values() {
            let info = subtask.read().await;
            let weight = info.weight as u32 as f64;
            match &info.state {
                SubtaskState::Completed => {
                    completed_weight += weight;
                }
                SubtaskState::Active(progress) => {
                    completed_weight += progress.fraction() * weight;
                }
                SubtaskState::Inactive => {}
            }
        }

        completed_weight / total_weight
    }

    pub async fn get_active_subtasks(&self) -> Vec<N> {
        let mut active_subtasks = Vec::new();
        for subtask in self.subtasks.values() {
            let info = subtask.read().await;
            if let SubtaskState::Active(_) = info.state {
                active_subtasks.push(info.name.clone());
            }
        }
        active_subtasks
    }

    pub async fn get_current_progress(&self) -> f64 {
        self.calculate_progress().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    enum TaskName {
        MainTask,
        Subtask1,
        Subtask2,
    }

    async fn collect_progress_updates(
        rx: &mut mpsc::UnboundedReceiver<f64>,
        duration: Duration,
    ) -> Vec<f64> {
        let mut updates = Vec::new();
        let start = std::time::Instant::now();
        while let Ok(Some(progress)) = timeout(Duration::from_millis(10), rx.recv()).await {
            updates.push(progress);
            if start.elapsed() > duration {
                break;
            }
        }
        updates
    }

    #[tokio::test]
    async fn test_vtask_creation_and_progress() -> Result<()> {
        let (task, mut progress_rx) = VTask::new(
            TaskName::MainTask,
            vec![
                (TaskName::Subtask1, Weight::Low),
                (TaskName::Subtask2, Weight::High),
            ],
        );

        assert_eq!(task.subtasks.len(), 2);
        assert_eq!(task.total_weight, 7); // 2 + 5

        // Start Subtask1
        task.start_subtask(&TaskName::Subtask1).await?;

        // Set progress for Subtask1
        task.set_progress(
            &TaskName::Subtask1,
            Progress::new(50.0, 100.0, ProgressUnit::Percentage),
        )
        .await?;

        // Start and set progress for Subtask2
        task.start_subtask(&TaskName::Subtask2).await?;
        task.set_progress(
            &TaskName::Subtask2,
            Progress::new(2.5, 5.0, ProgressUnit::Items),
        )
        .await?;

        // Complete Subtask1
        task.complete_subtask(&TaskName::Subtask1).await?;

        // Collect progress updates
        let updates = collect_progress_updates(&mut progress_rx, Duration::from_millis(100)).await;

        assert!(!updates.is_empty(), "No progress updates received");

        let final_progress = *updates.last().unwrap();
        let expected_progress = (2.0 + 0.5 * 5.0) / 7.0; // (completed Subtask1 + 50% of Subtask2) / total weight
        assert!(
            (final_progress - expected_progress).abs() < 0.001,
            "Expected progress close to {}, got {}",
            expected_progress,
            final_progress
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_subtask_state_changes() -> Result<()> {
        let (task, mut progress_rx) = VTask::new(
            TaskName::MainTask,
            vec![
                (TaskName::Subtask1, Weight::Low),
                (TaskName::Subtask2, Weight::High),
            ],
        );

        // Start Subtask1
        task.start_subtask(&TaskName::Subtask1).await?;

        // Set progress for Subtask1
        task.set_progress(
            &TaskName::Subtask1,
            Progress::new(30.0, 100.0, ProgressUnit::Percentage),
        )
        .await?;

        // Start Subtask2
        task.start_subtask(&TaskName::Subtask2).await?;

        // Set progress for Subtask2
        task.set_progress(
            &TaskName::Subtask2,
            Progress::new(100.0, 100.0, ProgressUnit::Bytes),
        )
        .await?;

        // Complete Subtask2
        task.complete_subtask(&TaskName::Subtask2).await?;

        // Collect progress updates
        let updates = collect_progress_updates(&mut progress_rx, Duration::from_millis(100)).await;

        assert!(!updates.is_empty(), "No progress updates received");

        let final_progress = *updates.last().unwrap();
        let expected_progress = (0.3 * 2.0 + 1.0 * 5.0) / 7.0; // (30% of Subtask1 + completed Subtask2) / total weight
        assert!(
            (final_progress - expected_progress).abs() < 0.001,
            "Expected progress close to {}, got {}",
            expected_progress,
            final_progress
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_active_subtasks() -> Result<()> {
        let (task, _progress_rx) = VTask::new(
            TaskName::MainTask,
            vec![
                (TaskName::Subtask1, Weight::Low),
                (TaskName::Subtask2, Weight::High),
            ],
        );

        let active = task.get_active_subtasks().await;
        assert_eq!(active.len(), 0);

        task.start_subtask(&TaskName::Subtask1).await?;

        let active = task.get_active_subtasks().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], TaskName::Subtask1);

        task.start_subtask(&TaskName::Subtask2).await?;
        task.complete_subtask(&TaskName::Subtask1).await?;

        let active = task.get_active_subtasks().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], TaskName::Subtask2);

        Ok(())
    }
}
