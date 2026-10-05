//! Whole-operation deletion producer shared by single and group deletion.
use crate::managers::runtime_lifecycle::{ActivityPolicy, RuntimeLifecycle, spawn_retained};
use std::{future::Future, sync::Arc};

pub(super) fn spawn_deletion<T: Send + 'static>(
    lifecycle: &Arc<RuntimeLifecycle>,
    owner: String,
    work: impl Future<Output = T> + Send + 'static,
) -> anyhow::Result<tokio::task::JoinHandle<T>> {
    // Admission is synchronous: no producer can acknowledge or yield first.
    // The shared supervisor finishes only after the entire worker returns;
    // losing a requester does not cancel entered removal or error recovery.
    let activity = lifecycle.admit_owned(owner, ActivityPolicy::Deletion)?;
    Ok(spawn_retained(activity, move |_| Box::pin(work)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::runtime_lifecycle::{ShutdownPhase, ShutdownPurpose};

    fn ready() -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }

    #[tokio::test]
    async fn deletion_freeze_refuses_before_worker_enters() {
        let lifecycle = ready();
        lifecycle.begin_relocation("frozen").unwrap();
        let entered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_entered = entered.clone();
        let result = spawn_deletion(&lifecycle, "single deletion".into(), async move {
            worker_entered.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        let refused = result.is_err();
        if let Ok(worker) = result {
            worker.await.unwrap();
        }
        assert!(
            refused,
            "frozen deletion must refuse synchronously before spawn"
        );
        assert!(!entered.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn deletion_single_waiter_drop_retains_entered_removal_and_recovery() {
        for fail in [false, true] {
            let lifecycle = ready();
            let root = tempfile::tempdir().unwrap();
            let selected = root.path().join("selected");
            let retired = root.path().join("retired-or-recovered");
            std::fs::write(&selected, b"selected fixture").unwrap();
            let (entered, arrived) = tokio::sync::oneshot::channel();
            let (release, released) = tokio::sync::oneshot::channel();
            let (done, finished) = tokio::sync::oneshot::channel();
            let worker_retired = retired.clone();
            let worker = spawn_deletion(&lifecycle, "single deletion".into(), async move {
                tokio::fs::remove_file(selected).await.unwrap();
                entered.send(()).unwrap();
                released.await.unwrap();
                // The producer includes the terminal retirement or error recovery,
                // rather than acknowledging only completion of removal.
                tokio::fs::write(
                    worker_retired,
                    if fail {
                        b"recovered".as_slice()
                    } else {
                        b"retired"
                    },
                )
                .await
                .unwrap();
                done.send(()).unwrap();
                if fail {
                    anyhow::bail!("controlled retirement error, recovery complete");
                }
                Ok(())
            })
            .unwrap();
            arrived.await.unwrap();
            drop(worker); // RPC requester disappears; the producer remains owned.
            let relocation_blocked = lifecycle.begin_relocation("during-removal").is_err();
            if !relocation_blocked {
                lifecycle.cancel_relocation("during-removal").unwrap();
            }
            lifecycle
                .begin_shutdown("quit", ShutdownPurpose::Quit)
                .unwrap();
            let before = lifecycle.settle_shutdown("quit", Ok(())).unwrap();
            assert!(!retired.exists(), "terminal phase has not yet run");
            release.send(()).unwrap();
            finished.await.unwrap();
            assert!(relocation_blocked, "entered deletion must block relocation");
            assert_eq!(
                before.phase,
                ShutdownPhase::Unresolved,
                "entered deletion must block shutdown"
            );
            assert_eq!(
                std::fs::read(retired).unwrap(),
                if fail {
                    b"recovered".as_slice()
                } else {
                    b"retired"
                }
            );
            // Retry through the production watch-based settlement path. The
            // injected server drain owns no external work or native process.
            tokio::time::timeout(
                std::time::Duration::from_secs(2),
                lifecycle
                    .prepare_shutdown_and_commit("quit", ShutdownPurpose::Quit, async { Ok(()) }),
            )
            .await
            .expect("controlled deletion settlement")
            .unwrap();
            assert_eq!(
                lifecycle
                    .shutdown_status("quit", ShutdownPurpose::Quit)
                    .unwrap()
                    .phase,
                ShutdownPhase::Committed
            );
        }
    }

    #[tokio::test]
    async fn deletion_group_inherits_one_owner_after_freeze_until_last_child() {
        let lifecycle = ready();
        let root = tempfile::tempdir().unwrap();
        let child1 = root.path().join("child-one");
        let child2 = root.path().join("child-two");
        std::fs::write(&child1, b"one").unwrap();
        std::fs::write(&child2, b"two").unwrap();
        let retained_child = child2.clone();
        let (entered, arrived) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let worker = spawn_deletion(&lifecycle, "group deletion".into(), async move {
            tokio::fs::remove_file(child1).await.unwrap();
            entered.send(()).unwrap();
            released.await.unwrap();
            // Already admitted selected children must settle even after freeze.
            tokio::fs::remove_file(child2).await.unwrap();
            Ok::<_, anyhow::Error>(())
        })
        .unwrap();
        arrived.await.unwrap();
        lifecycle
            .begin_shutdown("group", ShutdownPurpose::Quit)
            .unwrap();
        let before = lifecycle.settle_shutdown("group", Ok(())).unwrap();
        assert!(retained_child.exists());
        release.send(()).unwrap();
        worker.await.unwrap().unwrap();
        assert_eq!(
            before.phase,
            ShutdownPhase::Unresolved,
            "all selected children share unsettled authority"
        );
        assert!(!retained_child.exists());
        lifecycle
            .begin_shutdown("group", ShutdownPurpose::Quit)
            .unwrap();
        assert_eq!(
            lifecycle.settle_shutdown("group", Ok(())).unwrap().phase,
            ShutdownPhase::Ready
        );
        lifecycle
            .commit_prepared_shutdown("group", ShutdownPurpose::Quit)
            .unwrap();
    }

    /// A deletion starts no program, so a panic that unwinds its worker
    /// leaves nothing of it running: the panic is kept for its caller, but
    /// it does not hold up every later quit or refuse every move.
    #[tokio::test]
    async fn deletion_worker_panic_is_settled_and_kept() {
        let lifecycle = ready();
        let worker = spawn_deletion(&lifecycle, "panicked deletion".into(), async {
            panic!("controlled deletion panic");
        })
        .unwrap();
        assert!(worker.await.unwrap_err().is_panic());
        lifecycle.begin_relocation("move").unwrap();
        lifecycle.cancel_relocation("move").unwrap();
        lifecycle
            .begin_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
        assert_eq!(
            lifecycle.settle_shutdown("quit", Ok(())).unwrap().phase,
            ShutdownPhase::Ready
        );
    }
}
