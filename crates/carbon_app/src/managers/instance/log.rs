use carbon_parsing::log::{LogParser, ParsedItem};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use itertools::Itertools;
use serde::Serialize;
use std::{
    borrow::Cow,
    ops::{Bound, RangeBounds},
    path::{Path, PathBuf},
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
use super::ansi::AnsiStripper;

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
    /// The game printed that it crashed, whether or not that line was kept.
    said_crashed: bool,
}

impl GameLog {
    /// Every place the query is found in the log.
    #[cfg(test)]
    pub fn search(
        &self,
        query: &str,
        match_case: bool,
        match_whole_word: bool,
        use_regex: bool,
    ) -> Vec<SearchResult> {
        self.search_up_to(query, match_case, match_whole_word, use_regex, usize::MAX)
            .results
    }

    /// The first `limit` places the query is found in the log, and whether
    /// there are more.
    ///
    /// A hit is never empty: a pattern that can match nothing (`error|`,
    /// `x*`) matches at every character, and such a hit has nothing to
    /// highlight or to step to.
    pub fn search_up_to(
        &self,
        query: &str,
        match_case: bool,
        match_whole_word: bool,
        use_regex: bool,
        limit: usize,
    ) -> SearchResults {
        let mut found = SearchResults::default();
        if query.is_empty() {
            return found;
        }

        if use_regex {
            let Ok(regex) = regex::RegexBuilder::new(query)
                .case_insensitive(!match_case)
                .build()
            else {
                return found;
            };

            for (entry_index, entry) in self.entries.iter().enumerate() {
                // Hits come in order, so the UTF-16 position of each is
                // counted on from the one before, not from the start of the
                // message again.
                let mut counted_bytes = 0;
                let mut counted_utf16 = 0;
                let mut utf16_at = |byte: usize| {
                    counted_utf16 += entry.message[counted_bytes..byte].encode_utf16().count();
                    counted_bytes = byte;
                    counted_utf16
                };

                for hit in regex.find_iter(&entry.message) {
                    if hit.is_empty() {
                        continue;
                    }
                    let pos = utf16_at(hit.start());
                    let len = utf16_at(hit.end()) - pos;
                    if !found.push(
                        SearchResult {
                            entry_index,
                            pos,
                            len,
                        },
                        limit,
                    ) {
                        return found;
                    }
                }
            }

            return found;
        }

        let search_query = if match_case {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        let query_graphemes: Vec<&str> = search_query.graphemes(true).collect();

        for (entry_index, entry) in self.entries.iter().enumerate() {
            let message: Cow<str> = if match_case {
                entry.message.as_str().into()
            } else {
                entry.message.to_lowercase().into()
            };

            // A hit is a run of whole graphemes that spells the query, so a
            // message that does not hold the query's text cannot hold one.
            // Most messages don't, and are spared the per-character work.
            if !message.contains(&*search_query) {
                continue;
            }

            let message_graphemes: Vec<&str> = message.graphemes(true).collect();

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

            for pos in 0..message_graphemes.len() {
                if !message_graphemes[pos..].starts_with(&query_graphemes) {
                    continue;
                }

                if match_whole_word {
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
                    if !(is_word_start && is_word_end) {
                        continue;
                    }
                }

                let (pos, len) = literal_span(pos, query_graphemes.len());
                if !found.push(
                    SearchResult {
                        entry_index,
                        pos,
                        len,
                    },
                    limit,
                ) {
                    return found;
                }
            }
        }

        found
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
    /// An exception kept apart from the message, for the Java issue check,
    /// which reads why a game crashed. An entry read from the game's log has
    /// none: its exception is already under its `message`, where the log
    /// view shows it and the check reads it.
    #[serde(skip)]
    pub throwable: Option<String>,
}

impl From<(LogEntrySourceKind, carbon_parsing::log::LogEntry)> for LogEntry {
    fn from((source_kind, entry): (LogEntrySourceKind, carbon_parsing::log::LogEntry)) -> Self {
        // The exception goes under its message, the way the game's own log
        // file prints the two.
        let message = entry.message_with_throwable();
        let carbon_parsing::log::LogEntry {
            logger,
            level,
            timestamp,
            thread_name,
            ..
        } = entry;

        Self {
            source_kind,
            logger: logger.to_owned(),
            timestamp,
            thread: thread_name.to_owned(),
            level: level.into(),
            message,
            throwable: None,
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
            throwable: None,
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
            throwable: None,
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
        // Before the cap: a long session's last lines are the ones that say
        // how it ended.
        if !self.said_crashed && super::run::game_crash::says_crashed(&entry.message) {
            self.said_crashed = true;
        }
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

        self.message_bytes += entry.message.len() + entry.throwable.as_ref().map_or(0, String::len);
        self.entries.push(entry);
        true
    }

    /// Whether the game printed that it crashed.
    pub fn said_crashed(&self) -> bool {
        self.said_crashed
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

/// The most hits one search answers with. A single letter is found
/// millions of times in a large log, far more than can be stepped through,
/// and every hit is sent to the interface.
pub const MAX_SEARCH_RESULTS: usize = 10_000;

/// The hits of a search, in the order they stand in the log.
#[derive(Debug, Default)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    /// There are more hits than the search was asked for.
    pub truncated: bool,
}

impl SearchResults {
    /// Takes the hit unless `limit` hits are held already. Says whether the
    /// search goes on.
    fn push(&mut self, result: SearchResult, limit: usize) -> bool {
        if self.results.len() >= limit {
            self.truncated = true;
            return false;
        }
        self.results.push(result);
        true
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

/// How a session's log file is named after the time the session started.
const LOG_FILE_STAMP: &str = "%Y-%m-%d_%H-%M-%S";

fn log_file_name(datetime: &DateTime<Local>) -> String {
    format!("{}.log", datetime.format(LOG_FILE_STAMP))
}

/// What the launcher holds of one game session's log.
///
/// Past sessions are known by their file and read when one is opened, so
/// that listing them costs nothing and only what is looked at takes memory.
#[derive(Debug)]
pub struct GameLogRecord {
    pub instance_id: InstanceId,
    /// When the session started; its file is named after it.
    pub datetime: DateTime<Local>,
    /// The log in memory: a running game's, and a past session's from when
    /// it is opened until another one is. `None` for a session known by its
    /// file only.
    pub log: Option<watch::Receiver<GameLog>>,
    /// The whole log can be read back from its file, so the copy in memory
    /// may be let go.
    pub on_disk: bool,
}

impl GameLogRecord {
    /// A game is still writing it.
    pub fn is_active(&self) -> bool {
        self.log
            .as_ref()
            .is_some_and(|log| log.has_changed().is_ok())
    }

    pub fn file_name(&self) -> String {
        log_file_name(&self.datetime)
    }
}

/// Why a session's log could not be opened.
#[derive(Debug)]
pub enum OpenLogError {
    /// No session has this id (any more).
    Unknown,
    /// Its file is not in the folder that was looked in.
    Missing,
    /// Its file could not be read.
    Unreadable(String),
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
            Some(record) if record.is_active() => Err(anyhow::anyhow!("cannot delete active log")),
            Some(_) => {
                let _ = logs.remove(&id);
                self.app.invalidate(GET_LOGS, None);
                Ok(())
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
        let filename = logs
            .get(&log_id)
            .ok_or_else(|| anyhow::anyhow!(InvalidGameLogIdError))?
            .file_name();
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

        let log_file_path = logs_path.join(&filename);

        // Return the file path if it exists, otherwise return the logs folder
        if log_file_path.exists() {
            Ok(log_file_path.to_string_lossy().to_string())
        } else {
            Ok(logs_path.to_string_lossy().to_string())
        }
    }

    /// The log of a session. A past session's is read from its file first
    /// (see [`InstanceManager::open_log`]).
    ///
    /// A session whose file is gone (deleted by hand, or rotated away between
    /// the list and the click) cannot be opened: it leaves the list, and the
    /// interface is told so.
    pub async fn get_log(
        self,
        id: GameLogId,
    ) -> Result<watch::Receiver<GameLog>, InvalidGameLogIdError> {
        let instance_id = match self.held_log(id).await {
            Ok((Some(log), _)) => return Ok(log),
            Ok((None, instance_id)) => instance_id,
            Err(_) => return Err(InvalidGameLogIdError),
        };

        let mut looked_in: Option<PathBuf> = None;
        loop {
            // Looked up for every try: a rename moves the folder, and a file
            // that is not where the folder was is not a file that is gone.
            let Some(logs_dir) = self.logs_dir(instance_id).await else {
                return Err(InvalidGameLogIdError);
            };
            #[cfg(test)]
            self.test_hooks.pause("log_open", instance_id).await;
            match self.open_log(id, &logs_dir).await {
                Ok(log) => return Ok(log),
                Err(OpenLogError::Unknown) => return Err(InvalidGameLogIdError),
                Err(OpenLogError::Unreadable(error)) => {
                    tracing::warn!(%error, "Failed to read a past session's log");
                    return Err(InvalidGameLogIdError);
                }
                Err(OpenLogError::Missing) if looked_in.as_ref() == Some(&logs_dir) => {
                    if self.forget_log_on_disk(id).await {
                        self.app.invalidate(GET_LOGS, None);
                    }
                    return Err(InvalidGameLogIdError);
                }
                Err(OpenLogError::Missing) => looked_in = Some(logs_dir),
            }
        }
    }

    /// Where the instance's session logs are kept, by its folder as it is
    /// named now.
    async fn logs_dir(self, instance_id: InstanceId) -> Option<PathBuf> {
        let instances = self.instances.read().await;
        let instance = instances.get(&instance_id)?;
        Some(
            self.app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&instance.shortpath)
                .get_gdl_logs_path(),
        )
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
                .filter(|(_, record)| record.instance_id == instance_id)
                .map(|(id, record)| {
                    let active = record.is_active();
                    // For inactive logs, compute file size from the log file
                    let file_size = if active {
                        None
                    } else if let Some(ref logs_path) = logs_path {
                        let file_path = logs_path.join(record.file_name());
                        std::fs::metadata(&file_path).ok().map(|m| m.len())
                    } else {
                        None
                    };
                    GameLogEntry {
                        id: *id,
                        instance_id: record.instance_id,
                        active,
                        datetime: record.datetime,
                        file_size,
                    }
                })
                .sorted_by_key(|entry| entry.id.0)
                .collect()
        }

        // The gate is held only for this snapshot, taken against root
        // rename/delete. Looking through the folder of past logs must not
        // hold up Play, settings or the game's state changes meanwhile. A delete retires this history registration
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
    ) -> anyhow::Result<SearchResults> {
        let log = self.get_log(id).await?;
        let log = log.borrow();

        Ok(log.search_up_to(
            query,
            match_case,
            match_whole_word,
            use_regex,
            MAX_SEARCH_RESULTS,
        ))
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
        let file_name = log_file_name(&current_datetime);
        let existing = logs
            .iter()
            .find(|(_, record)| {
                record.instance_id == instance_id
                    && !record.is_active()
                    && record.file_name() == file_name
            })
            .map(|(id, _)| *id);
        let id = existing.unwrap_or(id);
        logs.insert(
            id,
            GameLogRecord {
                instance_id,
                datetime: current_datetime,
                log: Some(log_rx),
                // Not until the game has ended and its file is closed (see
                // `end_log`).
                on_disk: false,
            },
        );
        (id, log_tx)
    }

    /// The session's log if it is in memory, and the instance it belongs to.
    /// Asking for a log is opening it, so the past sessions opened before
    /// are let go (see `release_other_logs`).
    pub(super) async fn held_log(
        &self,
        id: GameLogId,
    ) -> Result<(Option<watch::Receiver<GameLog>>, InstanceId), OpenLogError> {
        let (log, instance_id) = {
            let logs = self.game_logs.read().await;
            let record = logs.get(&id).ok_or(OpenLogError::Unknown)?;
            (record.log.clone(), record.instance_id)
        };
        if log.is_some() {
            self.release_other_logs(id).await;
        }
        Ok((log, instance_id))
    }

    /// The log of a session, read from its file in `logs_dir` when it is not
    /// in memory. It then stays in memory until another session is opened.
    pub(super) async fn open_log(
        &self,
        id: GameLogId,
        logs_dir: &Path,
    ) -> Result<watch::Receiver<GameLog>, OpenLogError> {
        if let (Some(log), _) = self.held_log(id).await? {
            return Ok(log);
        }

        // One file at a time: the stream and a search asking for the same
        // session read it once, and the second finds it in memory.
        let _reading = self.log_reads.lock().await;
        let file_name = {
            let logs = self.game_logs.read().await;
            let record = logs.get(&id).ok_or(OpenLogError::Unknown)?;
            if let Some(log) = &record.log {
                return Ok(log.clone());
            }
            record.file_name()
        };

        // Off the async workers: a long session takes a while to parse, and
        // the workers are what every other request is answered on.
        let path = logs_dir.join(&file_name);
        let rx = match tokio::task::spawn_blocking(move || read_log_file(&path)).await {
            Ok(read) => read?,
            Err(error) => return Err(OpenLogError::Unreadable(error.to_string())),
        };

        {
            let mut logs = self.game_logs.write().await;
            // Deleted with its instance while the file was read.
            let record = logs.get_mut(&id).ok_or(OpenLogError::Unknown)?;
            match &record.log {
                // A game started in the same second took the record over.
                Some(log) => return Ok(log.clone()),
                None => record.log = Some(rx.clone()),
            }
        }
        self.release_other_logs(id).await;
        Ok(rx)
    }

    /// Lets go of every ended session's log in memory but `opened`: one
    /// past session is looked at at a time, and the others can be read from
    /// their files again. A running game's log is never let go, nor one
    /// that has no complete file.
    async fn release_other_logs(&self, opened: GameLogId) {
        let mut logs = self.game_logs.write().await;
        for (id, record) in logs.iter_mut() {
            if *id != opened && record.on_disk && record.log.is_some() && !record.is_active() {
                record.log = None;
            }
        }
    }

    /// Ends a game's log: `sender` is what the game's output was written
    /// through, and `on_disk` says whether all of it is in the session's
    /// file as well, flushed and closed.
    ///
    /// A log that is on disk and that no Logs tab is following is let go
    /// from memory here. One that is being followed stays until another
    /// session is opened, so the rows on screen and a search of them keep
    /// agreeing.
    pub async fn end_log(&self, id: GameLogId, sender: watch::Sender<GameLog>, on_disk: bool) {
        let mut logs = self.game_logs.write().await;
        // This registry holds one receiver; any other is a Logs tab
        // following the game, or a search under way.
        let followed = sender.receiver_count() > 1;
        drop(sender);
        if let Some(record) = logs.get_mut(&id) {
            record.on_disk = on_disk;
            if on_disk && !followed {
                record.log = None;
            }
        }
    }

    /// Forgets a session that is known by its file only, once that file
    /// turned out to be gone. Says whether the list of sessions changed.
    pub(super) async fn forget_log_on_disk(&self, id: GameLogId) -> bool {
        let mut logs = self.game_logs.write().await;
        if logs.get(&id).is_some_and(|record| record.log.is_none()) {
            logs.remove(&id);
            return true;
        }
        false
    }

    /// Forgets the ended sessions of an instance whose files are no longer
    /// in `logs_dir`: starting a game rotates the oldest files away, and a
    /// session without its file can no longer be opened. Says whether the
    /// list of sessions changed.
    pub async fn forget_logs_without_file(&self, instance_id: InstanceId, logs_dir: &Path) -> bool {
        let ended: Vec<(GameLogId, String)> = self
            .game_logs
            .read()
            .await
            .iter()
            .filter(|(_, record)| {
                record.instance_id == instance_id && record.on_disk && !record.is_active()
            })
            .map(|(id, record)| (*id, record.file_name()))
            .collect();

        let mut gone = Vec::new();
        for (id, file_name) in ended {
            // Only a file known to be absent counts; one that could not be
            // looked at stays listed.
            if matches!(
                tokio::fs::try_exists(logs_dir.join(&file_name)).await,
                Ok(false)
            ) {
                gone.push((id, file_name));
            }
        }

        let mut logs = self.game_logs.write().await;
        let mut changed = false;
        for (id, file_name) in gone {
            // Not one a game started in the same second has taken over.
            let still_ended = logs.get(&id).is_some_and(|record| {
                record.on_disk && !record.is_active() && record.file_name() == file_name
            });
            if still_ended {
                logs.remove(&id);
                changed = true;
            }
        }
        changed
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
            let matches = |record: &GameLogRecord| {
                record.instance_id == instance_id
                    && record.datetime.format(LOG_FILE_STAMP).to_string() == stem
            };
            if self.game_logs.read().await.values().any(matches) {
                continue;
            }
            // The file is only looked at here; it is read when its session
            // is opened (see `open_log`).
            match tokio::fs::metadata(&path).await {
                Ok(metadata) if metadata.is_file() => {}
                Ok(_) => {
                    record_failure(anyhow::anyhow!(
                        "Failed to read retained log {}: not a file",
                        path.display()
                    ));
                    continue;
                }
                Err(error) => {
                    record_failure(anyhow::anyhow!(
                        "Failed to read retained log {}: {error}",
                        path.display()
                    ));
                    continue;
                }
            }
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
            if !logs.values().any(matches) {
                let id = GameLogId(LOG_ID.fetch_add(1, Ordering::Relaxed));
                logs.insert(
                    id,
                    GameLogRecord {
                        instance_id,
                        datetime,
                        log: None,
                        on_disk: true,
                    },
                );
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

/// A past session's log, read from its file and parsed. Blocks for as long
/// as that takes.
fn read_log_file(path: &Path) -> Result<watch::Receiver<GameLog>, OpenLogError> {
    #[cfg(test)]
    read_thread::note(path);

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(OpenLogError::Missing);
        }
        Err(error) => {
            return Err(OpenLogError::Unreadable(format!(
                "{}: {error}",
                path.display()
            )));
        }
    };
    let (tx, rx) = watch::channel(GameLog::new());
    LogProcessor::for_stream(LogEntrySourceKind::StdOut, &tx).parse(&bytes);
    Ok(rx)
}

/// Which thread read each log file, for the test that it is not one of the
/// async workers.
#[cfg(test)]
pub(super) mod read_thread {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::thread::ThreadId;

    static READERS: Mutex<Option<HashMap<PathBuf, ThreadId>>> = Mutex::new(None);

    pub(super) fn note(path: &Path) {
        READERS
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(path.to_owned(), std::thread::current().id());
    }

    pub fn of(path: &Path) -> Option<ThreadId> {
        READERS.lock().unwrap().as_ref()?.get(path).copied()
    }
}

pub struct LogProcessor<'a> {
    pub parser: LogParser,
    /// Takes the colour codes out of the output before it is parsed.
    stripper: AnsiStripper,
    pub kind: LogEntrySourceKind,
    pub log: &'a watch::Sender<GameLog>,
}

impl<'a> LogProcessor<'a> {
    pub async fn new(kind: LogEntrySourceKind, log: &'a watch::Sender<GameLog>) -> Self {
        Self::for_stream(kind, log)
    }

    fn for_stream(kind: LogEntrySourceKind, log: &'a watch::Sender<GameLog>) -> Self {
        Self {
            parser: LogParser::new(),
            stripper: AnsiStripper::new(),
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

        self.parse(data);

        Ok(())
    }

    /// Adds what `data` completes to the log. Nothing in here waits.
    fn parse(&mut self, data: &[u8]) {
        let text = self.stripper.strip(data);

        self.parser.feed(text.as_bytes());

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

    /// The live path reads the game's output in pieces and past sessions
    /// are read whole; both go through `process_data`, and both must show the
    /// stack trace of a logged exception, findable by the search.
    #[tokio::test]
    async fn process_data_keeps_the_stack_trace_of_a_logged_exception() {
        let event = r#"<log4j:Event logger="ModLoader" timestamp="1234567890" level="ERROR" thread="main">
            <log4j:Message><![CDATA[Failed to create mod instance]]></log4j:Message>
            <log4j:Throwable><![CDATA[java.lang.IllegalStateException: duplicate member
	at org.example.Mod.init(Mod.java:151)
	at org.example.Mod.<init>(Mod.java:64)
]]></log4j:Throwable>
        </log4j:Event>"#;

        for pieces in [
            vec![event.as_bytes()],
            event.as_bytes().chunks(40).collect(),
        ] {
            let (tx, rx) = watch::channel(GameLog::new());
            let mut processor = LogProcessor::new(LogEntrySourceKind::StdOut, &tx).await;
            for piece in pieces {
                processor.process_data(piece, None).await.unwrap();
            }

            let log = rx.borrow();
            assert_eq!(log.len(), 1);
            let lines: Vec<&str> = log
                .get_entry(0)
                .unwrap()
                .message
                .lines()
                .map(str::trim)
                .collect();
            assert_eq!(
                lines,
                [
                    "Failed to create mod instance",
                    "java.lang.IllegalStateException: duplicate member",
                    "at org.example.Mod.init(Mod.java:151)",
                    "at org.example.Mod.<init>(Mod.java:64)",
                ]
            );
            assert_eq!(log.search("Mod.java:151", true, false, false).len(), 1);
        }
    }

    /// The rows of an output read in the given pieces.
    async fn rows(pieces: &[&[u8]]) -> Vec<String> {
        let (tx, rx) = watch::channel(GameLog::new());
        let mut processor = LogProcessor::new(LogEntrySourceKind::StdOut, &tx).await;
        for piece in pieces {
            processor.process_data(piece, None).await.unwrap();
        }
        let log = rx.borrow();
        (0..log.len())
            .map(|index| log.get_entry(index).unwrap().message.clone())
            .collect()
    }

    /// The game's output is read 1024 bytes at a time, so an event is cut
    /// wherever a read stops: inside a character, inside a colour code.
    #[tokio::test]
    async fn process_data_gives_the_same_row_wherever_a_read_stops() {
        let event = "<log4j:Event logger=\"Chat\" timestamp=\"1\" level=\"INFO\" thread=\"main\">\
            <log4j:Message><![CDATA[\x1b[1;32mИгрок 张伟\x1b[0m joined\n\tat a.B.c(B.java:1)]]></log4j:Message>\
            </log4j:Event>"
            .as_bytes();
        let expected = ["Игрок 张伟 joined\n\tat a.B.c(B.java:1)"];

        assert_eq!(rows(&[event]).await, expected);
        for at in 0..event.len() {
            assert_eq!(
                rows(&[&event[..at], &event[at..]]).await,
                expected,
                "read stopped after byte {at}"
            );
        }
    }

    fn log(message: &str) -> GameLog {
        let mut log = GameLog::new();
        log.add_entry(LogEntry::plaintext(message, LogEntrySourceKind::StdOut));
        log
    }

    fn hits(results: &[SearchResult]) -> Vec<(usize, usize, usize)> {
        results
            .iter()
            .map(|hit| (hit.entry_index, hit.pos, hit.len))
            .collect()
    }

    #[test]
    fn a_pattern_that_can_match_nothing_gives_no_empty_hits() {
        // On the way to typing "error|warn".
        assert_eq!(
            hits(&log("an error here").search("error|", false, false, true)),
            [(0, 3, 5)]
        );
        assert_eq!(
            hits(&log("axxb xx").search("x*", false, false, true)),
            [(0, 1, 2), (0, 5, 2)]
        );
        assert!(log("abc").search("x*", false, false, true).is_empty());
        assert!(log("abc").search("", false, false, true).is_empty());
        assert!(log("abc").search("", false, false, false).is_empty());
        assert!(log("abc").search("", false, true, false).is_empty());
    }

    #[test]
    fn several_regex_hits_in_a_row_keep_their_utf16_positions() {
        assert_eq!(
            hits(&log("😀 ab 😀 ab é ab").search("ab", true, false, true)),
            [(0, 3, 2), (0, 9, 2), (0, 14, 2)]
        );
    }

    #[test]
    fn a_search_stops_at_its_limit_and_says_there_is_more() {
        let mut log = GameLog::new();
        for _ in 0..3 {
            log.add_entry(LogEntry::plaintext("banana", LogEntrySourceKind::StdOut));
        }

        for regex in [false, true] {
            // Nine hits in all: three letters in each of three rows.
            let all = log.search_up_to("a", true, false, regex, 9);
            assert_eq!(all.results.len(), 9);
            assert!(!all.truncated, "regex: {regex}");

            let some = log.search_up_to("a", true, false, regex, 4);
            assert_eq!(
                hits(&some.results),
                [(0, 1, 1), (0, 3, 1), (0, 5, 1), (1, 1, 1)]
            );
            assert!(some.truncated, "regex: {regex}");
        }
    }

    /// Before, "error|" on this log gave a hit at every one of its 1.8
    /// million characters and found the position of each by reading its row
    /// again from the start; the search did not finish in nine minutes.
    #[tokio::test]
    async fn a_search_of_the_sample_log_is_answered() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../carbon_parsing/fixtures/log_example.log"
        );
        let (tx, rx) = watch::channel(GameLog::new());
        LogProcessor::new(LogEntrySourceKind::StdOut, &tx)
            .await
            .process_data(&std::fs::read(path).unwrap(), None)
            .await
            .unwrap();
        let log = rx.borrow();

        let plain = hits(&log.search("error", false, false, false));
        assert!(!plain.is_empty());
        assert_eq!(hits(&log.search("error|", false, false, true)), plain);
        assert_eq!(hits(&log.search("error", false, false, true)), plain);

        for regex in [false, true] {
            let letter = log.search_up_to("e", false, false, regex, MAX_SEARCH_RESULTS);
            assert_eq!(letter.results.len(), MAX_SEARCH_RESULTS);
            assert!(letter.truncated);
        }
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
mod tests {
    use super::*;

    fn log(message: &str) -> GameLog {
        let mut log = GameLog::new();
        log.add_entry(LogEntry::plaintext(message, LogEntrySourceKind::StdOut));
        log
    }

    #[test]
    fn search_regex_case_toggle() {
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
    fn search_original_utf16_spans() {
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
    fn search_literal_controls() {
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
    async fn history_play_before_logs_preserves_live_receiver() {
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
                .log
                .as_ref()
                .unwrap()
                .borrow()
                .search("later", true, false, false)
                .len(),
            1
        );
        let retained = *logs.keys().find(|log| **log != current).unwrap();
        drop(logs);
        assert!(
            !manager
                .open_log(retained, dir.path())
                .await
                .unwrap()
                .borrow()
                .search("retained", true, false, false)
                .is_empty()
        );
        assert!(std::fs::read(dir.path().join("2025-01-14_12-00-00.log")).is_ok());
    }

    #[tokio::test]
    async fn history_logs_first_and_current_only() {
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
    async fn history_concurrent_load_retry_and_registration_orders() {
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
        assert_eq!(
            manager.game_logs.read().await[&current]
                .log
                .as_ref()
                .unwrap()
                .borrow()
                .len(),
            1
        );
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
    fn search_matching_unicode_and_regex_syntax() {
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
    async fn history_bad_entry_does_not_hide_later_healthy_sessions() {
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
                .log
                .as_ref()
                .unwrap()
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
        let repaired = *logs
            .keys()
            .find(|log| **log != healthy_id && **log != live)
            .unwrap();
        drop(logs);
        assert!(
            !manager
                .open_log(repaired, dir.path())
                .await
                .unwrap()
                .borrow()
                .search("repaired", true, false, false)
                .is_empty()
        );
        assert!(!manager.load_log_history(id, dir.path()).await.unwrap());
    }

    #[tokio::test]
    async fn history_entry_enumeration_error_keeps_following_file() {
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

#[cfg(test)]
mod session_registry_tests {
    use super::*;

    const INSTANCE: InstanceId = InstanceId(17);

    fn at(day: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2025, 1, day, 12, 0, 0)
            .single()
            .unwrap()
    }

    fn file_of(day: u32) -> String {
        format!("2025-01-{day:02}_12-00-00.log")
    }

    /// A folder of past sessions, one for each day given, and a manager
    /// that has listed them.
    async fn listed(days: &[u32]) -> (tempfile::TempDir, InstanceManager) {
        let dir = tempfile::tempdir().unwrap();
        for day in days {
            std::fs::write(dir.path().join(file_of(*day)), format!("session {day}\n")).unwrap();
        }
        let manager = InstanceManager::new();
        manager
            .load_log_history(INSTANCE, dir.path())
            .await
            .unwrap();
        (dir, manager)
    }

    async fn id_of(manager: &InstanceManager, day: u32) -> GameLogId {
        *manager
            .game_logs
            .read()
            .await
            .iter()
            .find(|(_, record)| record.file_name() == file_of(day))
            .unwrap()
            .0
    }

    async fn in_memory(manager: &InstanceManager, id: GameLogId) -> bool {
        manager.game_logs.read().await[&id].log.is_some()
    }

    fn text(log: &watch::Receiver<GameLog>) -> String {
        let log = log.borrow();
        (0..log.len())
            .map(|index| log.get_entry(index).unwrap().message.clone())
            .collect()
    }

    #[tokio::test]
    async fn listing_past_sessions_reads_none_of_them() {
        let (_dir, manager) = listed(&[14, 15]).await;
        let logs = manager.game_logs.read().await;
        assert_eq!(logs.len(), 2);
        for record in logs.values() {
            assert!(record.log.is_none());
            assert!(record.on_disk);
            assert!(!record.is_active());
        }
    }

    #[tokio::test]
    async fn a_past_session_is_read_when_it_is_opened_and_stays_until_another_is() {
        let (dir, manager) = listed(&[14, 15]).await;
        let (first, second) = (id_of(&manager, 14).await, id_of(&manager, 15).await);

        let log = manager.open_log(first, dir.path()).await.unwrap();
        assert_eq!(text(&log), "session 14\n");
        assert!(log.has_changed().is_err(), "a past session is not active");
        assert!(in_memory(&manager, first).await);
        assert!(!in_memory(&manager, second).await);

        // Opened again (a search of it): the same log, not read again.
        std::fs::write(dir.path().join(file_of(14)), "changed on disk\n").unwrap();
        assert_eq!(
            text(&manager.open_log(first, dir.path()).await.unwrap()),
            "session 14\n"
        );

        let log = manager.open_log(second, dir.path()).await.unwrap();
        assert_eq!(text(&log), "session 15\n");
        assert!(in_memory(&manager, second).await);
        assert!(
            !in_memory(&manager, first).await,
            "the one before is let go"
        );

        // And read from its file again when it is gone back to.
        assert_eq!(
            text(&manager.open_log(first, dir.path()).await.unwrap()),
            "changed on disk\n"
        );
        assert!(!in_memory(&manager, second).await);
    }

    #[tokio::test]
    async fn the_same_session_asked_for_twice_at_once_is_one_log() {
        let (dir, manager) = listed(&[14]).await;
        let id = id_of(&manager, 14).await;
        let (stream, search) = tokio::join!(
            manager.open_log(id, dir.path()),
            manager.open_log(id, dir.path())
        );
        let (stream, search) = (stream.unwrap(), search.unwrap());
        assert_eq!(text(&stream), "session 14\n");
        assert!(stream.same_channel(&search));
    }

    #[tokio::test]
    async fn opening_a_session_never_lets_go_of_a_log_that_cannot_be_read_back() {
        let (dir, manager) = listed(&[14]).await;
        let past = id_of(&manager, 14).await;

        // A running game.
        let (running, running_tx) = manager.register_log(INSTANCE, Some(at(20))).await;
        running_tx.send_modify(|log| log.add_entry(LogEntry::system_message("running")));
        // A game that ended without a complete file.
        let (fileless, fileless_tx) = manager.register_log(InstanceId(18), Some(at(21))).await;
        fileless_tx.send_modify(|log| log.add_entry(LogEntry::system_message("only in memory")));
        let follower = manager.game_logs.read().await[&fileless]
            .log
            .clone()
            .unwrap();
        manager.end_log(fileless, fileless_tx, false).await;
        drop(follower);

        manager.open_log(past, dir.path()).await.unwrap();

        let logs = manager.game_logs.read().await;
        assert!(logs[&running].is_active());
        assert_eq!(text(logs[&running].log.as_ref().unwrap()), "running");
        assert_eq!(
            text(logs[&fileless].log.as_ref().unwrap()),
            "only in memory"
        );
        drop(logs);
        drop(running_tx);
    }

    #[tokio::test]
    async fn a_game_that_ends_unwatched_leaves_its_log_to_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let manager = InstanceManager::new();
        let (id, tx) = manager.register_log(INSTANCE, Some(at(14))).await;
        tx.send_modify(|log| log.add_entry(LogEntry::system_message("from memory")));
        std::fs::write(dir.path().join(file_of(14)), "from the file\n").unwrap();

        manager.end_log(id, tx, true).await;

        assert!(!in_memory(&manager, id).await);
        assert!(!manager.game_logs.read().await[&id].is_active());
        // Still listed, and read from its file when opened.
        assert_eq!(
            text(&manager.open_log(id, dir.path()).await.unwrap()),
            "from the file\n"
        );
    }

    #[tokio::test]
    async fn a_game_that_ends_while_watched_keeps_its_log_until_another_session_is_opened() {
        let (dir, manager) = listed(&[14]).await;
        let past = id_of(&manager, 14).await;
        let (id, tx) = manager.register_log(INSTANCE, Some(at(20))).await;
        tx.send_modify(|log| log.add_entry(LogEntry::system_message("watched")));
        // The Logs tab following the game.
        let (stream, _) = manager.held_log(id).await.unwrap();
        let stream = stream.unwrap();

        manager.end_log(id, tx, true).await;

        assert!(
            stream.has_changed().is_err(),
            "the stream sees the game end"
        );
        assert!(in_memory(&manager, id).await);
        // A search of the rows on screen still finds them where they are.
        let (searched, _) = manager.held_log(id).await.unwrap();
        assert_eq!(text(&searched.unwrap()), "watched");

        manager.open_log(past, dir.path()).await.unwrap();
        assert!(!in_memory(&manager, id).await);
    }

    #[tokio::test]
    async fn a_game_that_ends_without_a_complete_file_keeps_its_log() {
        let (dir, manager) = listed(&[14]).await;
        let past = id_of(&manager, 14).await;
        let (id, tx) = manager.register_log(INSTANCE, Some(at(20))).await;
        tx.send_modify(|log| log.add_entry(LogEntry::system_message("only in memory")));

        manager.end_log(id, tx, false).await;
        manager.open_log(past, dir.path()).await.unwrap();

        let (log, _) = manager.held_log(id).await.unwrap();
        assert_eq!(text(&log.unwrap()), "only in memory");
    }

    #[tokio::test]
    async fn a_session_whose_file_is_gone_is_missing_and_can_be_forgotten() {
        let (dir, manager) = listed(&[14, 15]).await;
        let (gone, kept) = (id_of(&manager, 14).await, id_of(&manager, 15).await);
        std::fs::remove_file(dir.path().join(file_of(14))).unwrap();

        assert!(matches!(
            manager.open_log(gone, dir.path()).await,
            Err(OpenLogError::Missing)
        ));
        // Still listed until the caller has made sure of the folder.
        assert!(manager.game_logs.read().await.contains_key(&gone));
        assert!(manager.forget_log_on_disk(gone).await);
        assert!(!manager.game_logs.read().await.contains_key(&gone));
        assert!(matches!(
            manager.open_log(gone, dir.path()).await,
            Err(OpenLogError::Unknown)
        ));

        // One that is in memory is not forgotten for its file.
        manager.open_log(kept, dir.path()).await.unwrap();
        assert!(!manager.forget_log_on_disk(kept).await);
        assert!(manager.game_logs.read().await.contains_key(&kept));
    }

    #[tokio::test]
    async fn a_file_that_cannot_be_read_leaves_its_session_listed() {
        let (dir, manager) = listed(&[14]).await;
        let id = id_of(&manager, 14).await;
        std::fs::remove_file(dir.path().join(file_of(14))).unwrap();
        std::fs::create_dir(dir.path().join(file_of(14))).unwrap();

        assert!(matches!(
            manager.open_log(id, dir.path()).await,
            Err(OpenLogError::Unreadable(_))
        ));
        assert!(manager.game_logs.read().await.contains_key(&id));
        assert!(!in_memory(&manager, id).await);
    }

    #[tokio::test]
    async fn sessions_rotated_away_leave_the_list_and_the_others_stay() {
        let (dir, manager) = listed(&[10, 11, 12]).await;
        let (rotated, opened, kept) = (
            id_of(&manager, 10).await,
            id_of(&manager, 11).await,
            id_of(&manager, 12).await,
        );
        // Another instance's session of the same time, in another folder.
        let other_dir = tempfile::tempdir().unwrap();
        std::fs::write(other_dir.path().join(file_of(10)), "other\n").unwrap();
        manager
            .load_log_history(InstanceId(18), other_dir.path())
            .await
            .unwrap();
        // The game being started, whose file is not written yet, and one
        // that ended without a complete file.
        let (starting, starting_tx) = manager.register_log(INSTANCE, Some(at(20))).await;
        let (fileless, fileless_tx) = manager.register_log(INSTANCE, Some(at(19))).await;
        manager.end_log(fileless, fileless_tx, false).await;

        // Nothing is gone: nothing changes.
        assert!(!manager.forget_logs_without_file(INSTANCE, dir.path()).await);
        assert_eq!(manager.game_logs.read().await.len(), 6);

        // The oldest is rotated away, and so is one that is being looked at.
        manager.open_log(opened, dir.path()).await.unwrap();
        std::fs::remove_file(dir.path().join(file_of(10))).unwrap();
        std::fs::remove_file(dir.path().join(file_of(11))).unwrap();

        assert!(manager.forget_logs_without_file(INSTANCE, dir.path()).await);
        let logs = manager.game_logs.read().await;
        assert!(!logs.contains_key(&rotated));
        assert!(!logs.contains_key(&opened));
        assert!(logs.contains_key(&kept));
        assert!(logs.contains_key(&starting));
        assert!(logs.contains_key(&fileless));
        assert_eq!(
            logs.values()
                .filter(|record| record.instance_id == InstanceId(18))
                .count(),
            1
        );
        drop(logs);
        drop(starting_tx);
    }

    #[tokio::test]
    async fn a_game_started_in_the_second_of_a_listed_session_takes_its_record_over() {
        let (dir, manager) = listed(&[14]).await;
        let past = id_of(&manager, 14).await;

        let (id, tx) = manager.register_log(INSTANCE, Some(at(14))).await;
        assert_eq!(id, past);
        assert_eq!(manager.game_logs.read().await.len(), 1);
        let record_is_active = manager.game_logs.read().await[&id].is_active();
        assert!(record_is_active);
        assert!(!manager.game_logs.read().await[&id].on_disk);
        // Opening it gives the running game's log, not the file's.
        tx.send_modify(|log| log.add_entry(LogEntry::system_message("live")));
        assert_eq!(
            text(&manager.open_log(id, dir.path()).await.unwrap()),
            "live"
        );
    }
}
