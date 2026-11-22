//! Whether a server lets in only the players on its whitelist.

use super::{ServerManager, properties};
use crate::domain::server::ServerId;
use crate::managers::ManagerRef;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const WHITELIST_PROPERTY: &str = "white-list";
const SERVER_SAVE_POLL: Duration = Duration::from_millis(50);

impl ManagerRef<'_, ServerManager> {
    /// Makes the whitelist count, or stop counting.
    ///
    /// The list itself changes nothing until the server's `white-list`
    /// property is on. A stopped server reads the property when it starts.
    /// A running one is told on its console, so no restart is needed; it
    /// then saves its properties as it holds them in memory, which would
    /// drop what was changed in the file since it started. Those changes are
    /// put back, with the property as asked.
    pub async fn set_whitelist_enabled(self, id: ServerId, enabled: bool) -> anyhow::Result<()> {
        let mut updates = HashMap::new();
        if self.is_server_running(id).await {
            let before = self.properties_text(id).await;
            let command = if enabled {
                "whitelist on"
            } else {
                "whitelist off"
            };
            self.send_console_if_running(id, command.into()).await;
            if let Some(after) = self.properties_saved_by_server(id, &before).await {
                let saved = properties::parse_properties(&after);
                updates.extend(
                    properties::parse_properties(&before)
                        .into_iter()
                        .filter(|(key, value)| saved.get(key) != Some(value)),
                );
            }
        }
        updates.insert(WHITELIST_PROPERTY.to_owned(), enabled.to_string());
        self.update_server_properties(id, updates).await
    }

    async fn properties_text(self, id: ServerId) -> String {
        let Ok(path) = self.server_properties_path(id).await else {
            return String::new();
        };
        tokio::fs::read(path)
            .await
            .map(|bytes| properties::decode(bytes).0)
            .unwrap_or_default()
    }

    /// The properties file once the server has written it again, or nothing
    /// when it has not in time (it was told what it already does, or it is
    /// still loading).
    async fn properties_saved_by_server(self, id: ServerId, before: &str) -> Option<String> {
        let wait = Duration::from_millis(self.server_save_wait_ms.load(Ordering::Relaxed));
        let deadline = Instant::now() + wait;
        loop {
            let now = self.properties_text(id).await;
            // A file caught half written is not empty and not the old one;
            // it is taken once two reads in a row agree.
            if !now.is_empty() && now != before {
                tokio::time::sleep(SERVER_SAVE_POLL).await;
                if self.properties_text(id).await == now {
                    return Some(now);
                }
            }
            if Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(SERVER_SAVE_POLL).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::stale_marker_tests::{hold_watcher, server_fixture};
    use super::super::tests::running_server_with_handle;
    use super::*;
    use tokio::sync::mpsc;

    async fn run_fixture(app: &crate::managers::App, id: ServerId) -> mpsc::Receiver<String> {
        let (stdin_tx, stdin_rx) = mpsc::channel::<String>(8);
        let mut server = running_server_with_handle();
        server.handle.as_mut().unwrap().stdin_tx = stdin_tx;
        app.server_manager.servers.write().await.insert(id, server);
        stdin_rx
    }

    async fn properties_fixture(root: &std::path::Path, content: &str) -> std::path::PathBuf {
        let data = root.join("server");
        tokio::fs::create_dir_all(&data).await.unwrap();
        let file = data.join("server.properties");
        tokio::fs::write(&file, content).await.unwrap();
        file
    }

    async fn read(app: &crate::managers::App, id: ServerId) -> Vec<(String, String)> {
        app.server_manager()
            .get_server_properties(id)
            .await
            .unwrap()
            .into_iter()
            .collect()
    }

    fn pairs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    /// A stopped server reads its properties when it starts: the property
    /// is written, the others are left, and nothing is sent anywhere.
    #[tokio::test]
    async fn a_stopped_server_gets_the_property_written() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "whitelist-stopped").await;
        app.server_manager().load_servers().await.unwrap();
        properties_fixture(&root, "motd=Mine\nwhite-list=false\n").await;

        app.server_manager()
            .set_whitelist_enabled(id, true)
            .await
            .unwrap();
        assert_eq!(
            read(&app, id).await,
            pairs(&[("motd", "Mine"), ("white-list", "true")])
        );

        app.server_manager()
            .set_whitelist_enabled(id, false)
            .await
            .unwrap();
        assert_eq!(
            read(&app, id).await,
            pairs(&[("motd", "Mine"), ("white-list", "false")])
        );
    }

    /// A running server is told, and saves its properties as it holds them
    /// in memory. What was changed in the file since it started (here the
    /// description) is still there afterwards.
    #[tokio::test]
    async fn a_running_server_is_told_and_keeps_what_was_changed_in_its_file() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "whitelist-running").await;
        let file = properties_fixture(&root, "motd=Changed while running\nmax-players=20\n").await;
        let mut console = run_fixture(&app, id).await;
        let server = tokio::spawn(async move {
            let command = console.recv().await.unwrap();
            tokio::time::sleep(Duration::from_millis(120)).await;
            let memory =
                "#Minecraft server properties\nmax-players=20\nmotd=As started\nwhite-list=true\n";
            tokio::fs::write(&file, memory).await.unwrap();
            command
        });

        app.server_manager()
            .set_whitelist_enabled(id, true)
            .await
            .unwrap();

        assert_eq!(server.await.unwrap(), "whitelist on");
        assert_eq!(
            read(&app, id).await,
            pairs(&[
                ("max-players", "20"),
                ("motd", "Changed while running"),
                ("white-list", "true")
            ])
        );
    }

    /// A server that saves nothing in time (it already does what it was
    /// told, or it is still loading) gets the property written for it.
    #[tokio::test]
    async fn a_running_server_that_saves_nothing_gets_the_property_written() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        app.server_manager
            .server_save_wait_ms
            .store(150, Ordering::Relaxed);
        let (id, root) = server_fixture(&app, "whitelist-silent").await;
        properties_fixture(&root, "motd=Mine\nwhite-list=true\n").await;
        let mut console = run_fixture(&app, id).await;

        app.server_manager()
            .set_whitelist_enabled(id, false)
            .await
            .unwrap();

        assert_eq!(console.recv().await.as_deref(), Some("whitelist off"));
        assert_eq!(
            read(&app, id).await,
            pairs(&[("motd", "Mine"), ("white-list", "false")])
        );
    }
}
