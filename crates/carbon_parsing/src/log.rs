use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::io::BufReader;
use thiserror::Error;

/// Represents a parsed log4j message.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LogEntry {
    /// The name of the logger.
    pub logger: String,
    /// The log level of the entry.
    pub level: LogEntryLevel,
    /// The time the event was logged.
    pub timestamp: u64,
    /// The name of the thread.
    pub thread_name: String,
    /// The log message.
    pub message: String,
}

/// The log level of the log entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogEntryLevel {
    /// `TRACE` log level.
    Trace,
    /// `DEBUG` log level.
    Debug,
    /// `INFO` log level.
    Info,
    /// `WARN` log level.
    Warn,
    /// `ERROR` log level.
    Error,
    /// `FATAL` log level.
    Fatal,
    /// `UNKNOWN` log level.
    Unknown,
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("XML parsing error: {0}")]
    XmlError(#[from] quick_xml::Error),
    #[error("Invalid timestamp: {0}")]
    TimestampError(#[from] std::num::ParseIntError),
    #[error("Missing required attribute: {0}")]
    MissingAttribute(String),
    #[error("Invalid log level: {0}")]
    InvalidLogLevel(String),
    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Attribute error: {0}")]
    AttrError(#[from] quick_xml::events::attributes::AttrError),
}

#[derive(Debug, PartialEq)]
pub enum ParsedItem {
    LogEntry(LogEntry),
    PlainText(String),
    Partial(Vec<u8>),
}

pub struct LogParser {
    buffer: Vec<u8>,
    partial_data: Vec<u8>,
}

impl LogParser {
    /// Creates a new LogParser instance.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            partial_data: Vec::new(),
        }
    }

    /// Feeds new data into the parser.
    pub fn feed(&mut self, data: &[u8]) {
        if !self.partial_data.is_empty() {
            // If we have partial data from a previous parse, prepend it
            let mut new_buffer = self.partial_data.clone();
            new_buffer.extend_from_slice(data);
            self.buffer = new_buffer;
            self.partial_data.clear();
        } else {
            self.buffer.extend_from_slice(data);
        }
    }

    /// Parses a log level string into the LogEntryLevel enum.
    fn parse_log_level(level: &str) -> LogEntryLevel {
        match level.trim().to_uppercase().as_str() {
            "TRACE" => LogEntryLevel::Trace,
            "DEBUG" => LogEntryLevel::Debug,
            "INFO" => LogEntryLevel::Info,
            "WARN" => LogEntryLevel::Warn,
            "ERROR" => LogEntryLevel::Error,
            "FATAL" => LogEntryLevel::Fatal,
            _ => LogEntryLevel::Unknown,
        }
    }

    /// Parses XML attributes into a LogEntry struct.
    fn parse_attributes(element: &BytesStart) -> Result<LogEntry, ParserError> {
        let mut entry = LogEntry {
            logger: String::new(),
            level: LogEntryLevel::Info,
            timestamp: 0,
            thread_name: String::new(),
            message: String::new(),
        };

        for attr in element.attributes() {
            let attr = attr?;
            let key = std::str::from_utf8(attr.key.as_ref()).unwrap();
            let value = attr.unescape_value()?.into_owned();

            match key {
                "logger" => {
                    entry.logger = value.trim().to_string();
                    if entry.logger.is_empty() {
                        return Err(ParserError::MissingAttribute("logger".to_string()));
                    }
                }
                "timestamp" => {
                    if value.trim().is_empty() {
                        return Err(ParserError::MissingAttribute("timestamp".to_string()));
                    }
                    entry.timestamp = value.trim().parse()?
                }
                "level" => entry.level = Self::parse_log_level(&value),
                "thread" => entry.thread_name = value.trim().to_string(),
                _ => {}
            }
        }

        if entry.logger.is_empty() {
            return Err(ParserError::MissingAttribute("logger".to_string()));
        }

        Ok(entry)
    }

    /// Attempts to parse the next item from the buffer.
    pub fn parse_next(&mut self) -> Result<Option<ParsedItem>, ParserError> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        // Skip pure whitespace at the start
        if self.buffer.iter().all(|&c| c.is_ascii_whitespace()) {
            self.buffer.clear();
            return Ok(None);
        }

        // First try to find a complete log4j:Event
        let is_complete_log4j = {
            let mut reader = Reader::from_reader(&self.buffer[..]);
            reader.config_mut().check_end_names = false;
            reader.config_mut().trim_text(false);
            let mut test_buf = Vec::new();

            match reader.read_event_into(&mut test_buf) {
                Ok(Event::Text(e)) => {
                    let text = e.unescape().map_err(|e| ParserError::XmlError(e.into()))?;
                    if text.chars().all(|c| c.is_ascii_whitespace()) {
                        let consumed = reader.buffer_position();
                        if consumed > 0 {
                            self.buffer.drain(..consumed as usize);
                            return self.parse_next();
                        }
                        false
                    } else {
                        false
                    }
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"log4j:Event" => {
                    // Found start tag, now look for the end tag
                    let mut depth = 1;
                    while depth > 0 {
                        match reader.read_event_into(&mut test_buf) {
                            Ok(Event::Start(_)) => depth += 1,
                            Ok(Event::End(_)) => {
                                depth -= 1;
                            }
                            Ok(Event::Eof) | Err(_) => break,
                            _ => {}
                        }
                    }
                    depth == 0
                }
                _ => false,
            }
        };

        if is_complete_log4j {
            let mut reader = Reader::from_reader(&self.buffer[..]);
            reader.config_mut().check_end_names = false;
            reader.config_mut().trim_text(false);
            let mut buf = Vec::new();

            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"log4j:Event" => {
                    let mut entry = Self::parse_attributes(e)?;
                    let mut found_message = false;
                    let mut depth = 1;

                    loop {
                        match reader.read_event_into(&mut buf) {
                            Ok(Event::Start(ref e)) => {
                                depth += 1;
                                if e.name().as_ref() == b"log4j:Message" {
                                    let mut message_buf = Vec::new();
                                    let mut message_complete = false;

                                    while !message_complete {
                                        match reader.read_event_into(&mut buf) {
                                            Ok(Event::Text(e)) => {
                                                message_buf.extend_from_slice(&e.into_inner());
                                            }
                                            Ok(Event::CData(e)) => {
                                                message_buf.extend_from_slice(&e.into_inner());
                                            }
                                            Ok(Event::End(ref e))
                                                if e.name().as_ref() == b"log4j:Message" =>
                                            {
                                                message_complete = true;
                                            }
                                            Ok(Event::Eof) => return Ok(None),
                                            Err(_) => return Ok(None),
                                            _ => {}
                                        }
                                    }

                                    entry.message = String::from_utf8(message_buf)?;
                                    found_message = true;
                                    depth -= 1;
                                }
                            }
                            Ok(Event::End(ref e)) => {
                                depth -= 1;
                                if depth == 0 && e.name().as_ref() == b"log4j:Event" {
                                    if found_message {
                                        let consumed = reader.buffer_position();
                                        if consumed > 0 && consumed <= self.buffer.len() as u64 {
                                            self.buffer = self.buffer[consumed as usize..].to_vec();
                                            // Skip any pure whitespace after the event
                                            while !self.buffer.is_empty()
                                                && self
                                                    .buffer
                                                    .iter()
                                                    .take_while(|&&c| c.is_ascii_whitespace())
                                                    .count()
                                                    == self.buffer.len()
                                            {
                                                self.buffer.clear();
                                            }
                                        }
                                        return Ok(Some(ParsedItem::LogEntry(entry)));
                                    }
                                    return Err(ParserError::MissingAttribute(
                                        "message".to_string(),
                                    ));
                                }
                            }
                            Ok(Event::Eof) => return Ok(None),
                            Err(_) => return Ok(None),
                            _ => {}
                        }
                    }
                }
                _ => unreachable!("We already verified this was a log4j:Event"),
            }
        } else {
            // Check if what we have could be a start of a log4j event
            if is_potential_log4j_start(&self.buffer) {
                return Ok(Some(ParsedItem::Partial(self.buffer.clone())));
            }

            // Look for the next potential log4j start
            let mut start = 0;
            while start < self.buffer.len() {
                if let Some(pos) = self.buffer[start..].iter().position(|&c| c == b'<') {
                    let slice_start = start + pos;
                    let slice = &self.buffer[slice_start..];

                    if is_potential_log4j_start(slice) {
                        if slice_start > 0 {
                            let text = String::from_utf8(self.buffer[..slice_start].to_vec())?;
                            self.buffer = self.buffer[slice_start..].to_vec();
                            if !text.chars().all(|c| c.is_ascii_whitespace()) {
                                return Ok(Some(ParsedItem::PlainText(text)));
                            }
                            continue;
                        }
                        return Ok(Some(ParsedItem::Partial(self.buffer.clone())));
                    }
                    start = slice_start + 1;
                } else {
                    break;
                }
            }

            // No potential log4j events found, return all as plain text
            let text = String::from_utf8(self.buffer.clone())?;
            self.buffer.clear();
            if text.chars().all(|c| c.is_ascii_whitespace()) {
                Ok(None)
            } else {
                Ok(Some(ParsedItem::PlainText(text)))
            }
        }
    }

    /// Parses all available complete items from the buffer.
    pub fn parse_available(&mut self) -> Result<Vec<ParsedItem>, ParserError> {
        let mut items = Vec::new();
        while let Some(item) = self.parse_next()? {
            match item {
                ParsedItem::Partial(_) => break,
                item => items.push(item),
            }
        }
        Ok(items)
    }
}

/// Helper function to check if a buffer could be the start of a log4j event
fn is_potential_log4j_start(buffer: &[u8]) -> bool {
    let target = b"<log4j:Event";
    if buffer.is_empty() || buffer[0] != b'<' {
        return false;
    }

    let buffer_lower: Vec<u8> = buffer.iter().map(|&c| c.to_ascii_lowercase()).collect();

    let target_lower: Vec<u8> = target.iter().map(|&c| c.to_ascii_lowercase()).collect();

    target_lower.starts_with(&buffer_lower) || buffer_lower.starts_with(&target_lower)
}
