use carbon_parsing::log::{LogParser, ParsedItem};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use itertools::Itertools;
use serde::Serialize;
use std::{
    borrow::Cow,
    ops::{Bound, RangeBounds},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    },
    time::Instant,
};
use unicode_segmentation::UnicodeSegmentation;

use thiserror::Error;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{Mutex, watch},
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
            match regex::RegexBuilder::new(query)
                .case_insensitive(!match_case)
                .build()
            {
                Ok(r) => Some(r),
                Err(_) => return vec![],
            }
        } else {
            None
        };

        for (entry_index, entry) in self.entries.iter().enumerate() {
            if let Some(regex) = &regex {
                for mat in regex.find_iter(&entry.message) {
                    results.push(SearchResult {
                        entry_index,
                        pos: entry.message[..mat.start()].encode_utf16().count(),
                        len: entry.message[mat.start()..mat.end()].encode_utf16().count(),
                    });
                }
                continue;
            }
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

            // Literal matching keeps its grapheme/overlap/word rules on the folded
            // string. Only the returned spans are mapped back to the original text.
            let mut folded_bytes = 0;
            let mut utf16 = 0;
            let original_spans: Vec<_> = entry
                .message
                .chars()
                .map(|ch| {
                    let width = if match_case {
                        ch.len_utf8()
                    } else {
                        ch.to_lowercase().map(char::len_utf8).sum()
                    };
                    let span = (
                        folded_bytes,
                        folded_bytes + width,
                        utf16,
                        utf16 + ch.len_utf16(),
                    );
                    folded_bytes += width;
                    utf16 += ch.len_utf16();
                    span
                })
                .collect();
            let boundaries: Vec<usize> = message
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .chain(std::iter::once(message.len()))
                .collect();
            let literal_span = |pos: usize, len: usize| {
                let start = boundaries[pos];
                let end = boundaries[pos + len];
                let begin = original_spans
                    .get(original_spans.partition_point(|(_, b, _, _)| *b <= start))
                    .map(|(_, _, a, _)| *a)
                    .unwrap_or(utf16);
                let finish = if start == end {
                    begin
                } else {
                    original_spans
                        .get(
                            original_spans
                                .partition_point(|(a, _, _, _)| *a < end)
                                .saturating_sub(1),
                        )
                        .map(|(_, _, _, b)| *b)
                        .unwrap_or(begin)
                };
                (begin, finish - begin)
            };

            if match_whole_word {
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
                                pos: literal_span(pos, query_graphemes.len()).0,
                                len: literal_span(pos, query_graphemes.len()).1,
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
                            pos: literal_span(pos, query_graphemes.len()).0,
                            len: literal_span(pos, query_graphemes.len()).1,
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
    /// UTF-16 code units in the original message (JavaScript slice offsets).
    pub pos: usize,
    /// UTF-16 code units in the original matched substring.
    pub len: usize,
}

static LOG_ID: AtomicI32 = AtomicI32::new(0);
impl ManagerRef<'_, InstanceManager> {
    pub async fn create_log(
        self,
        instance_id: InstanceId,
        datetime: Option<DateTime<Local>>,
    ) -> (GameLogId, watch::Sender<GameLog>) {
        let result = self.register_log(instance_id, datetime).await;
        self.app.invalidate(GET_LOGS, None);
        result
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

        // The gate is held only for this snapshot, taken against root
        // rename/delete. Reading and parsing past logs can take seconds on a
        // large pack, and must not hold up Play, settings or the game's state
        // changes meanwhile. A delete retires this history registration
        // under the gate, so a load it overtakes adds nothing back.
        let (logs_path, history) = {
            let _instance_guard = self.lock_instance(instance_id).await;
            let instance_lock = self.instances.read().await;
            match instance_lock.get(&instance_id) {
                Some(v) => (
                    Some(
                        self.app
                            .settings_manager()
                            .runtime_path
                            .get_instances()
                            .get_instance_path(&v.shortpath)
                            .get_gdl_logs_path(),
                    ),
                    Some(self.log_history_state(instance_id)),
                ),
                None => (None, None),
            }
        };

        if let (Some(path), Some(history)) = (&logs_path, &history) {
            #[cfg(test)]
            self.test_hooks.pause("log_history", instance_id).await;
            match self.load_log_history_into(instance_id, path, history).await {
                Ok(true) => self.app.invalidate(GET_LOGS, None),
                Ok(false) => {}
                Err(error) => tracing::error!(%error, "Failed to initialize retained log history"),
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

impl InstanceManager {
    async fn register_log(
        &self,
        instance_id: InstanceId,
        datetime: Option<DateTime<Local>>,
    ) -> (GameLogId, watch::Sender<GameLog>) {
        let (log_tx, log_rx) = watch::channel(GameLog::new());
        let id = GameLogId(LOG_ID.fetch_add(1, Ordering::Relaxed));

        let current_datetime = datetime.unwrap_or_else(chrono::Local::now);

        let mut logs = self.game_logs.write().await;
        // If history won the IO race, promote that record to the new live receiver.
        // An already-live same-second launch retains the pre-existing filename limitation.
        let existing = logs
            .iter()
            .find(|(_, (owner, date, rx))| {
                *owner == instance_id
                    && rx.has_changed().is_err()
                    && date.format("%Y-%m-%d_%H-%M-%S").to_string()
                        == current_datetime.format("%Y-%m-%d_%H-%M-%S").to_string()
            })
            .map(|(id, _)| *id);
        let id = existing.unwrap_or(id);
        logs.insert(id, (instance_id, current_datetime, log_rx));
        (id, log_tx)
    }

    /// This instance's history registration: one load at a time, and loaded
    /// entries are kept only while it is still the registered one.
    pub(super) fn log_history_state(&self, instance_id: InstanceId) -> Arc<Mutex<bool>> {
        self.log_history.entry(instance_id).or_default().clone()
    }

    #[cfg(test)]
    async fn load_log_history(
        &self,
        instance_id: InstanceId,
        path: &std::path::Path,
    ) -> anyhow::Result<bool> {
        let state = self.log_history_state(instance_id);
        self.load_log_history_into(instance_id, path, &state).await
    }

    pub(super) async fn load_log_history_into(
        &self,
        instance_id: InstanceId,
        path: &std::path::Path,
        state: &Arc<Mutex<bool>>,
    ) -> anyhow::Result<bool> {
        let mut initialized = state.lock().await;
        if *initialized {
            return Ok(false);
        }
        let entries = match tokio::fs::read_dir(path).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // No logs folder in a folder that is there: nothing played
                // yet. With the folder itself gone (renamed meanwhile) the
                // next look tries the new one.
                *initialized = path.parent().is_some_and(Path::exists);
                return Ok(false);
            }
            Err(e) => return Err(e.into()),
        };
        let entries = futures::stream::unfold(entries, |mut entries| async move {
            match entries.next_entry().await {
                Ok(Some(entry)) => Some((Ok(entry.path()), entries)),
                Ok(None) => None,
                Err(error) => Some((Err(error), entries)),
            }
        });
        let changed = self
            .merge_log_history_entries(instance_id, state, entries)
            .await?;
        *initialized = true;
        Ok(changed)
    }

    async fn merge_log_history_entries(
        &self,
        instance_id: InstanceId,
        state: &Arc<Mutex<bool>>,
        entries: impl futures::Stream<Item = std::io::Result<std::path::PathBuf>>,
    ) -> anyhow::Result<bool> {
        let mut changed = false;
        let mut failures = 0usize;
        let mut first_error = None;
        let mut record_failure = |error: anyhow::Error| {
            tracing::warn!(%error, "Failed to load retained log history entry");
            failures += 1;
            if first_error.is_none() {
                first_error = Some(error);
            }
        };
        use futures::StreamExt;
        tokio::pin!(entries);
        while let Some(entry) = entries.next().await {
            let path = match entry {
                Ok(path) => path,
                Err(error) => {
                    record_failure(anyhow::anyhow!(
                        "Failed to enumerate retained log entry: {error}"
                    ));
                    continue;
                }
            };
            let name = path.file_name().expect("directory entries have filenames");
            let name = name.to_string_lossy();
            let Some(stem) = name.strip_suffix(".log") else {
                continue;
            };
            let Ok(naive) = NaiveDateTime::parse_from_str(stem, "%Y-%m-%d_%H-%M-%S") else {
                continue;
            };
            let Some(datetime) = Local.from_local_datetime(&naive).single() else {
                continue;
            };
            let matches = |id: &InstanceId, date: &DateTime<Local>| {
                *id == instance_id && date.format("%Y-%m-%d_%H-%M-%S").to_string() == stem
            };
            if self
                .game_logs
                .read()
                .await
                .values()
                .any(|(id, date, _)| matches(id, date))
            {
                continue;
            }
            let bytes = match tokio::fs::read(&path).await {
                Ok(bytes) => bytes,
                Err(error) => {
                    record_failure(anyhow::anyhow!(
                        "Failed to read retained log {}: {error}",
                        path.display()
                    ));
                    continue;
                }
            };
            let (tx, rx) = watch::channel(GameLog::new());
            if let Err(error) = LogProcessor::new(LogEntrySourceKind::StdOut, &tx)
                .await
                .process_data(&bytes, None)
                .await
            {
                record_failure(anyhow::anyhow!(
                    "Failed to process retained log {}: {error}",
                    path.display()
                ));
                continue;
            }
            drop(tx);
            // create_log may have registered this same filename while IO was pending.
            let mut logs = self.game_logs.write().await;
            // A delete retired this registration while the file was read
            // (it does so before clearing the instance's logs).
            let retired = !self
                .log_history
                .get(&instance_id)
                .is_some_and(|current| Arc::ptr_eq(&current, state));
            if retired {
                return Ok(changed);
            }
            if !logs.values().any(|(id, date, _)| matches(id, date)) {
                let id = GameLogId(LOG_ID.fetch_add(1, Ordering::Relaxed));
                logs.insert(id, (instance_id, datetime, rx));
                changed = true;
            }
        }
        // Healthy entries are already merged. Keep initialization incomplete so a
        // later request retries failures without replacing healthy/live receivers.
        if let Some(error) = first_error {
            anyhow::bail!(
                "Failed to load {failures} retained log history entries; first error: {error}"
            );
        }
        Ok(changed)
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

                    // Complete bad events and invalid plaintext are consumed by
                    // the parser. Continue only when recovery made progress;
                    // genuinely partial XML/UTF-8 is returned as Partial below.
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

#[cfg(test)]
mod task17_tests {
    use super::*;

    fn log(message: &str) -> GameLog {
        let mut log = GameLog::new();
        log.add_entry(LogEntry::plaintext(message, LogEntrySourceKind::StdOut));
        log
    }

    #[test]
    fn task17_search_regex_case_toggle() {
        let log = log("ERROR launching Minecraft");
        assert_eq!(log.search("ERROR", false, false, true).len(), 1);
        assert_eq!(log.search("ERROR", true, false, true).len(), 1);
        assert_eq!(log.search("error", false, false, true).len(), 1);
        assert_eq!(log.search("ERROR", false, false, false).len(), 1);
        assert!(log.search("error", true, false, true).is_empty());
        assert!(log.search("[", false, false, true).is_empty());
        assert_eq!(log.search(r"\p{Lu}+", true, false, true).len(), 2);
    }

    #[test]
    fn task17_search_original_utf16_spans() {
        for message in [
            "😀 ERROR",
            "e\u{301} ERROR",
            "İ ERROR",
            "👩‍💻 ERROR",
            "ascii ERROR",
        ] {
            for regex in [false, true] {
                let results = log(message).search("error", false, false, regex);
                assert_eq!(results.len(), 1, "{message} regex={regex}");
                let units: Vec<u16> = message.encode_utf16().collect();
                let r = &results[0];
                assert_eq!(
                    String::from_utf16(&units[r.pos..r.pos + r.len]).unwrap(),
                    "ERROR",
                    "{message} regex={regex}"
                );
            }
        }
    }

    #[test]
    fn task17_search_literal_controls() {
        assert_eq!(log("banana").search("ana", true, false, false).len(), 2);
        assert_eq!(
            log("error terror error_ error!")
                .search("error", false, true, false)
                .len(),
            3
        );
        assert!(log("e\u{301}").search("e", true, false, false).is_empty());
        let r = log("İ").search("i\u{307}", false, false, false);
        assert_eq!((r[0].pos, r[0].len), (0, 1));
    }

    #[tokio::test]
    async fn task17_history_play_before_logs_preserves_live_receiver() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("2025-01-14_12-00-00.log"),
            b"retained history\n",
        )
        .unwrap();
        let manager = InstanceManager::new();
        let id = InstanceId(17);
        let now = Local
            .with_ymd_and_hms(2025, 1, 15, 12, 0, 0)
            .single()
            .unwrap();
        let (current, tx) = manager.register_log(id, Some(now)).await;
        std::fs::write(
            dir.path().join("2025-01-15_12-00-00.log"),
            b"current disk\n",
        )
        .unwrap();
        manager.load_log_history(id, dir.path()).await.unwrap();
        let logs = manager.game_logs.read().await;
        assert_eq!(logs.len(), 2);
        tx.send_modify(|log| log.add_entry(LogEntry::system_message("later")));
        assert_eq!(
            logs[&current]
                .2
                .borrow()
                .search("later", true, false, false)
                .len(),
            1
        );
        assert!(logs.values().any(|(_, _, rx)| {
            !rx.borrow()
                .search("retained", true, false, false)
                .is_empty()
        }));
        assert!(std::fs::read(dir.path().join("2025-01-14_12-00-00.log")).is_ok());
    }

    #[tokio::test]
    async fn task17_history_logs_first_and_current_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("2025-01-14_12-00-00.log"), b"retained\n").unwrap();
        let manager = InstanceManager::new();
        manager
            .load_log_history(InstanceId(17), dir.path())
            .await
            .unwrap();
        assert_eq!(manager.game_logs.read().await.len(), 1);
        manager
            .load_log_history(InstanceId(17), dir.path())
            .await
            .unwrap();
        assert_eq!(manager.game_logs.read().await.len(), 1);
        let empty = tempfile::tempdir().unwrap();
        let manager = InstanceManager::new();
        manager.register_log(InstanceId(17), None).await;
        manager
            .load_log_history(InstanceId(17), empty.path())
            .await
            .unwrap();
        assert_eq!(manager.game_logs.read().await.len(), 1);
    }
    #[tokio::test]
    async fn task17_history_concurrent_load_retry_and_registration_orders() {
        let dir = tempfile::tempdir().unwrap();
        let blocked = dir.path().join("logs");
        std::fs::write(&blocked, b"not a directory").unwrap();
        let manager = InstanceManager::new();
        let id = InstanceId(17);
        assert!(manager.load_log_history(id, &blocked).await.is_err());
        std::fs::remove_file(&blocked).unwrap();
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("2025-01-14_12-00-00.log"), b"old\n").unwrap();
        let (a, b) = tokio::join!(
            manager.load_log_history(id, &blocked),
            manager.load_log_history(id, &blocked)
        );
        assert!(a.unwrap());
        assert!(!b.unwrap());
        assert_eq!(manager.game_logs.read().await.len(), 1);
        let date = Local
            .with_ymd_and_hms(2025, 1, 14, 12, 0, 0)
            .single()
            .unwrap();
        let (current, tx) = manager.register_log(id, Some(date)).await;
        assert_eq!(manager.game_logs.read().await.len(), 1);
        tx.send_modify(|log| log.add_entry(LogEntry::system_message("current")));
        assert_eq!(manager.game_logs.read().await[&current].2.borrow().len(), 1);
        // An initialized registry does not resurrect a log removed from memory.
        drop(tx);
        manager.game_logs.write().await.clear();
        assert!(!manager.load_log_history(id, &blocked).await.unwrap());
        assert!(manager.game_logs.read().await.is_empty());
        // A fresh core still rediscovers the retained physical session.
        let fresh = InstanceManager::new();
        fresh.load_log_history(id, &blocked).await.unwrap();
        assert_eq!(fresh.game_logs.read().await.len(), 1);
    }

    #[test]
    fn task17_search_matching_unicode_and_regex_syntax() {
        for (message, query, pos, len) in [
            ("a 😀 b", "😀", 2, 2),
            ("a e\u{301} b", "e\u{301}", 2, 2),
            ("İ ERROR", "ERROR", 2, 5),
        ] {
            for regex in [false, true] {
                let results = log(message).search(query, false, true, regex);
                assert_eq!((results[0].pos, results[0].len), (pos, len));
            }
        }
        assert_eq!(log("foo BAR").search(r"\S+", false, false, true).len(), 2);
        assert_eq!(log("error").search("ERROR", false, false, true).len(), 1);
        assert_eq!(log("ΟΣ").search("ος", false, false, false).len(), 1);
    }
    #[tokio::test]
    async fn task17_r1_history_bad_entry_does_not_hide_later_healthy_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("2025-01-14_12-00-00.log");
        let good = dir.path().join("2025-01-15_12-00-00.log");
        std::fs::create_dir(&bad).unwrap();
        std::fs::write(&good, b"healthy retained\n").unwrap();
        let manager = InstanceManager::new();
        let id = InstanceId(17);
        let now = Local
            .with_ymd_and_hms(2025, 1, 16, 12, 0, 0)
            .single()
            .unwrap();
        let (live, tx) = manager.register_log(id, Some(now)).await;
        let result = manager
            .merge_log_history_entries(
                id,
                &manager.log_history_state(id),
                futures::stream::iter(vec![Ok(bad.clone()), Ok(good.clone())]),
            )
            .await;
        assert!(
            result.is_err(),
            "failed historical identity must remain diagnosable"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("2025-01-14_12-00-00.log")
        );
        assert_eq!(
            manager.game_logs.read().await.len(),
            2,
            "a failed earlier entry must not hide healthy later history"
        );
        let healthy_id = *manager
            .game_logs
            .read()
            .await
            .keys()
            .find(|log| **log != live)
            .unwrap();
        tx.send_modify(|log| log.add_entry(LogEntry::system_message("still live")));
        assert_eq!(
            manager.game_logs.read().await[&live]
                .2
                .borrow()
                .search("still live", true, false, false)
                .len(),
            1
        );
        assert!(manager.load_log_history(id, dir.path()).await.is_err());
        assert_eq!(manager.game_logs.read().await.len(), 2);
        std::fs::remove_dir(&bad).unwrap();
        std::fs::write(&bad, b"repaired history\n").unwrap();
        assert!(manager.load_log_history(id, dir.path()).await.unwrap());
        let logs = manager.game_logs.read().await;
        assert_eq!(logs.len(), 3);
        assert!(logs.contains_key(&healthy_id));
        assert!(logs.contains_key(&live));
        assert!(logs.values().any(|(_, _, rx)| {
            !rx.borrow()
                .search("repaired", true, false, false)
                .is_empty()
        }));
        drop(logs);
        assert!(!manager.load_log_history(id, dir.path()).await.unwrap());
    }

    #[tokio::test]
    async fn task17_r1_history_entry_enumeration_error_keeps_following_file() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("2025-01-15_12-00-00.log");
        std::fs::write(&good, b"retained after enumeration error\n").unwrap();
        let manager = InstanceManager::new();
        let entries = futures::stream::iter(vec![
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "controlled directory entry denial",
            )),
            Ok(good),
        ]);
        assert!(
            manager
                .merge_log_history_entries(
                    InstanceId(17),
                    &manager.log_history_state(InstanceId(17)),
                    entries
                )
                .await
                .is_err()
        );
        assert_eq!(
            manager.game_logs.read().await.len(),
            1,
            "enumeration errors must not hide later valid entries"
        );
    }
}
