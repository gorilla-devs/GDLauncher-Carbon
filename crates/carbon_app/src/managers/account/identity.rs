//! The selected GDL identity and its local Electron publication share this gate.
use serde::Serialize;
use tokio::sync::Mutex;
#[derive(Clone, PartialEq, Serialize)]
pub(super) struct Selection {
    pub session: String,
    pub revision: u64,
    pub selected: Option<String>,
}
#[derive(Serialize)]
pub(super) struct Event {
    #[serde(flatten)]
    pub selection: Selection,
    pub email: Option<String>,
}
pub(super) struct Identity {
    pub state: Mutex<State>,
}
pub(super) struct State {
    pub selection: Selection,
    pub initialized: bool,
}
impl Default for Identity {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                selection: Selection {
                    session: std::env::var("GDL_IDENTITY_SESSION")
                        .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
                    revision: 0,
                    selected: None,
                },
                initialized: false,
            }),
        }
    }
}
impl State {
    pub fn select(&mut self, selected: Option<String>) -> Selection {
        self.selection.revision += 1;
        self.selection.selected = selected;
        self.initialized = true;
        self.selection.clone()
    }
}
impl Identity {
    pub async fn resolve_and_publish<F, Fut>(
        &self,
        selection: Selection,
        resolve: F,
        sink: impl FnOnce(Event),
    ) where
        F: FnOnce(Option<String>) -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        let email = resolve(selection.selected.clone()).await;
        self.publish(selection, email, sink).await;
    }

    pub async fn publish(
        &self,
        selection: Selection,
        email: Option<String>,
        sink: impl FnOnce(Event),
    ) {
        let state = self.state.lock().await;
        if state.selection != selection {
            return;
        }
        sink(Event { selection, email });
    }
}
pub(super) fn stdout(event: Event) {
    // A single JSON value, redacted as a whole by the Electron log boundary.
    println!(
        "_GDL_ACCOUNT_EMAIL_:{}",
        serde_json::to_string(&event).expect("identity event serializes")
    );
}
#[cfg(test)]
mod task19_identity_tests {
    use super::*;
    #[tokio::test]
    async fn stale_startup_and_save_lookup_cannot_restore_logged_out_identity() {
        for replacement in [None, Some("synthetic-b".to_owned())] {
            let identity = Identity::default();
            let old = identity
                .state
                .lock()
                .await
                .select(Some("synthetic-a".into()));
            let current = identity.state.lock().await.select(replacement);
            let events = std::sync::Mutex::new(Vec::new());
            identity
                .publish(current, None, |e| events.lock().unwrap().push(e))
                .await;
            identity
                .publish(old, Some("a@example.invalid".into()), |e| {
                    events.lock().unwrap().push(e)
                })
                .await;
            assert_eq!(
                events.lock().unwrap().len(),
                1,
                "old lookup published after logout/replacement"
            );
        }
    }
    #[tokio::test]
    async fn delayed_startup_and_save_resolution_cannot_overwrite_replacement_or_logout() {
        use std::sync::{Arc, Mutex};
        for _producer in ["startup", "save"] {
            for failure in [false, true] {
                for replacement in [false, true] {
                    let identity = Arc::new(Identity::default());
                    let events = Arc::new(Mutex::new(Vec::new()));
                    let old = identity.state.lock().await.select(Some("a".into()));
                    let (entered, ready) = tokio::sync::oneshot::channel();
                    let (release, resume) = tokio::sync::oneshot::channel();
                    let task = {
                        let identity = identity.clone();
                        let events = events.clone();
                        tokio::spawn(async move {
                            identity
                                .resolve_and_publish(
                                    old,
                                    |_| async {
                                        entered.send(()).unwrap();
                                        resume.await.unwrap();
                                        if failure {
                                            None
                                        } else {
                                            Some("a@example.invalid".into())
                                        }
                                    },
                                    |e| events.lock().unwrap().push(e),
                                )
                                .await;
                        })
                    };
                    ready.await.unwrap();
                    let current = identity.state.lock().await.select(if replacement {
                        Some("b".into())
                    } else {
                        None
                    });
                    identity
                        .publish(
                            current,
                            if replacement {
                                Some("b@example.invalid".into())
                            } else {
                                None
                            },
                            |e| events.lock().unwrap().push(e),
                        )
                        .await;
                    release.send(()).unwrap();
                    task.await.unwrap();
                    let events = events.lock().unwrap();
                    assert_eq!(events.len(), 1);
                    assert_eq!(
                        events[0].email.as_deref(),
                        if replacement {
                            Some("b@example.invalid")
                        } else {
                            None
                        }
                    );
                }
            }
        }
    }
    #[tokio::test]
    async fn final_identity_gate_rechecks_after_wait_and_current_failure_clears() {
        use std::sync::{Arc, Mutex};
        let identity = Arc::new(Identity::default());
        let old = identity.state.lock().await.select(Some("a".into()));
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut gate = identity.state.lock().await;
        let task = {
            let identity = identity.clone();
            let events = events.clone();
            tokio::spawn(async move {
                identity
                    .publish(old, Some("a@example.invalid".into()), |e| {
                        events.lock().unwrap().push(e)
                    })
                    .await
            })
        };
        let current = gate.select(Some("b".into()));
        drop(gate);
        task.await.unwrap();
        assert!(events.lock().unwrap().is_empty());
        identity
            .resolve_and_publish(
                current,
                |_| async { None },
                |e| events.lock().unwrap().push(e),
            )
            .await;
        assert!(events.lock().unwrap()[0].email.is_none());
    }
}
