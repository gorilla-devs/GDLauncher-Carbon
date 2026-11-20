use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
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
    #[error("event ended before its structure completed")]
    UnexpectedEventEnd,
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

    /// Bytes currently buffered and not yet resolved into a [`ParsedItem`].
    ///
    /// Complete bad events and definitely invalid plaintext advance the buffer
    /// before returning an error. Genuinely partial XML or UTF-8 is retained and
    /// reported as `Partial`. This length lets callers verify recovery progress.
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
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
            // Only ASCII keys (logger/timestamp/level/thread) are acted on below, so an
            // attribute name that is not valid UTF-8 can never be one we care about; skip it
            // instead of panicking on a malformed or custom log4j layout.
            let Ok(key) = std::str::from_utf8(attr.key.as_ref()) else {
                continue;
            };
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

    /// Parses a single `<log4j:Event>` element out of `data`, which the tag-balance
    /// scan in `parse_next` has already confirmed spans exactly one complete
    /// element (matched start/end tags, regardless of their names). Returns the
    /// parsed entry, or the specific error that makes this event unparseable (bad
    /// timestamp, empty logger, non-UTF8 message, missing `<log4j:Message>`).
    ///
    /// Callers must advance the real buffer past `data`'s length regardless of
    /// the result: a bad event's bytes are a property of those bytes, not of how
    /// much more data arrives later, so leaving them in the buffer would just
    /// reproduce the same error on every future call.
    fn parse_complete_event(data: &[u8]) -> Result<LogEntry, ParserError> {
        let mut reader = Reader::from_reader(data);
        reader.config_mut().check_end_names = false;
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();

        let start = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(start)) if start.name().as_ref() == b"log4j:Event" => start,
            // `data` is exactly the span the caller's tag-balance scan already
            // confirmed opens with a `<log4j:Event>` start tag; reading the
            // identical bytes again with the same reader config cannot disagree.
            _ => return Err(ParserError::UnexpectedEventEnd),
        };

        let mut entry = Self::parse_attributes(&start)?;
        let mut message = None;
        let mut throwable = Vec::new();
        let mut depth = 1;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => match e.name().as_ref() {
                    b"log4j:Message" => {
                        let text = Self::read_element_text(&mut reader, b"log4j:Message")?;
                        message = Some(String::from_utf8(text)?);
                    }
                    b"log4j:Throwable" => {
                        throwable.extend(Self::read_element_text(&mut reader, b"log4j:Throwable")?);
                    }
                    _ => depth += 1,
                },
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    if depth == 0 && e.name().as_ref() == b"log4j:Event" {
                        let Some(message) = message else {
                            return Err(ParserError::MissingAttribute("message".to_string()));
                        };
                        entry.message = Self::with_throwable(message, &throwable);
                        return Ok(entry);
                    }
                }
                Ok(Event::Eof) => return Err(ParserError::UnexpectedEventEnd),
                Err(e) => return Err(e.into()),
                _ => {}
            }
        }
    }

    /// Reads the text of the element whose start tag was just read, up to and
    /// including its end tag. Its text can come in several pieces: log4j
    /// splits a CDATA section wherever the text itself holds `]]>`.
    fn read_element_text(reader: &mut Reader<&[u8]>, name: &[u8]) -> Result<Vec<u8>, ParserError> {
        let mut buf = Vec::new();
        let mut text = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Text(e)) => text.extend_from_slice(&e.into_inner()),
                Ok(Event::CData(e)) => text.extend_from_slice(&e.into_inner()),
                Ok(Event::End(ref e)) if e.name().as_ref() == name => return Ok(text),
                Ok(Event::Eof) => return Err(ParserError::UnexpectedEventEnd),
                Err(e) => return Err(e.into()),
                _ => {}
            }
        }
    }

    /// The message with the event's exception under it, the way the game's
    /// own log file prints the two: log4j carries the stack trace of a logged
    /// exception in a `<log4j:Throwable>` element next to the message, and it
    /// is the part of an error that says why it happened.
    ///
    /// A trace that is not valid UTF-8 is kept with the bad bytes replaced
    /// rather than costing the whole event.
    fn with_throwable(message: String, throwable: &[u8]) -> String {
        let throwable = String::from_utf8_lossy(throwable);
        let throwable = throwable.trim_end();
        if throwable.is_empty() {
            return message;
        }

        let message = message.trim_end_matches(['\r', '\n']);
        if message.is_empty() {
            return throwable.to_owned();
        }
        format!("{message}\n{throwable}")
    }

    /// Attempts to parse the next item from the buffer.
    ///
    /// Complete malformed events and definite invalid plaintext are consumed;
    /// incomplete event syntax and trailing UTF-8 code points remain buffered.
    pub fn parse_next(&mut self) -> Result<Option<ParsedItem>, ParserError> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        // Skip pure whitespace at the start
        if self.buffer.iter().all(|&c| c.is_ascii_whitespace()) {
            self.buffer.clear();
            return Ok(None);
        }

        // First try to find a complete log4j:Event, recording exactly how many
        // bytes (from the start of the buffer) it spans so the buffer can always
        // be advanced past it below, regardless of whether the detailed
        // `parse_complete_event` pass further down succeeds. Without this, a
        // complete-but-unparseable event (bad timestamp, empty logger, non-UTF8
        // message, missing <log4j:Message>) would error out of `parse_next`
        // before the buffer was ever drained, so the same bytes would be
        // re-read and re-error on every future call: the live log freezes and
        // the buffer grows without bound for the rest of the session.
        let complete_event_len: Option<usize> = {
            let mut reader = Reader::from_reader(&self.buffer[..]);
            reader.config_mut().check_end_names = false;
            reader.config_mut().trim_text(false);
            let mut test_buf = Vec::new();

            match reader.read_event_into(&mut test_buf) {
                Ok(Event::Text(e)) => {
                    if e.as_ref().iter().all(u8::is_ascii_whitespace) {
                        let consumed = reader.buffer_position();
                        if consumed > 0 {
                            self.buffer.drain(..consumed as usize);
                            return self.parse_next();
                        }
                        None
                    } else {
                        None
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
                    (depth == 0).then(|| reader.buffer_position() as usize)
                }
                _ => None,
            }
        };

        if let Some(event_len) = complete_event_len {
            let result = Self::parse_complete_event(&self.buffer[..event_len]);

            // This event's bytes are fully consumed either way -- see the doc
            // comment on `parse_complete_event` for why an error must still drain.
            self.buffer.drain(..event_len);
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

            result.map(|entry| Some(ParsedItem::LogEntry(entry)))
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
                            return self.take_plaintext(slice_start, false);
                        }
                        return Ok(Some(ParsedItem::Partial(self.buffer.clone())));
                    }
                    start = slice_start + 1;
                } else {
                    break;
                }
            }

            self.take_plaintext(self.buffer.len(), true)
        }
    }

    // Preserve a possibly split trailing UTF-8 code point. Definite malformed
    // plaintext is consumed up to (and never including) the next event boundary.
    fn take_plaintext(
        &mut self,
        length: usize,
        may_be_partial: bool,
    ) -> Result<Option<ParsedItem>, ParserError> {
        match String::from_utf8(self.buffer[..length].to_vec()) {
            Ok(text) => {
                self.buffer.drain(..length);
                if text.chars().all(|c| c.is_ascii_whitespace()) {
                    self.parse_next()
                } else {
                    Ok(Some(ParsedItem::PlainText(text)))
                }
            }
            Err(error) if may_be_partial && error.utf8_error().error_len().is_none() => {
                let valid = error.utf8_error().valid_up_to();
                if valid > 0 {
                    self.take_plaintext(valid, false)
                } else {
                    Ok(Some(ParsedItem::Partial(self.buffer.clone())))
                }
            }
            Err(error) => {
                self.buffer.drain(..length);
                Err(error.into())
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

#[cfg(test)]
mod task06_tests {
    use super::*;
    const EVENT: &[u8] = br#"<log4j:Event logger="test" timestamp="1" level="INFO" thread="main"><log4j:Message><![CDATA[A & B]]></log4j:Message></log4j:Event>"#;
    #[test]
    fn task06_literal_ampersand_then_event() {
        let mut p = LogParser::new();
        for text in ["A & B\n", "ordinary\n", "&amp;\n"] {
            p.feed(text.as_bytes());
            assert_eq!(
                p.parse_next().unwrap(),
                Some(ParsedItem::PlainText(text.into()))
            );
        }
        p.feed(EVENT);
        assert!(
            matches!(p.parse_next().unwrap(), Some(ParsedItem::LogEntry(e)) if e.message == "A & B")
        );
        assert_eq!(p.buffered_len(), 0);
    }
    #[test]
    fn task06_partial_utf8_and_invalid_prefix() {
        let mut p = LogParser::new();
        p.feed(&[0xe2, 0x82]);
        assert!(matches!(
            p.parse_next().unwrap(),
            Some(ParsedItem::Partial(_))
        ));
        p.feed(&[0xac]);
        assert_eq!(
            p.parse_next().unwrap(),
            Some(ParsedItem::PlainText("€".into()))
        );
        p.feed(&[0xff, b'\n']);
        p.feed(EVENT);
        assert!(p.parse_next().is_err());
        assert!(matches!(
            p.parse_next().unwrap(),
            Some(ParsedItem::LogEntry(_))
        ));
    }
}

#[cfg(test)]
mod throwable_tests {
    use super::*;

    fn parse(event: &str) -> LogEntry {
        let mut parser = LogParser::new();
        parser.feed(event.as_bytes());
        match parser.parse_next().unwrap() {
            Some(ParsedItem::LogEntry(entry)) => {
                assert_eq!(parser.buffered_len(), 0);
                entry
            }
            other => panic!("expected a log entry, got {other:?}"),
        }
    }

    fn event(children: &str) -> String {
        format!(
            r#"<log4j:Event logger="test" timestamp="1" level="ERROR" thread="main">{children}</log4j:Event>"#
        )
    }

    #[test]
    fn the_stack_trace_follows_the_message() {
        let entry = parse(&event(
            "\n  <log4j:Message><![CDATA[Failed to load mod]]></log4j:Message>\n  \
             <log4j:Throwable><![CDATA[java.lang.IllegalStateException: boom\n\
             \tat a.B.c(B.java:1)\n\tat a.D.e(D.java:2)\n]]></log4j:Throwable>\n",
        ));

        assert_eq!(
            entry.message,
            "Failed to load mod\njava.lang.IllegalStateException: boom\n\tat a.B.c(B.java:1)\n\tat a.D.e(D.java:2)"
        );
    }

    #[test]
    fn an_event_without_an_exception_keeps_its_message_as_it_is() {
        let entry = parse(&event(
            "<log4j:Message><![CDATA[two lines\n]]></log4j:Message>",
        ));
        assert_eq!(entry.message, "two lines\n");

        let entry = parse(&event(
            "<log4j:Message><![CDATA[plain]]></log4j:Message><log4j:Throwable><![CDATA[\n]]></log4j:Throwable>",
        ));
        assert_eq!(entry.message, "plain");
    }

    #[test]
    fn the_trace_is_read_wherever_it_stands_in_the_event() {
        let entry = parse(&event(
            "<log4j:Throwable><![CDATA[java.lang.Error: first]]></log4j:Throwable>\
             <log4j:Message><![CDATA[message]]></log4j:Message>",
        ));
        assert_eq!(entry.message, "message\njava.lang.Error: first");
    }

    #[test]
    fn a_trace_split_over_several_cdata_sections_is_joined() {
        // How log4j writes a text that holds "]]>" itself.
        let entry = parse(&event(
            "<log4j:Message><![CDATA[message]]></log4j:Message>\
             <log4j:Throwable><![CDATA[java.lang.Error: a]]]]><![CDATA[>b\n\tat x.Y.z(Y.java:3)]]></log4j:Throwable>",
        ));
        assert_eq!(
            entry.message,
            "message\njava.lang.Error: a]]>b\n\tat x.Y.z(Y.java:3)"
        );
    }

    #[test]
    fn an_exception_logged_without_a_message_is_shown_alone() {
        let entry = parse(&event(
            "<log4j:Message><![CDATA[]]></log4j:Message>\
             <log4j:Throwable><![CDATA[java.lang.Error: alone]]></log4j:Throwable>",
        ));
        assert_eq!(entry.message, "java.lang.Error: alone");
    }

    #[test]
    fn a_trace_with_broken_text_does_not_cost_the_event() {
        let mut bytes = event("<log4j:Message><![CDATA[message]]></log4j:Message><log4j:Throwable><![CDATA[bad X trace]]></log4j:Throwable>").into_bytes();
        let at = bytes.iter().position(|&b| b == b'X').unwrap();
        bytes[at] = 0xff;

        let mut parser = LogParser::new();
        parser.feed(&bytes);
        let Some(ParsedItem::LogEntry(entry)) = parser.parse_next().unwrap() else {
            panic!("expected a log entry");
        };
        assert_eq!(entry.message, "message\nbad \u{fffd} trace");
    }

    #[test]
    fn an_event_without_a_message_is_still_refused() {
        let mut parser = LogParser::new();
        parser.feed(
            event("<log4j:Throwable><![CDATA[java.lang.Error: x]]></log4j:Throwable>").as_bytes(),
        );
        assert!(parser.parse_next().is_err());
        assert_eq!(parser.buffered_len(), 0);
    }

    #[test]
    fn every_trace_of_the_sample_log_reaches_its_entry() {
        let sample = include_bytes!("../fixtures/log_example.log");
        let traces = sample
            .windows(b"<log4j:Throwable>".len())
            .filter(|window| window == b"<log4j:Throwable>")
            .count();
        assert_eq!(traces, 9);

        let mut parser = LogParser::new();
        parser.feed(sample);
        let with_trace = parser
            .parse_available()
            .unwrap()
            .into_iter()
            .filter(|item| {
                matches!(item, ParsedItem::LogEntry(entry) if entry.message.contains("\n\tat "))
            })
            .count();
        assert_eq!(with_trace, traces);
    }
}
