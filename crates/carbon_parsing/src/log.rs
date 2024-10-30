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

#[derive(Debug)]
pub enum ParsedItem {
    LogEntry(LogEntry),
    PlainText(String),
    Partial(Vec<u8>),
}

pub struct LogParser {
    buffer: Vec<u8>,
    partial_data: Vec<u8>,
    tag_buffer: Vec<u8>,
}

impl LogParser {
    /// Creates a new LogParser instance.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            partial_data: Vec::new(),
            tag_buffer: Vec::new(),
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
    fn parse_log_level(level: &str) -> Result<LogEntryLevel, ParserError> {
        match level.trim().to_uppercase().as_str() {
            "TRACE" => Ok(LogEntryLevel::Trace),
            "DEBUG" => Ok(LogEntryLevel::Debug),
            "INFO" => Ok(LogEntryLevel::Info),
            "WARN" => Ok(LogEntryLevel::Warn),
            "ERROR" => Ok(LogEntryLevel::Error),
            _ => Err(ParserError::InvalidLogLevel(level.to_string())),
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
                "level" => entry.level = Self::parse_log_level(&value)?,
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

        let mut reader = Reader::from_reader(&self.buffer[..]);
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
                                loop {
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
                                            entry.message = String::from_utf8(message_buf.clone())?;
                                            found_message = true;
                                            depth -= 1;
                                            break;
                                        }
                                        Ok(Event::Eof) => {
                                            self.partial_data = self.buffer.clone();
                                            return Ok(Some(ParsedItem::Partial(
                                                self.buffer.clone(),
                                            )));
                                        }
                                        Err(_) => {
                                            self.partial_data = self.buffer.clone();
                                            return Ok(Some(ParsedItem::Partial(
                                                self.buffer.clone(),
                                            )));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        Ok(Event::End(ref e)) => {
                            depth -= 1;
                            if depth == 0 && e.name().as_ref() == b"log4j:Event" {
                                if found_message {
                                    let consumed = reader.buffer_position();
                                    if consumed > 0 && consumed <= self.buffer.len() as u64 {
                                        let remaining = self.buffer.split_off(consumed as usize);
                                        self.buffer = remaining;
                                    }
                                    return Ok(Some(ParsedItem::LogEntry(entry)));
                                }
                                return Err(ParserError::MissingAttribute("message".to_string()));
                            }
                        }
                        Ok(Event::Eof) => {
                            self.partial_data = self.buffer.clone();
                            return Ok(Some(ParsedItem::Partial(self.buffer.clone())));
                        }
                        Err(_) => {
                            self.partial_data = self.buffer.clone();
                            return Ok(Some(ParsedItem::Partial(self.buffer.clone())));
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape()?.into_owned();
                let consumed = reader.buffer_position();
                if consumed > 0 && consumed <= self.buffer.len() as u64 {
                    self.buffer.drain(..consumed as usize);
                }
                if !text.trim().is_empty() {
                    return Ok(Some(ParsedItem::PlainText(text)));
                }
                self.parse_next()
            }
            Ok(Event::Eof) => {
                if !self.buffer.is_empty() {
                    let text = String::from_utf8(self.buffer.clone())?;
                    self.buffer.clear();
                    if !text.trim().is_empty() {
                        return Ok(Some(ParsedItem::PlainText(text)));
                    }
                }
                Ok(None)
            }
            Err(e) => {
                if let Some(pos) = self.buffer.windows(1).position(|w| w[0] == b'<') {
                    let text = if pos > 0 {
                        let text_bytes = self.buffer[..pos].to_vec();
                        self.buffer.drain(..pos);
                        String::from_utf8(text_bytes)?
                    } else {
                        String::new()
                    };

                    if !text.trim().is_empty() {
                        return Ok(Some(ParsedItem::PlainText(text)));
                    }

                    // Check if it's a potential log4j tag
                    let remaining = &self.buffer[..];
                    if remaining.starts_with(b"<log") {
                        if remaining.len() < 4 {
                            self.partial_data = self.buffer.clone();
                            return Ok(Some(ParsedItem::Partial(self.buffer.clone())));
                        }

                        if remaining.len() >= 10 && !remaining[4..10].starts_with(b"4j:Eve") {
                            let text = String::from_utf8(self.buffer.clone())?;
                            self.buffer.clear();
                            return Ok(Some(ParsedItem::PlainText(text)));
                        }

                        self.partial_data = self.buffer.clone();
                        return Ok(Some(ParsedItem::Partial(self.buffer.clone())));
                    }

                    // Not a log4j tag, treat as plain text
                    let text = String::from_utf8(self.buffer.clone())?;
                    self.buffer.clear();
                    return Ok(Some(ParsedItem::PlainText(text)));
                }

                // If there's an error parsing and we have data,
                // treat it as plain text
                let text = String::from_utf8(self.buffer.clone())?;
                self.buffer.clear();
                if !text.trim().is_empty() {
                    return Ok(Some(ParsedItem::PlainText(text)));
                }
                Ok(None)
            }
            Ok(e) => {
                let text = String::from_utf8(e.to_vec())?;
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
