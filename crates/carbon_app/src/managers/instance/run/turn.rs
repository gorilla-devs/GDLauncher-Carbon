//! The turn for work that writes what instances share.
//!
//! Installs, updates, repairs and imports write the shared libraries, assets
//! and Java, and download in bulk, so they run one at a time: each waits for
//! the one turn (`PersistenceManager::instance_download_lock`), and its
//! instance shows as queued meanwhile.
//!
//! A Play of an instance that is already installed mostly has nothing to
//! write. It verifies its files without the turn and takes it only at the
//! step that finds something to download or install ([`InstallTurn::take`]),
//! so it starts at once instead of waiting behind another instance's install.
//! The steps it runs without the turn coordinate by themselves: downloads
//! are published by rename and own their destination, and Java, native
//! libraries, loader processors and asset mappings each have their own lock.
//! A step that has such a lock takes the turn first, never the other way
//! round.
//!
//! A run that waits can be taken out of the queue by the player
//! ([`InstanceManager::leave_queue`]): nothing of it has started.

use super::{InstanceManager, LaunchState};
use crate::api::keys::instance::INSTALL_TURN;
use crate::domain::instance::InstanceId;
use crate::domain::vtask::VisualTaskId;
use crate::managers::{AppInner, ManagerRef};
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;
use tokio_util::sync::CancellationToken;

/// The player took a waiting run out of the queue. Not a failure: its task
/// is withdrawn and its instance is as it was before the run.
#[derive(Debug)]
pub struct LeftQueue;

impl std::fmt::Display for LeftQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The instance was taken out of the queue.")
    }
}

impl std::error::Error for LeftQueue {}

/// One run's hold on the turn, from [`take`](Self::take) until it is
/// released or dropped.
pub(super) struct InstallTurn {
    app: Arc<AppInner>,
    instance: InstanceId,
    task: VisualTaskId,
    /// The run's own cancellation (a quit, restart or update).
    cancel: CancellationToken,
    /// Whether the instance shows as queued now, as every run does when it
    /// starts.
    shown_queued: bool,
    held: Option<Held>,
}

/// The turn itself. Dropping it passes the turn on.
struct Held {
    app: Arc<AppInner>,
    _permit: OwnedSemaphorePermit,
}

impl Drop for Held {
    fn drop(&mut self) {
        *self
            .app
            .instance_manager()
            .persistence_manager
            .turn_holder
            .lock()
            .unwrap() = None;
        self.app.invalidate(INSTALL_TURN, None);
    }
}

/// A run's place among those the player can take out of the queue, for as
/// long as it waits.
struct Waiting<'a> {
    app: &'a Arc<AppInner>,
    instance: InstanceId,
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        self.app
            .instance_manager()
            .persistence_manager
            .turn_waiters
            .lock()
            .unwrap()
            .remove(&self.instance);
    }
}

impl InstallTurn {
    pub(super) fn new(
        app: Arc<AppInner>,
        instance: InstanceId,
        task: VisualTaskId,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            app,
            instance,
            task,
            cancel,
            shown_queued: true,
            held: None,
        }
    }

    /// The turn of a run with no instance behind it, for tests of steps that
    /// take it. It shows nothing as queued, so it must find the turn free.
    #[cfg(test)]
    pub(super) fn detached_for_test(app: Arc<AppInner>) -> Self {
        Self {
            app,
            instance: InstanceId(i32::MAX),
            task: VisualTaskId(i32::MAX),
            cancel: CancellationToken::new(),
            shown_queued: false,
            held: None,
        }
    }

    /// Shows the instance as preparing, for a run that starts without the
    /// turn.
    pub(super) async fn start_without(&mut self) -> anyhow::Result<()> {
        self.show(false).await
    }

    async fn show(&mut self, queued: bool) -> anyhow::Result<()> {
        if self.shown_queued == queued {
            return Ok(());
        }
        let state = if queued {
            LaunchState::Queued(self.task)
        } else {
            LaunchState::Preparing(self.task)
        };
        self.app
            .instance_manager()
            .change_launch_state(self.instance, self.task, state)
            .await?;
        self.shown_queued = queued;
        Ok(())
    }

    /// Takes the turn unless this run already has it, waiting for it in the
    /// queue when another run has it. The instance shows as queued while it
    /// waits and as preparing once it has the turn.
    ///
    /// Fails with [`LeftQueue`] when the player takes the run out of the
    /// queue, and when the run itself is cancelled.
    pub(super) async fn take(&mut self) -> anyhow::Result<()> {
        if self.held.is_some() {
            return Ok(());
        }
        let app = self.app.clone();
        let persistence = &app.instance_manager().persistence_manager;
        let slot = persistence.instance_download_lock.clone();
        // Never ahead of a run that already waits: the turn is handed to
        // waiters in order, and is free only when there are none.
        let permit = match slot.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                let leave = CancellationToken::new();
                persistence
                    .turn_waiters
                    .lock()
                    .unwrap()
                    .insert(self.instance, leave.clone());
                let _waiting = Waiting {
                    app: &app,
                    instance: self.instance,
                };
                self.show(true).await?;
                tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => {
                        anyhow::bail!("Runtime operation cancelled while queued")
                    }
                    _ = leave.cancelled() => return Err(LeftQueue.into()),
                    permit = slot.acquire_owned() => permit?,
                }
            }
        };
        *persistence.turn_holder.lock().unwrap() = Some(self.instance);
        self.app.invalidate(INSTALL_TURN, None);
        self.held = Some(Held {
            app: self.app.clone(),
            _permit: permit,
        });
        self.show(false).await
    }

    pub(super) fn is_held(&self) -> bool {
        self.held.is_some()
    }

    /// Passes the turn on, once nothing this run still does writes what
    /// instances share.
    pub(super) fn release(&mut self) {
        self.held = None;
    }
}

impl ManagerRef<'_, InstanceManager> {
    /// The instance whose install, update, repair or launch has the turn
    /// now, which the queued ones wait for.
    pub fn install_turn_holder(self) -> Option<InstanceId> {
        *self.persistence_manager.turn_holder.lock().unwrap()
    }

    /// [`install_turn_holder`](Self::install_turn_holder) with the instance's
    /// name, for the queued ones to show. An instance deleted since has no
    /// name to wait for.
    pub async fn install_turn_holder_named(self) -> Option<(InstanceId, String)> {
        let id = self.install_turn_holder()?;
        let (_, config) = self.current_config(id).await.ok()?;
        Some((id, config.name))
    }

    /// Takes a run of `instance_id` that waits for the turn out of the
    /// queue. Nothing of it has started, so the instance is as it was before;
    /// a setup it was to run stays pending for the next Play.
    pub fn leave_queue(self, instance_id: InstanceId) -> anyhow::Result<()> {
        match self
            .persistence_manager
            .turn_waiters
            .lock()
            .unwrap()
            .get(&instance_id)
        {
            Some(leave) => {
                leave.cancel();
                Ok(())
            }
            None => anyhow::bail!("This instance is no longer waiting in the queue."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::instance::info::{GameVersion, StandardVersion};
    use crate::managers::instance::InstanceVersionSource;
    use std::collections::HashSet;
    use std::time::Duration;

    /// An instance shown as queued under `task`, as every run starts.
    async fn queued(
        app: &crate::managers::App,
        name: &str,
        task: i32,
    ) -> (InstanceId, VisualTaskId) {
        let group = app.instance_manager().get_default_group().await.unwrap();
        let id = app
            .instance_manager()
            .create_instance(
                group,
                name.into(),
                false,
                InstanceVersionSource::Version(GameVersion::Standard(StandardVersion {
                    release: "1.7.10".into(),
                    modloaders: HashSet::new(),
                })),
                String::new(),
            )
            .await
            .unwrap();
        let task = VisualTaskId(task);
        app.instance_manager
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .data_mut()
            .unwrap()
            .state = LaunchState::Queued(task);
        (id, task)
    }

    async fn state(app: &crate::managers::App, id: InstanceId) -> &'static str {
        match app.instance_manager.instances.read().await[&id]
            .data()
            .unwrap()
            .state
        {
            LaunchState::Queued(_) => "queued",
            LaunchState::Preparing(_) => "preparing",
            _ => "other",
        }
    }

    async fn settled<F: std::future::Future<Output = bool>>(mut check: impl FnMut() -> F) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !check().await {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the queue settles");
    }

    /// A run takes a free turn at once and is named as its holder until it
    /// passes it on. Taking it twice is taking it once.
    #[tokio::test]
    async fn a_free_turn_is_taken_at_once_and_names_its_holder() {
        let app = crate::setup_managers_for_test().await;
        let (id, task) = queued(&app, "turn-free", 1).await;
        let mut turn = InstallTurn::new(app.clone(), id, task, CancellationToken::new());
        assert_eq!(app.instance_manager().install_turn_holder(), None);

        turn.take().await.unwrap();
        turn.take().await.unwrap();
        assert!(turn.is_held());
        assert_eq!(app.instance_manager().install_turn_holder(), Some(id));
        assert_eq!(state(&app, id).await, "preparing");
        // Nothing waits, so there is nothing to take out of the queue.
        app.instance_manager()
            .leave_queue(id)
            .expect_err("a run that has the turn is not waiting");

        turn.release();
        assert!(!turn.is_held());
        assert_eq!(app.instance_manager().install_turn_holder(), None);
    }

    /// A run that began without the turn shows as preparing, queues only
    /// when it asks for the turn while another run has it, and goes on once
    /// that run passes it on.
    #[tokio::test]
    async fn a_run_that_began_without_the_turn_queues_only_when_it_needs_it() {
        let app = crate::setup_managers_for_test().await;
        let (first, first_task) = queued(&app, "turn-holder", 1).await;
        let (second, second_task) = queued(&app, "turn-later", 2).await;
        let mut holder = InstallTurn::new(app.clone(), first, first_task, CancellationToken::new());
        holder.take().await.unwrap();

        let mut later =
            InstallTurn::new(app.clone(), second, second_task, CancellationToken::new());
        later.start_without().await.unwrap();
        assert_eq!(state(&app, second).await, "preparing");
        assert!(!later.is_held());
        assert_eq!(app.instance_manager().install_turn_holder(), Some(first));

        let waiting = app.clone();
        let wait = tokio::spawn(async move {
            later.take().await.unwrap();
            later
        });
        settled(|| async { state(&waiting, second).await == "queued" }).await;
        assert_eq!(app.instance_manager().install_turn_holder(), Some(first));

        drop(holder);
        let later = tokio::time::timeout(Duration::from_secs(5), wait)
            .await
            .expect("the turn is passed on")
            .unwrap();
        assert!(later.is_held());
        assert_eq!(state(&app, second).await, "preparing");
        assert_eq!(app.instance_manager().install_turn_holder(), Some(second));
    }

    /// The player takes a waiting run out of the queue: it ends with
    /// `LeftQueue`, never gets the turn, and the run that has the turn is
    /// untouched. A run's own cancellation is not a leave.
    #[tokio::test]
    async fn a_waiting_run_leaves_the_queue_when_asked() {
        let app = crate::setup_managers_for_test().await;
        let (first, first_task) = queued(&app, "turn-kept", 1).await;
        let (second, second_task) = queued(&app, "turn-left", 2).await;
        let (third, third_task) = queued(&app, "turn-cancelled", 3).await;
        let mut holder = InstallTurn::new(app.clone(), first, first_task, CancellationToken::new());
        holder.take().await.unwrap();
        app.instance_manager()
            .leave_queue(second)
            .expect_err("nothing waits yet");

        let mut leaving =
            InstallTurn::new(app.clone(), second, second_task, CancellationToken::new());
        let left = tokio::spawn(async move { leaving.take().await });
        let cancel = CancellationToken::new();
        let mut cancelled = InstallTurn::new(app.clone(), third, third_task, cancel.clone());
        let stopped = tokio::spawn(async move { cancelled.take().await });
        settled(|| async {
            app.instance_manager()
                .persistence_manager
                .turn_waiters
                .lock()
                .unwrap()
                .len()
                == 2
        })
        .await;

        app.instance_manager().leave_queue(second).unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), left)
            .await
            .expect("leaving ends the wait")
            .unwrap()
            .expect_err("it left the queue");
        assert!(error.is::<LeftQueue>(), "{error:#}");
        cancel.cancel();
        let error = tokio::time::timeout(Duration::from_secs(5), stopped)
            .await
            .expect("cancelling ends the wait")
            .unwrap()
            .expect_err("it was cancelled");
        assert!(!error.is::<LeftQueue>(), "{error:#}");

        assert!(
            app.instance_manager()
                .persistence_manager
                .turn_waiters
                .lock()
                .unwrap()
                .is_empty()
        );
        app.instance_manager()
            .leave_queue(second)
            .expect_err("it no longer waits");
        assert!(holder.is_held());
        assert_eq!(app.instance_manager().install_turn_holder(), Some(first));
    }
}
