use std::{
    ops::{Bound, RangeBounds},
    sync::atomic::{AtomicI32, Ordering},
};

use thiserror::Error;
use tokio::sync::watch;

use crate::{api::keys::instance::*, domain::instance::GameLogEntry};
use crate::{
    domain::instance::{GameLogId, InstanceId},
    managers::ManagerRef,
};

use super::InstanceManager;

#[derive(Debug, Default)]
pub struct GameLog(Vec<LogEntry>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub kind: EntryType,
    pub data: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EntryType {
    System,
    StdOut,
    StdErr,
    // more entries once log levels are handled
}

impl GameLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new line into the log.
    pub fn add_new_line(&mut self, kind: EntryType, line: impl ToString) {
        self.0.push(LogEntry {
            kind,
            data: line.to_string(),
        })
    }

    /// Retrieves the requested line from the log.
    pub fn get_line(&self, line: usize) -> Option<&LogEntry> {
        self.0.get(line)
    }

    /// Get a region of log entries containing the given start and end lines
    /// Truncates the range if it is out of bounds.
    pub fn get_span(&self, lines: impl RangeBounds<usize>) -> &[LogEntry] {
        let start = match lines.start_bound() {
            Bound::Included(s) => *s,
            Bound::Unbounded => 0,
            Bound::Excluded(_) => unreachable!("start bounds are never excluded"),
        };

        let end = match lines.end_bound() {
            Bound::Included(e) if *e <= self.0.len() => *e + 1, // normalize to excluded
            Bound::Excluded(e) if *e < self.0.len() => *e,
            _ => self.0.len(),
        };

        if start >= end {
            return Default::default();
        }

        &self.0[start..end]
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl ManagerRef<'_, InstanceManager> {
    pub async fn create_log(self, instance_id: InstanceId) -> (GameLogId, watch::Sender<GameLog>) {
        static LOG_ID: AtomicI32 = AtomicI32::new(0);
        let (log_tx, log_rx) = watch::channel(GameLog::new());
        let id = GameLogId(LOG_ID.fetch_add(1, Ordering::Relaxed));
        self.game_logs
            .write()
            .await
            .insert(id, (instance_id, log_rx));
        self.app.invalidate(GET_LOGS, None);

        (id, log_tx)
    }

    pub async fn delete_log(self, id: GameLogId) -> anyhow::Result<()> {
        let mut logs = self.game_logs.write().await;

        match logs.get(&id) {
            Some((_, rx)) => {
                // sender dropped
                match rx.has_changed() {
                    Ok(_) => Err(anyhow::anyhow!("cannot delete active log")),
                    Err(_) => {
                        let _ = logs.remove(&id);
                        self.app.invalidate(GET_LOGS, None);
                        Ok(())
                    }
                }
            }
            None => Err(anyhow::anyhow!(InvalidGameLogIdError)),
        }
    }

    pub async fn get_log(
        self,
        id: GameLogId,
    ) -> Result<watch::Receiver<GameLog>, InvalidGameLogIdError> {
        match self.game_logs.read().await.get(&id) {
            Some((_, log)) => Ok(log.clone()),
            None => Err(InvalidGameLogIdError),
        }
    }

    pub async fn get_logs(self) -> Vec<GameLogEntry> {
        self.game_logs
            .read()
            .await
            .iter()
            .map(|(id, (instance_id, rx))| GameLogEntry {
                id: *id,
                instance_id: *instance_id,
                active: rx.has_changed().is_ok(),
            })
            .collect()
    }
}

#[derive(Error, Debug)]
#[error("log id does not refer to a valid game log")]
pub struct InvalidGameLogIdError;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn span() {
        let mut log = GameLog::new();

        log.add_new_line(EntryType::StdOut, "item 1");
        log.add_new_line(EntryType::StdOut, "item 2");
        log.add_new_line(EntryType::StdOut, "item 3");
        log.add_new_line(EntryType::StdOut, "item 4");

        // Test each kind of range

        #[track_caller]
        fn test_span<R, const N: usize>(log: &GameLog, range: R, expected: [&str; N])
        where
            R: std::ops::RangeBounds<usize>,
        {
            let span = log
                .get_span(range)
                .iter()
                .map(|entry| &entry.data)
                .collect::<Vec<_>>();

            assert_eq!(span, expected);
        }

        // ..
        test_span(&log, .., ["item 1", "item 2", "item 3", "item 4"]);

        // a..
        test_span(&log, 1.., ["item 2", "item 3", "item 4"]);
        test_span(&log, 3.., ["item 4"]);
        test_span(&log, 5.., []);

        //  ..b
        test_span(&log, ..5, ["item 1", "item 2", "item 3", "item 4"]);
        test_span(&log, ..=3, ["item 1", "item 2", "item 3", "item 4"]);
        test_span(&log, ..3, ["item 1", "item 2", "item 3"]);
        test_span(&log, ..0, []);

        // a..b
        test_span(&log, 1..1, []);
        test_span(&log, 1..0, []);
        test_span(&log, 1..2, ["item 2"]);
        test_span(&log, 1..=3, ["item 2", "item 3", "item 4"]);
    }
}
