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
    /// How much of the front of `buffer` is already resolved. Moving this on
    /// costs nothing, where removing the bytes would move the whole rest of
    /// the buffer once per item: a log fed in one piece then takes time that
    /// grows with the square of its size.
    consumed: usize,
}

impl LogParser {
    /// Creates a new LogParser instance.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            consumed: 0,
        }
    }

    /// The bytes not yet resolved into a [`ParsedItem`].
    fn pending(&self) -> &[u8] {
        &self.buffer[self.consumed..]
    }

    /// Marks the first `length` pending bytes as resolved.
    fn consume(&mut self, length: usize) {
        self.consumed += length;
        if self.consumed == self.buffer.len() {
            self.buffer.clear();
            self.consumed = 0;
        }
    }

    /// Bytes currently buffered and not yet resolved into a [`ParsedItem`].
    ///
    /// Complete bad events and definitely invalid plaintext advance the buffer
    /// before returning an error. Genuinely partial XML or UTF-8 is retained and
    /// reported as `Partial`. This length lets callers verify recovery progress.
    pub fn buffered_len(&self) -> usize {
        self.pending().len()
    }

    /// Feeds new data into the parser.
    pub fn feed(&mut self, data: &[u8]) {
        // The one place the resolved bytes are really removed: once per piece
        // of data, however many items the piece before it held.
        if self.consumed > 0 {
            self.buffer.drain(..self.consumed);
            self.consumed = 0;
        }
        self.buffer.extend_from_slice(data);
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
        if self.pending().is_empty() {
            return Ok(None);
        }

        // Skip pure whitespace at the start
        if self.pending().iter().all(|&c| c.is_ascii_whitespace()) {
            self.consume(self.pending().len());
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
            let mut reader = Reader::from_reader(self.pending());
            reader.config_mut().check_end_names = false;
            reader.config_mut().trim_text(false);
            let mut test_buf = Vec::new();

            match reader.read_event_into(&mut test_buf) {
                Ok(Event::Text(e)) => {
                    if e.as_ref().iter().all(u8::is_ascii_whitespace) {
                        let consumed = reader.buffer_position();
                        if consumed > 0 {
                            self.consume(consumed as usize);
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
            let result = Self::parse_complete_event(&self.pending()[..event_len]);

            // This event's bytes are fully consumed either way -- see the doc
            // comment on `parse_complete_event` for why an error must still drain.
            self.consume(event_len);
            // Skip any pure whitespace after the event
            if self.pending().iter().all(|&c| c.is_ascii_whitespace()) {
                self.consume(self.pending().len());
            }

            result.map(|entry| Some(ParsedItem::LogEntry(entry)))
        } else {
            // Check if what we have could be a start of a log4j event
            let pending = self.pending();
            if is_potential_log4j_start(pending) {
                return Ok(Some(ParsedItem::Partial(pending.to_vec())));
            }

            // Look for the next potential log4j start
            let mut start = 0;
            while start < pending.len() {
                if let Some(pos) = pending[start..].iter().position(|&c| c == b'<') {
                    let slice_start = start + pos;
                    let slice = &pending[slice_start..];

                    if is_potential_log4j_start(slice) {
                        if slice_start > 0 {
                            return self.take_plaintext(slice_start, false);
                        }
                        return Ok(Some(ParsedItem::Partial(pending.to_vec())));
                    }
                    start = slice_start + 1;
                } else {
                    break;
                }
            }

            self.take_plaintext(pending.len(), true)
        }
    }

    // Preserve a possibly split trailing UTF-8 code point. Definite malformed
    // plaintext is consumed up to (and never including) the next event boundary.
    fn take_plaintext(
        &mut self,
        length: usize,
        may_be_partial: bool,
    ) -> Result<Option<ParsedItem>, ParserError> {
        match String::from_utf8(self.pending()[..length].to_vec()) {
            Ok(text) => {
                self.consume(length);
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
                    Ok(Some(ParsedItem::Partial(self.pending().to_vec())))
                }
            }
            Err(error) => {
                self.consume(length);
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
///
/// True for the start tag in any letter case, and for a shorter buffer that
/// is the beginning of one. Only the tag's own length is looked at: this runs
/// at every `<` of plain text, with the whole rest of the log behind it.
fn is_potential_log4j_start(buffer: &[u8]) -> bool {
    let target = b"<log4j:Event";
    if buffer.is_empty() || buffer[0] != b'<' {
        return false;
    }

    let compared = buffer.len().min(target.len());
    buffer[..compared].eq_ignore_ascii_case(&target[..compared])
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

#[cfg(test)]
mod read_position_tests {
    use super::*;
    use std::time::{Duration, Instant};

    const SAMPLE: &[u8] = include_bytes!("../fixtures/log_example.log");
    const EVENT: &[u8] = br#"<log4j:Event logger="test" timestamp="1" level="INFO" thread="main"><log4j:Message><![CDATA[hello]]></log4j:Message></log4j:Event>"#;

    /// Everything the parser resolves from `pieces`, driven the way the
    /// launcher drives it. Adjoining plain text is joined and its whitespace
    /// left out: where a read stops decides where plain text is cut.
    fn resolved<'a>(pieces: impl IntoIterator<Item = &'a [u8]>) -> Vec<ParsedItem> {
        let mut parser = LogParser::new();
        let mut items: Vec<ParsedItem> = Vec::new();
        for piece in pieces {
            parser.feed(piece);
            loop {
                let item = parser
                    .parse_next()
                    .expect("the sample log has no broken part");
                match item {
                    None | Some(ParsedItem::Partial(_)) => break,
                    Some(ParsedItem::PlainText(text)) => {
                        let text: String = text.split_ascii_whitespace().collect();
                        match items.last_mut() {
                            Some(ParsedItem::PlainText(last)) => last.push_str(&text),
                            _ => items.push(ParsedItem::PlainText(text)),
                        }
                    }
                    Some(entry) => items.push(entry),
                }
            }
        }
        assert_eq!(parser.buffered_len(), 0);
        items
    }

    fn entries(items: &[ParsedItem]) -> usize {
        items
            .iter()
            .filter(|item| matches!(item, ParsedItem::LogEntry(_)))
            .count()
    }

    #[test]
    fn the_sample_log_resolves_the_same_whatever_the_size_of_its_pieces() {
        let whole = resolved([SAMPLE]);
        assert_eq!(entries(&whole), 3196);
        assert!(whole.len() > entries(&whole), "the sample has plain text");

        for size in [13, 4096, 65_536, 1_000_003] {
            assert!(
                resolved(SAMPLE.chunks(size)) == whole,
                "pieces of {size} bytes resolve differently from the whole log"
            );
        }
    }

    #[test]
    fn resolving_an_item_leaves_the_rest_of_the_buffer_where_it_is() {
        let mut log = Vec::new();
        for _ in 0..100 {
            log.extend_from_slice(EVENT);
            log.push(b'\n');
        }

        let mut parser = LogParser::new();
        parser.feed(&log);
        let mut left = parser.buffered_len();
        for resolved in 1..=100 {
            assert!(matches!(
                parser.parse_next().unwrap(),
                Some(ParsedItem::LogEntry(_))
            ));
            assert!(parser.buffered_len() < left);
            left = parser.buffered_len();
            if resolved < 100 {
                assert_eq!(parser.buffer.len(), log.len(), "after {resolved} items");
            }
        }

        // With nothing left to resolve, nothing is held.
        assert_eq!(parser.buffered_len(), 0);
        assert!(parser.buffer.is_empty());
        assert_eq!(parser.parse_next().unwrap(), None);
    }

    /// A running game's output, every read of it ending inside an event: the
    /// resolved bytes have to leave the buffer although it never empties.
    #[test]
    fn the_buffer_does_not_grow_while_every_read_ends_inside_an_event() {
        let (head, tail) = EVENT.split_at(20);
        let mut parser = LogParser::new();
        parser.feed(head);
        assert!(matches!(
            parser.parse_next().unwrap(),
            Some(ParsedItem::Partial(_))
        ));

        for _ in 0..1000 {
            let mut read = tail.to_vec();
            read.extend_from_slice(b"\n");
            read.extend_from_slice(head);
            parser.feed(&read);

            assert!(matches!(
                parser.parse_next().unwrap(),
                Some(ParsedItem::LogEntry(_))
            ));
            assert_eq!(
                parser.parse_next().unwrap(),
                Some(ParsedItem::Partial(head.to_vec()))
            );
            assert_eq!(parser.buffered_len(), head.len());
            // One read and the start left over from the read before it.
            assert_eq!(parser.buffer.len(), head.len() + read.len());
        }
    }

    #[test]
    fn an_event_start_is_known_in_any_case_and_when_cut_short() {
        for start in [
            "<log4j:Event",
            "<log4j:Event logger=\"a\">",
            "<LOG4J:EVENT>",
            "<Log4J:event and a long tail",
            "<",
            "<l",
            "<LOG4j:Ev",
        ] {
            assert!(is_potential_log4j_start(start.as_bytes()), "{start}");
        }
        for other in [
            "",
            "log4j:Event",
            " <log4j:Event",
            "<init>",
            "<log4j:Message>",
            "<log4j-Event>",
            "<lo>",
            "<log4j:Even>",
        ] {
            assert!(!is_potential_log4j_start(other.as_bytes()), "{other}");
        }
    }

    fn resolve_whole(log: &[u8]) -> (usize, Duration) {
        let started = Instant::now();
        let mut parser = LogParser::new();
        parser.feed(log);
        let mut items = 0;
        while let Ok(Some(item)) = parser.parse_next() {
            if matches!(item, ParsedItem::Partial(_)) {
                break;
            }
            items += 1;
        }
        (items, started.elapsed())
    }

    /// A past session's log is fed in one piece. The time it takes must grow
    /// with its size, not with the square of it: these two take a second or
    /// two here, and did not finish in minutes when every event moved the
    /// rest of the log and every `<` copied it.
    #[test]
    fn a_log_fed_in_one_piece_is_resolved_in_time_in_proportion_to_its_size() {
        let generous = Duration::from_secs(30);

        let events = SAMPLE.repeat(16);
        let (items, took) = resolve_whole(&events);
        assert!(items >= 16 * 3196);
        assert!(took < generous, "38 MB of events took {took:?}");

        let plain_text = b"\tat a.B.<init>(B.java:1)\n".repeat(160_000);
        let (items, took) = resolve_whole(&plain_text);
        assert_eq!(items, 1);
        assert!(took < generous, "4 MB of plain text took {took:?}");
    }
}
