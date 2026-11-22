//! Operators of a server: the players who may run its commands, and at
//! which level.

use super::{PlayerListFile, ServerManager};
use crate::domain::server::{OpsEntry, ServerId};
use crate::managers::ManagerRef;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const OPERATOR_SAVE_POLL: Duration = Duration::from_millis(50);
/// The level a server gives when its properties name none.
const DEFAULT_OPERATOR_LEVEL: i32 = 4;

impl ManagerRef<'_, ServerManager> {
    /// Makes a player an operator.
    ///
    /// A stopped server reads the list when it starts, so the entry is
    /// written at `level`. A running one keeps its operators in memory, gives
    /// a new one the level of its `op-permission-level` property and has no
    /// command for any other, so `level` cannot be honoured: the server is
    /// only told, and this answers once it has saved the player. The list
    /// read next then shows the level the player has, not the one asked for.
    pub async fn add_operator(
        self,
        id: ServerId,
        uuid: String,
        name: String,
        level: i32,
    ) -> anyhow::Result<()> {
        let level = if self.is_server_running(id).await {
            self.send_console_if_running(id, format!("op {name}")).await;
            if self.operator_saved(id, &name).await {
                return Ok(());
            }
            // Still loading, or too busy to save in time: the entry is
            // written for it, at the level it gives.
            self.own_operator_level(id).await
        } else {
            level
        };
        let mut list = self
            .get_player_list::<OpsEntry>(id, PlayerListFile::Ops)
            .await?;
        match list.iter_mut().find(|entry| entry.uuid == uuid) {
            Some(entry) => entry.level = level,
            None => list.push(OpsEntry {
                uuid,
                name,
                level,
                bypasses_player_limit: false,
            }),
        }
        self.write_player_list(id, PlayerListFile::Ops, &list).await
    }

    /// Whether the server's own list names the player before the wait is
    /// over. A list caught half written counts as not yet.
    async fn operator_saved(self, id: ServerId, name: &str) -> bool {
        let wait = Duration::from_millis(self.server_save_wait_ms.load(Ordering::Relaxed));
        let deadline = Instant::now() + wait;
        loop {
            let saved = self
                .get_player_list::<OpsEntry>(id, PlayerListFile::Ops)
                .await
                .is_ok_and(|list| {
                    list.iter()
                        .any(|entry| entry.name.eq_ignore_ascii_case(name))
                });
            if saved {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(OPERATOR_SAVE_POLL).await;
        }
    }

    /// The level the server gives a new operator, by its properties.
    async fn own_operator_level(self, id: ServerId) -> i32 {
        self.get_server_properties(id)
            .await
            .ok()
            .and_then(|properties| properties.get("op-permission-level")?.trim().parse().ok())
            .filter(|level| (1..=4).contains(level))
            .unwrap_or(DEFAULT_OPERATOR_LEVEL)
    }
}

#[cfg(test)]
mod tests {
    use super::super::stale_marker_tests::{hold_watcher, server_fixture};
    use super::super::tests::running_server_with_handle;
    use super::*;
    use tokio::sync::mpsc;

    const NOTCH: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";

    /// Marks a fixture server as running under this session, and hands back
    /// the far end of its console.
    async fn run_fixture(app: &crate::managers::App, id: ServerId) -> mpsc::Receiver<String> {
        let (stdin_tx, stdin_rx) = mpsc::channel::<String>(8);
        let mut server = running_server_with_handle();
        server.handle.as_mut().unwrap().stdin_tx = stdin_tx;
        app.server_manager.servers.write().await.insert(id, server);
        stdin_rx
    }

    async fn ops_fixture(root: &std::path::Path, content: &str) -> std::path::PathBuf {
        let data = root.join("server");
        tokio::fs::create_dir_all(&data).await.unwrap();
        let ops = data.join(PlayerListFile::Ops.filename());
        tokio::fs::write(&ops, content).await.unwrap();
        ops
    }

    async fn ops(app: &crate::managers::App, id: ServerId) -> Vec<(String, i32)> {
        app.server_manager()
            .get_player_list::<OpsEntry>(id, PlayerListFile::Ops)
            .await
            .unwrap()
            .into_iter()
            .map(|entry| (entry.name, entry.level))
            .collect()
    }

    /// A running server keeps its operators in memory and gives a new one
    /// its own level; nothing the launcher writes changes that. The launcher
    /// only tells it, and answers once the server has saved the operator, so
    /// the list read next shows the level the player really has.
    #[tokio::test]
    async fn an_operator_added_to_a_running_server_has_the_level_the_server_saved() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "ops-running").await;
        let file = ops_fixture(&root, "[]").await;
        let mut console = run_fixture(&app, id).await;
        let server = tokio::spawn({
            let file = file.clone();
            async move {
                let command = console.recv().await.unwrap();
                let untouched = tokio::fs::read_to_string(&file).await.unwrap();
                tokio::time::sleep(Duration::from_millis(150)).await;
                let saved = serde_json::json!([{
                    "uuid": NOTCH, "name": "Notch", "level": 4, "bypassesPlayerLimit": false
                }]);
                tokio::fs::write(&file, saved.to_string()).await.unwrap();
                (command, untouched)
            }
        });

        app.server_manager()
            .add_operator(id, NOTCH.into(), "notch".into(), 1)
            .await
            .unwrap();

        assert_eq!(ops(&app, id).await, [("Notch".to_string(), 4)]);
        let (command, untouched) = server.await.unwrap();
        assert_eq!(command, "op notch");
        assert_eq!(untouched, "[]", "the launcher wrote no level of its own");
    }

    /// A server that has not saved the operator in time (it is still
    /// loading, say) gets the entry written at the level it gives by its own
    /// properties, never at the one picked.
    #[tokio::test]
    async fn a_running_server_that_saves_nothing_lists_the_operator_at_its_own_level() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        app.server_manager
            .server_save_wait_ms
            .store(150, Ordering::Relaxed);
        let (id, root) = server_fixture(&app, "ops-silent").await;
        ops_fixture(&root, "[]").await;
        let mut console = run_fixture(&app, id).await;

        app.server_manager()
            .add_operator(id, NOTCH.into(), "Notch".into(), 1)
            .await
            .unwrap();

        assert_eq!(console.recv().await.as_deref(), Some("op Notch"));
        assert_eq!(ops(&app, id).await, [("Notch".to_string(), 4)]);

        tokio::fs::write(
            root.join("server").join("server.properties"),
            "op-permission-level=2\n",
        )
        .await
        .unwrap();
        app.server_manager()
            .add_operator(
                id,
                "00000000-0000-0000-0000-000000000002".into(),
                "Alex".into(),
                1,
            )
            .await
            .unwrap();

        assert_eq!(
            ops(&app, id).await,
            [("Notch".to_string(), 4), ("Alex".to_string(), 2)]
        );
    }

    /// A stopped server reads the list when it starts: the level picked is
    /// the one written, and nothing is sent anywhere.
    #[tokio::test]
    async fn an_operator_added_to_a_stopped_server_has_the_level_picked() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "ops-stopped").await;
        app.server_manager().load_servers().await.unwrap();
        ops_fixture(&root, "[]").await;

        app.server_manager()
            .add_operator(id, NOTCH.into(), "Notch".into(), 1)
            .await
            .unwrap();

        assert_eq!(ops(&app, id).await, [("Notch".to_string(), 1)]);
    }

    /// Adding a player who is an operator already changes their level; it
    /// used to list them twice.
    #[tokio::test]
    async fn an_operator_added_again_to_a_stopped_server_changes_level() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "ops-again").await;
        app.server_manager().load_servers().await.unwrap();
        ops_fixture(&root, "[]").await;

        for level in [4, 2] {
            app.server_manager()
                .add_operator(id, NOTCH.into(), "Notch".into(), level)
                .await
                .unwrap();
        }

        assert_eq!(ops(&app, id).await, [("Notch".to_string(), 2)]);
    }
}
