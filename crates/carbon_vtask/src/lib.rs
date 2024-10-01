use anyhow::{anyhow, Result};
use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::{watch, Mutex, RwLock},
    time::timeout,
};

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

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct VTaskId(pub u32);

#[derive(Clone, Debug)]
pub enum VTaskCommand {
    Pause,
    Resume,
    Cancel,
    Complete,
}

pub struct SubtaskInfo<N: Clone> {
    name: N,
    progress: Option<Progress>,
    weight: Weight,
    cleanup: Option<Arc<dyn Fn() -> Result<()> + Send + Sync>>,
}

#[derive(Clone, Debug)]
pub enum VTaskState {
    Running,
    Paused,
    Cancelled,
    Completed,
    Failed,
}

pub struct VTask<N: Clone> {
    id: VTaskId,
    name: N,
    subtasks: RwLock<HashMap<N, Arc<RwLock<SubtaskInfo<N>>>>>,
    total_weight: RwLock<u32>,
    progress_tx: watch::Sender<(VTaskId, f64)>,
    command_rx: watch::Receiver<VTaskCommand>,
    command_tx: watch::Sender<VTaskCommand>,
    state: Arc<RwLock<VTaskState>>,
}

impl<N: Clone + Display + std::hash::Hash + Eq + Send + Sync + 'static> VTask<N> {
    pub async fn handle_command(&self, command: VTaskCommand) -> Result<()> {
        match command {
            VTaskCommand::Pause => {
                *self.state.write().await = VTaskState::Paused;
            }
            VTaskCommand::Resume => {
                *self.state.write().await = VTaskState::Running;
            }
            VTaskCommand::Cancel => {
                *self.state.write().await = VTaskState::Cancelled;
                self.cancel_subtasks().await?;
                self.execute_cleanup().await?;
            }
            VTaskCommand::Complete => {
                *self.state.write().await = VTaskState::Completed;
                self.cancel_subtasks().await?;
            }
        }
        let _ = self.command_tx.send(command);
        Ok(())
    }

    pub async fn fail(&self) -> Result<()> {
        *self.state.write().await = VTaskState::Failed;
        self.cancel_subtasks().await?;
        self.execute_cleanup().await?;
        Ok(())
    }

    async fn cancel_subtasks(&self) -> Result<()> {
        let mut subtasks = self.subtasks.write().await;
        for subtask in subtasks.values() {
            let info = subtask.write().await;
            if let Some(cleanup) = &info.cleanup {
                cleanup()?;
            }
        }
        subtasks.clear();
        *self.total_weight.write().await = 0;
        Ok(())
    }

    async fn execute_cleanup(&self) -> Result<()> {
        let subtasks = self.subtasks.read().await;
        for subtask in subtasks.values() {
            let info = subtask.read().await;
            if let Some(cleanup) = &info.cleanup {
                cleanup()?;
            }
        }
        Ok(())
    }

    pub fn new(
        id: VTaskId,
        name: N,
        subtasks: Vec<(N, Weight)>,
    ) -> (Arc<Self>, watch::Receiver<(VTaskId, f64)>) {
        let mut task_subtasks = HashMap::new();
        let mut total_weight = 0;

        for (subtask_name, weight) in subtasks {
            let info = SubtaskInfo {
                name: subtask_name.clone(),
                progress: None,
                weight,
                cleanup: None,
            };
            task_subtasks.insert(subtask_name.clone(), Arc::new(RwLock::new(info)));
            total_weight += weight as u32;
        }

        let (progress_tx, progress_rx) = watch::channel((id, 0.0));
        let (command_tx, command_rx) = watch::channel(VTaskCommand::Resume);

        (
            Arc::new(VTask {
                id,
                name,
                subtasks: RwLock::new(task_subtasks),
                total_weight: RwLock::new(total_weight),
                progress_tx,
                command_rx,
                command_tx,
                state: Arc::new(RwLock::new(VTaskState::Running)),
            }),
            progress_rx,
        )
    }

    pub async fn run_subtask<F, Fut>(&self, name: &N, f: F) -> Result<()>
    where
        F: FnOnce(SubtaskContext) -> Fut,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        println!("Entering run_subtask for {}", name);
        let subtask = {
            let subtasks = self.subtasks.read().await;
            println!("Acquired read lock on subtasks");
            subtasks
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("Subtask not found"))?
        };
        println!("Retrieved subtask");

        let context = SubtaskContext {
            state: self.state.clone(),
            command_rx: self.command_rx.clone(),
        };
        println!("Created SubtaskContext");

        println!("About to execute subtask function");
        let result = f(context).await;
        println!("Subtask function completed");

        println!("Attempting to acquire write lock on subtasks");
        let remove_result = timeout(Duration::from_secs(5), async {
            let mut subtasks = self.subtasks.write().await;
            println!("Acquired write lock on subtasks");
            if let Some(removed_subtask) = subtasks.remove(name) {
                println!("Removed subtask from list");
                let info = removed_subtask.read().await;
                let weight = info.weight as u32;
                drop(info);
                let mut total_weight = self.total_weight.write().await;
                *total_weight -= weight;
                println!("Updated total weight");
            }
            println!("Finished subtask removal process");
        })
        .await;

        match remove_result {
            Ok(_) => println!("Successfully removed subtask"),
            Err(_) => println!("Timed out while trying to remove subtask"),
        }

        println!("About to update progress");
        self.update_progress().await;
        println!("Progress updated");

        println!("Exiting run_subtask");
        result
    }

    pub async fn set_subtask_progress(&self, name: &N, progress: Option<Progress>) -> Result<()> {
        let subtasks = self.subtasks.read().await;
        let subtask = subtasks
            .get(name)
            .ok_or_else(|| anyhow!("Subtask not found"))?;

        let mut info = subtask.write().await;
        info.progress = progress;
        drop(info);
        drop(subtasks);
        self.update_progress().await;
        Ok(())
    }

    pub async fn set_subtask_cleanup<F>(&self, name: &N, cleanup: F) -> Result<()>
    where
        F: Fn() -> Result<()> + Send + Sync + 'static,
    {
        let subtasks = self.subtasks.read().await;
        let subtask = subtasks
            .get(name)
            .ok_or_else(|| anyhow!("Subtask not found"))?;

        let mut info = subtask.write().await;
        info.cleanup = Some(Arc::new(cleanup));
        Ok(())
    }

    async fn update_progress(&self) {
        let progress = self.calculate_progress().await;
        let _ = self.progress_tx.send((self.id, progress));
    }

    async fn calculate_progress(&self) -> f64 {
        let subtasks = self.subtasks.read().await;
        let mut completed_weight = 0.0;
        let total_weight = *self.total_weight.read().await as f64;

        for subtask in subtasks.values() {
            let info = subtask.read().await;
            let weight = info.weight as u32 as f64;
            if let Some(progress) = &info.progress {
                completed_weight += progress.fraction() * weight;
            }
        }

        if total_weight == 0.0 {
            1.0 // All subtasks completed
        } else {
            completed_weight / total_weight
        }
    }

    pub async fn get_subtask_progress(&self) -> HashMap<N, Option<Progress>> {
        let subtasks = self.subtasks.read().await;
        let mut progress = HashMap::new();
        for (name, subtask) in subtasks.iter() {
            let info = subtask.read().await;
            progress.insert(name.clone(), info.progress.clone());
        }
        progress
    }

    pub fn get_id(&self) -> VTaskId {
        self.id
    }

    pub fn get_name(&self) -> N {
        self.name.clone()
    }
}

#[derive(Clone)]
pub struct SubtaskContext {
    state: Arc<RwLock<VTaskState>>,
    command_rx: watch::Receiver<VTaskCommand>,
}

impl SubtaskContext {
    pub async fn check_pause_cancel(&mut self) -> Result<()> {
        loop {
            let state = self.state.read().await;
            match *state {
                VTaskState::Running => return Ok(()),
                VTaskState::Cancelled => return Err(anyhow!("Task cancelled")),
                VTaskState::Completed => return Err(anyhow!("Task completed")),
                VTaskState::Failed => return Err(anyhow!("Task failed")),
                VTaskState::Paused => {
                    drop(state);
                    tokio::select! {
                        result = self.command_rx.changed() => {
                            if result.is_err() {
                                return Err(anyhow!("Command channel closed"));
                            }
                            let command = self.command_rx.borrow().clone();
                            match command {
                                VTaskCommand::Resume => {
                                    *self.state.write().await = VTaskState::Running;
                                    return Ok(());
                                }
                                VTaskCommand::Cancel => {
                                    *self.state.write().await = VTaskState::Cancelled;
                                    return Err(anyhow!("Task cancelled"));
                                }
                                VTaskCommand::Complete => {
                                    *self.state.write().await = VTaskState::Completed;
                                    return Err(anyhow!("Task completed"));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

pub async fn interruptable<F, Fut, R>(context: &mut SubtaskContext, f: F) -> Result<R>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<R>>,
{
    context.check_pause_cancel().await?;
    f().await
}

pub struct VTaskManager<N: Clone + Display + std::hash::Hash + Eq + Send + Sync + 'static> {
    tasks: RwLock<HashMap<VTaskId, Arc<VTask<N>>>>,
    next_id: Mutex<u32>,
}

impl<N: Clone + Display + std::hash::Hash + Eq + Send + Sync + 'static> VTaskManager<N> {
    pub fn new() -> Self {
        VTaskManager {
            tasks: RwLock::new(HashMap::new()),
            next_id: Mutex::new(0),
        }
    }

    pub async fn create_task(
        &self,
        name: N,
        subtasks: Vec<(N, Weight)>,
    ) -> (VTaskId, watch::Receiver<(VTaskId, f64)>) {
        let mut id_guard = self.next_id.lock().await;
        let id = VTaskId(*id_guard);
        *id_guard += 1;
        drop(id_guard);

        let (task, progress_rx) = VTask::new(id, name, subtasks);
        self.tasks.write().await.insert(id, task);
        (id, progress_rx)
    }

    pub async fn get_task(&self, id: VTaskId) -> Option<Arc<VTask<N>>> {
        self.tasks.read().await.get(&id).cloned()
    }

    pub async fn remove_task(&self, id: VTaskId) -> Result<()> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks
                .get(&id)
                .cloned()
                .ok_or_else(|| anyhow!("Task not found"))?
        };

        let state = task.state.read().await;
        match *state {
            VTaskState::Completed | VTaskState::Cancelled | VTaskState::Failed => {
                drop(state); // Explicitly drop the state lock before modifying tasks
                self.tasks.write().await.remove(&id);
                Ok(())
            }
            _ => Err(anyhow!("Cannot remove an active task")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::{Duration, Instant},
    };
    use tokio::time::{sleep, timeout};

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    enum TaskName {
        MainTask,
        Subtask1,
        Subtask2,
    }

    impl Display for TaskName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TaskName::MainTask => write!(f, "MainTask"),
                TaskName::Subtask1 => write!(f, "Subtask1"),
                TaskName::Subtask2 => write!(f, "Subtask2"),
            }
        }
    }

    async fn collect_progress_updates(
        rx: &mut watch::Receiver<(VTaskId, f64)>,
        duration: Duration,
    ) -> Vec<(VTaskId, f64)> {
        let mut updates = Vec::new();
        let start = Instant::now();
        let timeout_duration = Duration::from_millis(10);

        while start.elapsed() < duration {
            match timeout(timeout_duration, rx.changed()).await {
                Ok(Ok(_)) => {
                    updates.push(*rx.borrow());
                }
                Ok(Err(_)) => break, // Channel closed
                Err(_) => {}         // Timeout, continue loop
            }
        }
        updates
    }

    #[tokio::test]
    async fn test_vtask_post_subtask_completion() -> Result<()> {
        println!("Starting test_vtask_post_subtask_completion");
        let manager = Arc::new(VTaskManager::new());
        let (id, mut progress_rx) = manager
            .create_task(TaskName::MainTask, vec![(TaskName::Subtask1, Weight::Low)])
            .await;

        let task = manager.get_task(id).await.unwrap();
        let task_for_assert = task.clone();

        println!("Spawning task execution");
        let task_handle = tokio::spawn(async move {
            let task_clone = task.clone();
            let task_clone_1 = task.clone();
            let task_clone_2 = task.clone();

            println!("Starting Subtask1");
            let subtask_result = task_clone
                .run_subtask(&TaskName::Subtask1, |mut context| async move {
                    interruptable(&mut context, || async {
                        task_clone_1
                            .set_subtask_progress(
                                &TaskName::Subtask1,
                                Some(Progress::new(100.0, 100.0, ProgressUnit::Percentage)),
                            )
                            .await?;
                        println!("Subtask1: Progress set to 100%");
                        Ok(())
                    })
                    .await?;

                    println!("Subtask1 completed");
                    Ok::<_, anyhow::Error>(())
                })
                .await;

            println!(
                "Subtask1 run_subtask completed with result: {:?}",
                subtask_result
            );

            println!("Checking task state after subtask completion");
            let state = task_clone_2.state.read().await;
            println!("Task state: {:?}", *state);
            drop(state);

            println!("Checking remaining subtasks");
            let subtasks = task_clone_2.subtasks.read().await;
            println!("Remaining subtasks: {}", subtasks.len());
            drop(subtasks);

            println!("Task execution completed");
        });

        println!("Waiting for task to finish");
        match timeout(Duration::from_secs(10), task_handle).await {
            Ok(result) => {
                println!("Task handle completed");
                result.expect("Task panicked");
            }
            Err(_) => panic!("Task execution timed out after 10 seconds"),
        }

        println!("Task handle finished, checking final state");
        let final_state = task_for_assert.state.read().await;
        println!("Final task state: {:?}", *final_state);
        drop(final_state);

        let final_subtasks = task_for_assert.subtasks.read().await;
        println!("Final remaining subtasks: {}", final_subtasks.len());
        drop(final_subtasks);

        println!("Collecting progress updates");
        let updates = collect_progress_updates(&mut progress_rx, Duration::from_millis(100)).await;

        assert!(!updates.is_empty(), "No progress updates received");

        let (final_id, final_progress) = updates.last().unwrap();
        println!(
            "Final progress update: id={:?}, progress={}",
            final_id, final_progress
        );

        println!("test_vtask_post_subtask_completion completed");
        Ok(())
    }

    #[tokio::test]
    async fn test_vtask_creation_and_progress() -> Result<()> {
        println!("Starting test_vtask_creation_and_progress");
        let manager = Arc::new(VTaskManager::new());
        let (id, mut progress_rx) = manager
            .create_task(
                TaskName::MainTask,
                vec![
                    (TaskName::Subtask1, Weight::Low),
                    (TaskName::Subtask2, Weight::High),
                ],
            )
            .await;

        let task = manager.get_task(id).await.unwrap();
        let task_for_assert = task.clone();

        assert_eq!(task.subtasks.read().await.len(), 2);
        assert_eq!(*task.total_weight.read().await, 7);

        println!("Spawning VTask execution");
        let vtask_handle = tokio::spawn(async move {
            let task_clone = task.clone();

            println!("Running Subtask1");
            match timeout(
                Duration::from_secs(2),
                task_clone.run_subtask(&TaskName::Subtask1, |mut context| {
                    let task_clone = task_clone.clone();
                    async move {
                        interruptable(&mut context, || async {
                            println!("Subtask1: Setting progress to 50%");
                            task_clone
                                .set_subtask_progress(
                                    &TaskName::Subtask1,
                                    Some(Progress::new(50.0, 100.0, ProgressUnit::Percentage)),
                                )
                                .await?;
                            Ok(())
                        })
                        .await?;

                        interruptable(&mut context, || async {
                            println!("Subtask1: Setting progress to 100%");
                            task_clone
                                .set_subtask_progress(
                                    &TaskName::Subtask1,
                                    Some(Progress::new(100.0, 100.0, ProgressUnit::Percentage)),
                                )
                                .await?;
                            Ok(())
                        })
                        .await?;

                        println!("Subtask1 completed");
                        Ok::<_, anyhow::Error>(())
                    }
                }),
            )
            .await
            {
                Ok(result) => result.unwrap(),
                Err(_) => panic!("Subtask1 timed out"),
            }

            println!("Running Subtask2");
            match timeout(
                Duration::from_secs(2),
                task_clone.run_subtask(&TaskName::Subtask2, |mut context| {
                    let task_clone = task_clone.clone();
                    async move {
                        interruptable(&mut context, || async {
                            println!("Subtask2: Setting progress to 50%");
                            task_clone
                                .set_subtask_progress(
                                    &TaskName::Subtask2,
                                    Some(Progress::new(2.5, 5.0, ProgressUnit::Items)),
                                )
                                .await?;
                            Ok(())
                        })
                        .await?;

                        interruptable(&mut context, || async {
                            println!("Subtask2: Setting progress to 100%");
                            task_clone
                                .set_subtask_progress(
                                    &TaskName::Subtask2,
                                    Some(Progress::new(5.0, 5.0, ProgressUnit::Items)),
                                )
                                .await?;
                            Ok(())
                        })
                        .await?;

                        println!("Subtask2 completed");
                        Ok::<_, anyhow::Error>(())
                    }
                }),
            )
            .await
            {
                Ok(result) => result.unwrap(),
                Err(_) => panic!("Subtask2 timed out"),
            }

            println!("Both subtasks completed");
        });

        println!("Waiting for VTask to finish");
        match timeout(Duration::from_secs(10), vtask_handle).await {
            Ok(result) => result.expect("VTask panicked"),
            Err(_) => panic!("VTask execution timed out after 10 seconds"),
        }

        println!("Collecting progress updates");
        let updates = collect_progress_updates(&mut progress_rx, Duration::from_millis(100)).await;

        assert!(!updates.is_empty(), "No progress updates received");

        let (final_id, final_progress) = updates.last().unwrap();
        assert_eq!(*final_id, id, "Mismatched task ID in progress update");
        let expected_progress = 1.0;
        assert!(
            (final_progress - expected_progress).abs() < 0.001,
            "Expected progress close to {}, got {}",
            expected_progress,
            final_progress
        );

        println!("Checking final task state");
        assert_eq!(task_for_assert.subtasks.read().await.len(), 0);
        assert_eq!(*task_for_assert.total_weight.read().await, 0);

        println!("test_vtask_creation_and_progress completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_vtask_pause_resume() -> Result<()> {
        println!("Starting test_vtask_pause_resume");
        let manager = Arc::new(VTaskManager::new());
        let (id, mut progress_rx) = manager
            .create_task(
                TaskName::MainTask,
                vec![(TaskName::Subtask1, Weight::Medium)],
            )
            .await;

        let task = manager.get_task(id).await.unwrap();
        let task_for_assert = task.clone();

        let paused = Arc::new(AtomicBool::new(false));
        let resumed = Arc::new(AtomicBool::new(false));

        let paused_clone = paused.clone();
        let resumed_clone = resumed.clone();

        println!("Spawning VTask");
        let vtask_handle = tokio::spawn(async move {
            let task_clone = task.clone();
            task_clone
                .run_subtask(&TaskName::Subtask1, |mut context| {
                    let paused_clone = paused_clone.clone();
                    let resumed_clone = resumed_clone.clone();
                    async move {
                        println!("Subtask1: Before first pausable_async");
                        interruptable(&mut context, || async {
                            paused_clone.store(true, Ordering::SeqCst);
                            println!("Subtask1: Paused");
                            Ok(())
                        })
                        .await?;

                        sleep(Duration::from_millis(500)).await;

                        println!("Subtask1: Before second pausable_async");
                        interruptable(&mut context, || async {
                            resumed_clone.store(true, Ordering::SeqCst);
                            println!("Subtask1: Resumed");
                            Ok(())
                        })
                        .await?;

                        println!("Subtask1: Completed");
                        Ok::<_, anyhow::Error>(())
                    }
                })
                .await
                .unwrap();
        });

        println!("Pausing the task");
        sleep(Duration::from_millis(50)).await;
        task_for_assert.handle_command(VTaskCommand::Pause).await?;

        println!("Waiting for task to pause");
        sleep(Duration::from_millis(50)).await;
        assert!(paused.load(Ordering::SeqCst), "Task did not pause");
        assert!(!resumed.load(Ordering::SeqCst), "Task resumed prematurely");

        println!("Resuming the task");
        task_for_assert.handle_command(VTaskCommand::Resume).await?;

        println!("Waiting for task to complete");
        timeout(Duration::from_secs(5), vtask_handle)
            .await
            .expect("Test timed out")
            .expect("VTask panicked");

        assert!(resumed.load(Ordering::SeqCst), "Task did not resume");

        println!("test_vtask_pause_resume completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_vtask_cancellation() -> Result<()> {
        println!("Starting test_vtask_cancellation");
        let manager = Arc::new(VTaskManager::new());
        let (id, mut progress_rx) = manager
            .create_task(
                TaskName::MainTask,
                vec![(TaskName::Subtask1, Weight::Medium)],
            )
            .await;

        let task = manager.get_task(id).await.unwrap();

        let task_for_assert = task.clone();

        let completed = Arc::new(AtomicBool::new(false));

        let completed_clone = completed.clone();

        println!("Spawning VTask");
        let vtask_handle = tokio::spawn(async move {
            let task_clone = task.clone();
            task_clone
                .run_subtask(&TaskName::Subtask1, |mut context| async move {
                    let completed_clone = completed_clone.clone();

                    interruptable(&mut context, || async {
                        println!("Subtask1: Running");
                        sleep(Duration::from_millis(3000)).await;
                        println!("Subtask1: Completed");
                        Ok(())
                    })
                    .await?;

                    interruptable(&mut context, || async {
                        println!("Subtask1 part 2: Running");
                        completed_clone.store(true, Ordering::SeqCst);
                        Ok(())
                    })
                    .await?;

                    Ok::<_, anyhow::Error>(())
                })
                .await
        });

        sleep(Duration::from_millis(100)).await;

        println!("Cancelling the task");

        let init_time = Instant::now();

        task_for_assert.handle_command(VTaskCommand::Cancel).await?;

        println!("Cancellation took {:?}", init_time.elapsed());

        let vtask_handle = vtask_handle.await.unwrap();

        sleep(Duration::from_millis(50)).await;
        assert!(vtask_handle.is_err(), "Task should have errored");
        assert_eq!(vtask_handle.unwrap_err().to_string(), "Task cancelled",);

        assert!(
            !completed.load(Ordering::SeqCst),
            "Task should not have completed"
        );

        println!("Checking task state");
        let task_state = task_for_assert.state.read().await;
        assert!(
            matches!(*task_state, VTaskState::Cancelled),
            "Task state should be Cancelled"
        );

        assert_eq!(
            task_for_assert.subtasks.read().await.len(),
            0,
            "All subtasks should be removed"
        );
        assert_eq!(
            *task_for_assert.total_weight.read().await,
            0,
            "Total weight should be 0"
        );

        println!("test_vtask_cancellation completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_vtask_completion() -> Result<()> {
        println!("Starting test_vtask_completion");
        let manager = Arc::new(VTaskManager::new());
        let (id, mut progress_rx) = manager
            .create_task(
                TaskName::MainTask,
                vec![(TaskName::Subtask1, Weight::Medium)],
            )
            .await;

        let task = manager.get_task(id).await.unwrap();
        let task_for_assert = task.clone();

        println!("Spawning VTask");
        let vtask_handle = tokio::spawn(async move {
            let task_clone = task.clone();
            task_clone
                .run_subtask(&TaskName::Subtask1, |mut context| async move {
                    interruptable(&mut context, || async {
                        println!("Subtask1: Running");
                        sleep(Duration::from_millis(50)).await;
                        Ok(())
                    })
                    .await?;

                    println!("Subtask1: Completed");
                    Ok::<_, anyhow::Error>(())
                })
                .await
                .unwrap();

            println!("Completing the task");
            task_clone
                .handle_command(VTaskCommand::Complete)
                .await
                .unwrap();
        });

        println!("Waiting for task to complete");
        timeout(Duration::from_secs(5), vtask_handle)
            .await
            .expect("Test timed out")
            .expect("VTask panicked");

        println!("Checking task state");
        let task_state = task_for_assert.state.read().await;
        assert!(
            matches!(*task_state, VTaskState::Completed),
            "Task state should be Completed"
        );

        assert_eq!(
            task_for_assert.subtasks.read().await.len(),
            0,
            "All subtasks should be removed"
        );
        assert_eq!(
            *task_for_assert.total_weight.read().await,
            0,
            "Total weight should be 0"
        );

        println!("test_vtask_completion completed successfully");
        Ok(())
    }
}
