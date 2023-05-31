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

pub struct GameLog {
    // buffer holding the full log
    log: String,
    lines: Vec<Option<InternalLogEntry>>,
    last_entry: usize,
    last_was_terminated: bool,
}

// Note: Option<LogEntry> shares a repr with LogEntry due to enum optimization
pub struct InternalLogEntry {
    type_: EntryType,
    start: usize,
    end: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LogEntry<'a> {
    pub type_: EntryType,
    pub start_line: usize,
    pub text: &'a str,
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
        Self {
            log: String::new(),
            lines: Vec::new(),
            last_entry: 0,
            last_was_terminated: true,
        }
    }

    /// Push new text to the log.
    ///
    /// # Note
    /// If this entry's type matches that of the last entry and
    /// the last entry did not end in \n they will be merged.
    pub fn push(&mut self, type_: EntryType, mut text: &str) {
        fn push_newlines(self_: &mut GameLog, text: &str) {
            let mut newline = false;
            let mut newline_count = 0;

            for c in text.chars() {
                newline = c == '\n';

                if newline {
                    newline_count += 1;
                }
            }

            self_.last_was_terminated = newline;

            if newline {
                self_.lines.extend((1..newline_count).map(|_| None));
            }
        }

        let mut newline = false;
        let mut newline_count = 0;

        for c in text.chars() {
            newline = c == '\n';

            if newline {
                newline_count += 1;
            }
        }

        if newline {
            text = &text[0..text.len() - 1];
        }

        if !self.last_was_terminated {
            if let Some(Some(last)) = self.lines.get_mut(self.last_entry) {
                if last.type_ == type_ {
                    if text.is_empty() {
                        return;
                    }

                    self.log.push_str(text);
                    last.end = self.log.len();

                    self.last_was_terminated = newline;

                    if newline {
                        self.lines.extend((1..newline_count).map(|_| None));
                    }

                    return;
                }
            }
        }

        let start = self.log.len();
        self.log.push_str(text);
        let end = self.log.len();

        self.last_entry = self.lines.len(); // len == last + 1
        self.lines
            .push(Some(InternalLogEntry { type_, start, end }));

        self.last_was_terminated = newline;

        if newline {
            self.lines.extend((1..newline_count).map(|_| None));
        }
    }

    /// Get the first log entry before the given line
    pub fn get_entry(&self, line: usize) -> Option<LogEntry> {
        for i in (0..=line).rev() {
            let Some(entry) = self.lines.get(i) else { return None };

            if let Some(entry) = entry {
                return Some(LogEntry {
                    type_: entry.type_,
                    start_line: i,
                    text: &self.log[entry.start..entry.end],
                });
            }
        }

        return None;
    }

    /// Get a region of log entries containing the given start and end lines
    pub fn get_region(&self, lines: impl RangeBounds<usize>) -> Vec<LogEntry> {
        let mut entries = Vec::<LogEntry>::new();

        let start = match lines.start_bound() {
            Bound::Included(&v) => v,
            Bound::Excluded(&v) => v + 1,
            Bound::Unbounded => 0,
        };

        let end = match lines.end_bound() {
            Bound::Included(&v) => v + 1,
            Bound::Excluded(&v) => v,
            Bound::Unbounded => self.lines.len(),
        };

        for i in (0..end).rev() {
            let Some(entry) = self.lines.get(i) else { continue };

            if let Some(entry) = entry {
                entries.push(LogEntry {
                    type_: entry.type_,
                    start_line: i,
                    text: &self.log[entry.start..entry.end],
                });

                if i <= start {
                    break;
                }
            }
        }

        entries.reverse();
        return entries;
    }

    pub fn len(&self) -> usize {
        self.lines.len()
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
                type_: EntryType::StdOut,
                start_line: 0,
                text: "testing"
            }),
        );
    }

    #[test]
    fn region() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing1\n");
        log.push(EntryType::StdOut, "testing2\n");
        assert_eq!(
            log.get_region(..),
            vec![
                LogEntry {
                    type_: EntryType::StdOut,
                    start_line: 0,
                    text: "testing1"
                },
                LogEntry {
                    type_: EntryType::StdOut,
                    start_line: 1,
                    text: "testing2"
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
                type_: EntryType::StdOut,
                start_line: 0,
                text: "testing1testing2"
            }),
        );

        log.push(EntryType::StdOut, "testing3");
        log.push(EntryType::StdErr, "testing4\n");
        assert_eq!(
            log.get_region(..),
            vec![
                LogEntry {
                    type_: EntryType::StdOut,
                    start_line: 0,
                    text: "testing1testing2"
                },
                LogEntry {
                    type_: EntryType::StdOut,
                    start_line: 1,
                    text: "testing3"
                },
                LogEntry {
                    type_: EntryType::StdErr,
                    start_line: 2,
                    text: "testing4"
                },
            ],
        );
    }

    #[test]
    fn multiline_entry() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing1\ntesting2\n");

        let entry = LogEntry {
            type_: EntryType::StdOut,
            start_line: 0,
            text: "testing1\ntesting2",
        };

        assert_eq!(log.get_entry(0), Some(entry));
        assert_eq!(log.get_entry(1), Some(entry));
        assert_eq!(log.get_region(..), vec![entry]);
        assert_eq!(log.get_region(..2), vec![entry]);
        assert_eq!(log.get_region(..=1), vec![entry]);
        assert_eq!(log.get_region(0..), vec![entry]);
        assert_eq!(log.get_region(0..2), vec![entry]);
        assert_eq!(log.get_region(0..=1), vec![entry]);
        assert_eq!(log.get_region(0..1), vec![entry]);
        assert_eq!(log.get_region(1..2), vec![entry]);
    }
}
