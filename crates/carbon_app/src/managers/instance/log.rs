use carbon_parsing::log::{LogParser, ParsedItem};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use itertools::Itertools;
use serde::Serialize;
use std::{
    borrow::Cow,
    ops::{Bound, RangeBounds},
    sync::atomic::{AtomicI32, Ordering},
    time::Instant,
};
use unicode_segmentation::UnicodeSegmentation;

use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::watch,
};

use crate::{api::keys::instance::*, domain::instance::GameLogEntry};
use crate::{
    domain::instance::{GameLogId, InstanceId},
    managers::ManagerRef,
};

use super::InstanceManager;

/// Hard caps on in-memory log entries per game session. A log-spamming game would
/// otherwise grow this without bound; past either cap, entries are dropped after a
/// single notice (indices never shift, so the streaming API stays consistent).
/// The byte budget covers games that spam few but enormous lines, which the entry
/// count alone would not bound.
const MAX_LOG_ENTRIES: usize = 250_000;
const MAX_LOG_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct GameLog {
    entries: Vec<LogEntry>,
    message_bytes: usize,
    truncated: bool,
}

impl GameLog {
    pub fn search(
        &self,
        query: &str,
        match_case: bool,
        match_whole_word: bool,
        use_regex: bool,
    ) -> Vec<SearchResult> {
        let mut results = Vec::new();

        let regex = if use_regex {
            match regex::Regex::new(query) {
                Ok(r) => Some(r),
                Err(_) => return vec![],
            }
        } else {
            None
        };

        for (entry_index, entry) in self.entries.iter().enumerate() {
            let message: Cow<str> = if match_case {
                entry.message.as_str().into()
            } else {
                entry.message.to_lowercase().into()
            };
            let search_query = if match_case {
                query.to_string()
            } else {
                query.to_lowercase()
            };

            let message_graphemes: Vec<&str> = message.graphemes(true).collect();
            let query_graphemes: Vec<&str> = search_query.graphemes(true).collect();

            if use_regex {
                if let Some(regex) = &regex {
                    for mat in regex.find_iter(&message) {
                        let start_grapheme_pos = message[..mat.start()].graphemes(true).count();
                        let end_grapheme_pos = message[..mat.end()].graphemes(true).count();
                        results.push(SearchResult {
                            entry_index,
                            pos: start_grapheme_pos,
                            len: end_grapheme_pos - start_grapheme_pos,
                        });
                    }
                }
            } else if match_whole_word {
                let mut pos = 0;
                while pos < message_graphemes.len() {
                    if message_graphemes[pos..].starts_with(&query_graphemes) {
                        // Check word boundaries
                        let is_word_start = pos == 0
                            || !message_graphemes[pos - 1]
                                .chars()
                                .next()
                                .unwrap()
                                .is_alphanumeric();
                        let query_end = pos + query_graphemes.len();
                        let is_word_end = query_end >= message_graphemes.len()
                            || !message_graphemes[query_end]
                                .chars()
                                .next()
                                .unwrap()
                                .is_alphanumeric();

                        if is_word_start && is_word_end {
                            results.push(SearchResult {
                                entry_index,
                                pos,
                                len: query_graphemes.len(),
                            });
                        }
                    }
                    pos += 1;
                }
            } else {
                let mut pos = 0;
                while pos < message_graphemes.len() {
                    if message_graphemes[pos..].starts_with(&query_graphemes) {
                        results.push(SearchResult {
                            entry_index,
                            pos,
                            len: query_graphemes.len(),
                        });
                    }
                    pos += 1;
                }
            }
        }

        results
    }
}

/// Represents a log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// The source of the log entry.
    pub source_kind: LogEntrySourceKind,
    /// The name of the logger that emitted this entry.
    pub logger: String,
    /// The timestamp the entry was created.
    pub timestamp: u64,
    /// The name of the thread that created the entry.
    pub thread: String,
    /// The verbosity level of the entry.
    pub level: LogEntryLevel,
    /// The entry message itself.
    pub message: String,
}

impl From<(LogEntrySourceKind, carbon_parsing::log::LogEntry)> for LogEntry {
    fn from((source_kind, entry): (LogEntrySourceKind, carbon_parsing::log::LogEntry)) -> Self {
        let carbon_parsing::log::LogEntry {
            logger,
            level,
            timestamp,
            thread_name,
            message,
        } = entry;

        Self {
            source_kind,
            logger: logger.to_owned(),
            timestamp,
            thread: thread_name.to_owned(),
            level: level.into(),
            message: message.to_owned(),
        }
    }
}

impl LogEntry {
    /// Create a new system message.
    pub fn system_message(msg: impl ToString) -> Self {
        Self {
            source_kind: LogEntrySourceKind::System,
            logger: "GDLauncher".into(),
            timestamp: chrono::Local::now().timestamp_millis() as u64,
            thread: "N/A".into(),
            level: LogEntryLevel::Info,
            message: msg.to_string(),
        }
    }

    pub fn plaintext(msg: impl ToString, source_kind: LogEntrySourceKind) -> Self {
        Self {
            source_kind,
            logger: "N/A".into(),
            timestamp: chrono::Local::now().timestamp_millis() as u64,
            thread: "N/A".into(),
            level: LogEntryLevel::Info,
            message: msg.to_string(),
        }
    }

    /// Create a new system message with an `error` level.
    pub fn system_error(msg: impl ToString) -> Self {
        let mut this = Self::system_message(msg);

        this.level = LogEntryLevel::Error;

        this
    }
}

/// The level of the log entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LogEntryLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<carbon_parsing::log::LogEntryLevel> for LogEntryLevel {
    fn from(level: carbon_parsing::log::LogEntryLevel) -> Self {
        use carbon_parsing::log::LogEntryLevel as LogEntryLevel_;

        match level {
            LogEntryLevel_::Trace => Self::Trace,
            LogEntryLevel_::Debug => Self::Debug,
            LogEntryLevel_::Info => Self::Info,
            LogEntryLevel_::Warn => Self::Warn,
            LogEntryLevel_::Error => Self::Error,
            LogEntryLevel_::Fatal => Self::Error,
            LogEntryLevel_::Unknown => Self::Error,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum LogEntrySourceKind {
    System,
    StdOut,
    StdErr,
}

impl GameLog {
    /// Creates a new game log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new entry into the log.
    pub fn add_entry(&mut self, entry: LogEntry) {
        let _ = self.try_add_entry(entry);
    }

    /// Inserts a new entry into the log, returning whether the log was modified.
    /// Returns `false` once the entry or byte cap has been reached.
    pub fn try_add_entry(&mut self, entry: LogEntry) -> bool {
        if self.truncated {
            return false;
        }

        if self.entries.len() >= MAX_LOG_ENTRIES || self.message_bytes >= MAX_LOG_BYTES {
            self.truncated = true;
            self.entries.push(LogEntry::system_error(
                "Log exceeded the in-memory limit, further output will not be shown",
            ));
            return true;
        }

        self.message_bytes += entry.message.len();
        self.entries.push(entry);
        true
    }

    /// Retrieves the requested entry from the log.
    pub fn get_entry(&self, line: usize) -> Option<&LogEntry> {
        self.entries.get(line)
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
            Bound::Included(e) if *e <= self.entries.len() => *e + 1, // normalize to excluded
            Bound::Excluded(e) if *e < self.entries.len() => *e,
            _ => self.entries.len(),
        };

        if start >= end {
            return Default::default();
        }

        &self.entries[start..end]
    }

    /// Get the number of entries contained in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug)]
pub struct SearchResult {
    pub entry_index: usize,
    pub pos: usize,
    pub len: usize,
}

static LOG_ID: AtomicI32 = AtomicI32::new(0);
impl ManagerRef<'_, InstanceManager> {
    pub async fn create_log(
        self,
        instance_id: InstanceId,
        datetime: Option<DateTime<Local>>,
    ) -> (GameLogId, watch::Sender<GameLog>) {
        let (log_tx, log_rx) = watch::channel(GameLog::new());
        let id = GameLogId(LOG_ID.fetch_add(1, Ordering::Relaxed));

        let current_datetime = datetime.unwrap_or_else(chrono::Local::now);

        self.game_logs
            .write()
            .await
            .insert(id, (instance_id, current_datetime, log_rx));
        self.app.invalidate(GET_LOGS, None);

        (id, log_tx)
    }

    pub async fn delete_log(self, id: GameLogId) -> anyhow::Result<()> {
        let mut logs = self.game_logs.write().await;

        match logs.get(&id) {
            Some((_, _, rx)) => {
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

    pub async fn get_log_file_path(
        self,
        instance_id: InstanceId,
        log_id: GameLogId,
    ) -> anyhow::Result<String> {
        // Get the log entry to find the datetime
        let logs = self.game_logs.read().await;
        let (_, datetime, _) = logs
            .get(&log_id)
            .ok_or_else(|| anyhow::anyhow!(InvalidGameLogIdError))?;
        let datetime = datetime.clone();
        drop(logs);

        // Get the instance shortpath
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or_else(|| anyhow::anyhow!("Instance not found"))?;
        let shortpath = instance.shortpath.clone();
        drop(instances);

        // Construct the log file path
        let logs_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_gdl_logs_path();

        let filename = format!("{}.log", datetime.format("%Y-%m-%d_%H-%M-%S"));
        let log_file_path = logs_path.join(&filename);

        // Return the file path if it exists, otherwise return the logs folder
        if log_file_path.exists() {
            Ok(log_file_path.to_string_lossy().to_string())
        } else {
            Ok(logs_path.to_string_lossy().to_string())
        }
    }

    pub async fn get_log(
        self,
        id: GameLogId,
    ) -> Result<watch::Receiver<GameLog>, InvalidGameLogIdError> {
        match self.game_logs.read().await.get(&id) {
            Some((_, _, log)) => Ok(log.clone()),
            None => Err(InvalidGameLogIdError),
        }
    }

    pub async fn get_logs(self, instance_id: InstanceId) -> Vec<GameLogEntry> {
        async fn read_logs_from_memory(
            itself: ManagerRef<'_, InstanceManager>,
            instance_id: InstanceId,
            logs_path: Option<std::path::PathBuf>,
        ) -> Vec<GameLogEntry> {
            itself
                .game_logs
                .read()
                .await
                .iter()
                .filter(|(_, (id, _, _))| *id == instance_id)
                .map(|(id, (instance_id, datetime, rx))| {
                    let active = rx.has_changed().is_ok();
                    // For inactive logs, compute file size from the log file
                    let file_size = if active {
                        None
                    } else if let Some(ref logs_path) = logs_path {
                        let filename = format!("{}.log", datetime.format("%Y-%m-%d_%H-%M-%S"));
                        let file_path = logs_path.join(&filename);
                        std::fs::metadata(&file_path).ok().map(|m| m.len())
                    } else {
                        None
                    };
                    GameLogEntry {
                        id: *id,
                        instance_id: *instance_id,
                        active,
                        datetime: datetime.clone(),
                        file_size,
                    }
                })
                .sorted_by_key(|entry| entry.id.0)
                .collect()
        }

        // Get instance logs path for file size computation
        let logs_path = {
            let instance_lock = self.instances.read().await;
            instance_lock.get(&instance_id).map(|v| {
                self.app
                    .settings_manager()
                    .runtime_path
                    .get_instances()
                    .get_instance_path(&v.shortpath)
                    .get_gdl_logs_path()
            })
        };

        let in_memory_logs =
            read_logs_from_memory(self.clone(), instance_id, logs_path.clone()).await;

        if in_memory_logs.len() == 0 {
            let instance_lock = self.instances.read().await;
            let Some(shortpath) = instance_lock.get(&instance_id).map(|v| v.shortpath.clone())
            else {
                tracing::error!("instance id {instance_id} not found in instances");
                return in_memory_logs;
            };
            drop(instance_lock);

            let instance_logs_path = self
                .app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&shortpath)
                .get_gdl_logs_path();

            if instance_logs_path.exists() {
                let Ok(instance_logs_path) = instance_logs_path.read_dir() else {
                    tracing::error!("Failed to read instance logs directory");
                    return in_memory_logs;
                };

                for entry in instance_logs_path {
                    let Ok(entry) = entry else {
                        tracing::error!("Failed to read log file entry");
                        continue;
                    };
                    let file_name = entry.file_name();
                    let file_name = file_name.to_string_lossy();

                    if file_name.ends_with(".log") {
                        let file_datetime_str = file_name
                            .strip_suffix(".log")
                            .expect("file name should end with .log because we just checked that");

                        let Ok(naive) =
                            NaiveDateTime::parse_from_str(file_datetime_str, "%Y-%m-%d_%H-%M-%S")
                        else {
                            continue;
                        };

                        // Use from_local_datetime since the filename is in local time
                        let Some(file_as_datetime) = Local.from_local_datetime(&naive).single()
                        else {
                            continue;
                        };

                        let (log_id, tx) =
                            self.create_log(instance_id, Some(file_as_datetime)).await;

                        // read the file and send it to the log
                        let Ok(mut file) = tokio::fs::File::open(entry.path()).await else {
                            tracing::error!({ file_name = ?file_name }, "Failed to open log file");
                            continue;
                        };

                        let mut stdout_processor =
                            LogProcessor::new(LogEntrySourceKind::StdOut, &tx).await;

                        let mut buf = Vec::new();
                        let _ = file.read_to_end(&mut buf).await;

                        if let Err(e) = stdout_processor.process_data(&buf, None).await {
                            tracing::error!({ error = ?e }, "Failed to process stdout data");
                        }
                    }
                }
            }
        }

        read_logs_from_memory(self.clone(), instance_id, logs_path.clone()).await
    }

    pub async fn search_in_log(
        self,
        id: GameLogId,
        query: &str,
        match_case: bool,
        match_whole_word: bool,
        use_regex: bool,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let log = self.get_log(id).await?;
        let log = log.borrow();

        Ok(log.search(query, match_case, match_whole_word, use_regex))
    }
}

pub fn format_message_as_log4j_event(message: &str) -> String {
    format!(
        "<log4j:Event logger=\"GDLAUNCHER\" timestamp=\"{}\" level=\"INFO\" thread=\"N/A\">\n\t<log4j:Message><![CDATA[{}]]></log4j:Message>\n</log4j:Event>\n",
        Utc::now().timestamp_millis(),
        message
    )
}

/// On-disk game log with a hard size cap ([`crate::logger::MAX_LOG_FILE_SIZE`]): once
/// reached, further output is dropped after a single truncation notice, so a
/// log-spamming game can't fill the disk.
pub struct CappedLogFile<'a> {
    file: &'a mut File,
    written: u64,
    truncated: bool,
}

impl<'a> CappedLogFile<'a> {
    pub fn new(file: &'a mut File, already_written: u64) -> Self {
        Self {
            file,
            written: already_written,
            truncated: false,
        }
    }

    pub async fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        if self.truncated {
            return Ok(());
        }

        if self.written + data.len() as u64 > crate::logger::MAX_LOG_FILE_SIZE {
            self.truncated = true;
            self.file
                .write_all(crate::logger::LOG_TRUNCATION_NOTICE)
                .await?;
            self.file.flush().await?;
            return Ok(());
        }

        self.file.write_all(data).await?;
        self.written += data.len() as u64;
        Ok(())
    }
}

pub struct LogProcessor<'a> {
    pub parser: LogParser,
    pub kind: LogEntrySourceKind,
    pub log: &'a watch::Sender<GameLog>,
}

impl<'a> LogProcessor<'a> {
    pub async fn new(kind: LogEntrySourceKind, log: &'a watch::Sender<GameLog>) -> Self {
        Self {
            parser: LogParser::new(),
            kind,
            log,
        }
    }

    pub async fn process_data(
        &mut self,
        data: &[u8],
        file: Option<&mut CappedLogFile<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file) = file {
            file.write_all(data).await?;
        }

        let data = &*strip_ansi_escapes::strip(data);

        self.parser.feed(data);

        loop {
            let buffered_before = self.parser.buffered_len();

            let item = match self.parser.parse_next() {
                Ok(item) => item,
                Err(e) => {
                    tracing::warn!({ error = ?e }, "Skipping malformed log event");

                    // Most error paths (e.g. a complete-but-unparseable
                    // `<log4j:Event>`) already advanced the buffer past the
                    // bad bytes before returning, so it's safe -- and
                    // necessary, to reach any good events buffered right
                    // behind it -- to keep parsing the same chunk. The
                    // remaining paths leave the buffer untouched, so this
                    // stops for this call rather than re-erroring on the same
                    // bytes in a tight loop. Some of those resolve once more
                    // data arrives, the same as a `Partial` result -- but the
                    // leading-text non-UTF-8 arm does not: the bad prefix is
                    // already fully buffered, so no amount of further data
                    // makes it valid, and parsing stays stuck on it until the
                    // parser itself is reset (a preexisting gap, not fixed
                    // here). Either way `buffered_len` strictly decreases or
                    // the loop exits, so this can't spin.
                    if self.parser.buffered_len() < buffered_before {
                        continue;
                    }
                    break;
                }
            };

            let Some(item) = item else {
                break;
            };

            match item {
                ParsedItem::LogEntry(entry) => {
                    self.log
                        .send_if_modified(|log| log.try_add_entry((self.kind, entry).into()));
                }
                ParsedItem::PlainText(text) => {
                    self.log.send_if_modified(|log| {
                        log.try_add_entry(LogEntry::plaintext(text, self.kind))
                    });
                }
                ParsedItem::Partial(_) => {
                    break;
                }
            }
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
#[error("log id does not refer to a valid game log")]
pub struct InvalidGameLogIdError;

#[cfg(test)]
mod test {
    use super::*;

    /// Regression: a malformed event used to `?`-abort the whole chunk out
    /// of `process_data`, stranding any good events buffered right behind
    /// it until more bytes happened to arrive later. The bad event here is
    /// complete but unparseable (non-numeric timestamp), which
    /// `carbon_parsing`'s parser already advances the buffer past before
    /// returning `Err` -- so the good event that follows in the very same
    /// chunk must surface out of this same `process_data` call.
    #[tokio::test]
    async fn process_data_surfaces_a_good_event_behind_a_malformed_one_in_one_call() {
        let (tx, rx) = watch::channel(GameLog::new());
        let mut processor = LogProcessor::new(LogEntrySourceKind::StdOut, &tx).await;

        let bad_event = r#"<log4j:Event logger="Logger1" timestamp="not-a-number" level="INFO" thread="main">
            <log4j:Message><![CDATA[bad timestamp]]></log4j:Message>
        </log4j:Event>"#;
        let good_event = r#"<log4j:Event logger="Logger1" timestamp="1234567890" level="INFO" thread="main">
            <log4j:Message><![CDATA[good message]]></log4j:Message>
        </log4j:Event>"#;

        let mut chunk = Vec::new();
        chunk.extend_from_slice(bad_event.as_bytes());
        chunk.extend_from_slice(good_event.as_bytes());

        processor
            .process_data(&chunk, None)
            .await
            .expect("a malformed event must be skipped, not returned as an error");

        let log = rx.borrow();
        assert_eq!(log.len(), 1, "expected only the good event to be recorded");
        assert_eq!(log.get_entry(0).unwrap().message.trim(), "good message");
    }

    #[test]
    fn span() {
        let mut log = GameLog::new();

        log.add_entry(LogEntry::system_message("item 1"));
        log.add_entry(LogEntry::system_message("item 2"));
        log.add_entry(LogEntry::system_message("item 3"));
        log.add_entry(LogEntry::system_message("item 4"));

        // Test each kind of range

        #[track_caller]
        fn test_span<R, const N: usize>(log: &GameLog, range: R, expected: [&str; N])
        where
            R: std::ops::RangeBounds<usize>,
        {
            let span = log
                .get_span(range)
                .iter()
                .map(|entry| &entry.message)
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
        #[allow(clippy::reversed_empty_ranges)]
        test_span(&log, 1..0, []);
        test_span(&log, 1..2, ["item 2"]);
        test_span(&log, 1..=3, ["item 2", "item 3", "item 4"]);
    }

    #[test]
    fn entry_cap() {
        let mut log = GameLog::new();

        for i in 0..(MAX_LOG_ENTRIES + 100) {
            log.add_entry(LogEntry::system_message(format!("item {i}")));
        }

        // Capped entries plus a single truncation notice, indices never shift
        assert_eq!(log.len(), MAX_LOG_ENTRIES + 1);
        let last = log.get_entry(MAX_LOG_ENTRIES).unwrap();
        assert!(last.message.contains("further output will not be shown"));

        assert!(!log.try_add_entry(LogEntry::system_message("dropped")));
        assert_eq!(log.len(), MAX_LOG_ENTRIES + 1);
    }

    #[test]
    fn search() {
        let mut log = GameLog::new();

        log.add_entry(LogEntry::system_message("item 1"));
        log.add_entry(LogEntry::system_message("item 2"));
        log.add_entry(LogEntry::system_message("item 3"));

        let results = log.search("item", false, false, false);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].entry_index, 0);
        assert_eq!(results[1].entry_index, 1);
        assert_eq!(results[2].entry_index, 2);
    }

    #[test]
    fn search_multiline() {
        let mut log = GameLog::new();

        let msg = r#"first\u001bsomething\u001belse"#;

        log.add_entry(LogEntry::system_message(msg));

        let results = log.search("els", false, false, false);
        println!("{:?}", results);
    }
}
