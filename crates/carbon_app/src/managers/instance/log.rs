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
    use super::{EntryType, GameLog, LogEntry};

    #[test]
    fn push() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing\n");
        assert_eq!(
            log.get_entry(0),
            Some(LogEntry {
                kind: EntryType::StdOut,
                entry_number: 0,
                data: "testing"
            }),
        );
    }

    #[test]
    fn region() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing1\n");
        log.push(EntryType::StdOut, "testing2\n");
        assert_eq!(
            log.get_span(..),
            vec![
                LogEntry {
                    kind: EntryType::StdOut,
                    entry_number: 0,
                    data: "testing1"
                },
                LogEntry {
                    kind: EntryType::StdOut,
                    entry_number: 1,
                    data: "testing2"
                },
            ],
        );
    }

    #[test]
    fn line_merging() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing1");
        log.push(EntryType::StdOut, "testing2\n");
        assert_eq!(
            log.get_entry(0),
            Some(LogEntry {
                kind: EntryType::StdOut,
                entry_number: 0,
                data: "testing1testing2"
            }),
        );

        log.push(EntryType::StdOut, "testing3");
        log.push(EntryType::StdErr, "testing4\n");
        assert_eq!(
            log.get_span(..),
            vec![
                LogEntry {
                    kind: EntryType::StdOut,
                    entry_number: 0,
                    data: "testing1testing2"
                },
                LogEntry {
                    kind: EntryType::StdOut,
                    entry_number: 1,
                    data: "testing3"
                },
                LogEntry {
                    kind: EntryType::StdErr,
                    entry_number: 2,
                    data: "testing4"
                },
            ],
        );
    }

    #[test]
    fn multiline_entry() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing1\ntesting2\n");

        let entry = LogEntry {
            kind: EntryType::StdOut,
            entry_number: 0,
            data: "testing1\ntesting2",
        };

        assert_eq!(log.get_entry(0), Some(entry));
        assert_eq!(log.get_entry(1), Some(entry));
        assert_eq!(log.get_span(..), vec![entry]);
        assert_eq!(log.get_span(..2), vec![entry]);
        assert_eq!(log.get_span(..=1), vec![entry]);
        assert_eq!(log.get_span(0..), vec![entry]);
        assert_eq!(log.get_span(0..2), vec![entry]);
        assert_eq!(log.get_span(0..=1), vec![entry]);
        assert_eq!(log.get_span(0..1), vec![entry]);
        assert_eq!(log.get_span(1..2), vec![entry]);
    }
}
