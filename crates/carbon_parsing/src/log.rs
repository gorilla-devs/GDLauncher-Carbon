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
        if !self.tag_buffer.is_empty() {
            // If we have a partial tag, try to complete it
            self.tag_buffer.extend_from_slice(data);
            if let Some(full_tag) = self.try_complete_tag() {
                self.buffer.extend(full_tag);
                self.tag_buffer.clear();
            }
        } else {
            // Check if this data starts with a partial tag
            if let Some(pos) = self.find_incomplete_tag_start(data) {
                self.buffer.extend_from_slice(&data[..pos]);
                self.tag_buffer.extend_from_slice(&data[pos..]);
            } else {
                self.buffer.extend_from_slice(data);
            }
        }
    }

    /// Looks for incomplete tag starts in the data.
    fn find_incomplete_tag_start(&self, data: &[u8]) -> Option<usize> {
        for (i, window) in data.windows(5).enumerate() {
            if window.starts_with(b"<") {
                let remaining_len = data.len() - i;
                if remaining_len < b"<log4j:Event".len() {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Attempts to complete a partial tag.
    fn try_complete_tag(&self) -> Option<Vec<u8>> {
        let tag_pattern = b"<log4j:Event";

        if self.tag_buffer.len() < tag_pattern.len() {
            return None;
        }

        if &self.tag_buffer[..tag_pattern.len()] == tag_pattern {
            Some(self.tag_buffer.clone())
        } else {
            Some(self.tag_buffer.clone())
        }
    }

    /// Parses a log level string into the LogEntryLevel enum.
    fn parse_log_level(level: &str) -> Result<LogEntryLevel, ParserError> {
        match level.to_uppercase().as_str() {
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
                "logger" => entry.logger = value.trim().to_string(),
                "timestamp" => entry.timestamp = value.trim().parse()?,
                "level" => entry.level = Self::parse_log_level(value.trim())?,
                "thread" => entry.thread_name = value.trim().to_string(),
                _ => {}
            }
        }

        if entry.logger.is_empty() {
            return Err(ParserError::MissingAttribute("logger".to_string()));
        }
        if entry.timestamp == 0 {
            return Err(ParserError::MissingAttribute("timestamp".to_string()));
        }

        Ok(entry)
    }

    /// Attempts to parse the next item from the buffer.
    pub fn parse_next(&mut self) -> Result<Option<ParsedItem>, ParserError> {
        if self.buffer.is_empty() && self.partial_data.is_empty() && self.tag_buffer.is_empty() {
            return Ok(None);
        }

        // If we have a partial tag, wait for more data
        if !self.tag_buffer.is_empty() {
            return Ok(Some(ParsedItem::Partial(self.tag_buffer.clone())));
        }

        // Handle partial data from previous parse attempts
        if !self.partial_data.is_empty() {
            self.buffer.splice(0..0, self.partial_data.drain(..));
        }

        let mut reader = Reader::from_reader(BufReader::new(&self.buffer[..]));

        let mut buf = Vec::new();

        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"log4j:Event" => {
                let mut entry = Self::parse_attributes(e)?;
                let mut found_message = false;

                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Start(ref e)) if e.name().as_ref() == b"log4j:Message" => {
                            let mut message = Vec::new();
                            loop {
                                match reader.read_event_into(&mut buf) {
                                    Ok(Event::Text(e)) => {
                                        message.extend_from_slice(&e.into_inner());
                                    }
                                    Ok(Event::CData(e)) => {
                                        message.extend_from_slice(&e.into_inner());
                                    }
                                    Ok(Event::End(ref e))
                                        if e.name().as_ref() == b"log4j:Message" =>
                                    {
                                        // Preserve all whitespace in the message content
                                        entry.message = String::from_utf8(message)?;
                                        found_message = true;
                                        break;
                                    }
                                    Ok(Event::Eof) => {
                                        self.partial_data.extend_from_slice(&self.buffer);
                                        self.buffer.clear();
                                        return Ok(Some(ParsedItem::Partial(
                                            self.partial_data.clone(),
                                        )));
                                    }
                                    Err(_) => {
                                        self.partial_data.extend_from_slice(&self.buffer);
                                        self.buffer.clear();
                                        return Ok(Some(ParsedItem::Partial(
                                            self.partial_data.clone(),
                                        )));
                                    }
                                    _ => continue,
                                }
                            }
                        }
                        Ok(Event::End(ref e)) if e.name().as_ref() == b"log4j:Event" => {
                            if found_message {
                                let consumed = reader.buffer_position();
                                self.buffer.drain(..consumed as usize);
                                return Ok(Some(ParsedItem::LogEntry(entry)));
                            }
                            return Err(ParserError::MissingAttribute("message".to_string()));
                        }
                        Ok(Event::Eof) => {
                            self.partial_data.extend_from_slice(&self.buffer);
                            self.buffer.clear();
                            return Ok(Some(ParsedItem::Partial(self.partial_data.clone())));
                        }
                        Err(_) => {
                            self.partial_data.extend_from_slice(&self.buffer);
                            self.buffer.clear();
                            return Ok(Some(ParsedItem::Partial(self.partial_data.clone())));
                        }
                        _ => continue,
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape()?.into_owned();
                if !text.trim().is_empty() {
                    let consumed = reader.buffer_position();
                    self.buffer.drain(..consumed as usize);
                    return Ok(Some(ParsedItem::PlainText(text)));
                }
                let consumed = reader.buffer_position();
                self.buffer.drain(..consumed as usize);
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
            Err(_) => {
                if let Some(pos) = self.find_incomplete_tag_start(&self.buffer) {
                    self.tag_buffer.extend_from_slice(&self.buffer[pos..]);
                    self.buffer.truncate(pos);
                    if !self.buffer.is_empty() {
                        return self.parse_next();
                    }
                } else {
                    self.partial_data.extend_from_slice(&self.buffer);
                    self.buffer.clear();
                }
                Ok(Some(ParsedItem::Partial(self.partial_data.clone())))
            }
            Ok(_) => {
                let consumed = reader.buffer_position();
                if consumed > 0 {
                    self.buffer.drain(..consumed as usize);
                } else if !self.buffer.is_empty() {
                    self.buffer.remove(0);
                }
                self.parse_next()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_complete_xml_entry() {
        let input = r#"<log4j:Event logger="TestLogger" timestamp="1234567890" level="INFO" thread="main">
            <log4j:Message><![CDATA[Test message]]></log4j:Message>
        </log4j:Event>"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        match parser.parse_next().unwrap() {
            Some(ParsedItem::LogEntry(entry)) => {
                assert_eq!(entry.logger, "TestLogger");
                assert_eq!(entry.timestamp, 1234567890);
                assert_eq!(entry.level, LogEntryLevel::Info);
                assert_eq!(entry.thread_name, "main");
                assert_eq!(entry.message.trim(), "Test message");
            }
            _ => panic!("Expected LogEntry"),
        }
    }

    #[test]
    fn test_multiple_complete_events() {
        let input = r#"<log4j:Event logger="Logger1" timestamp="1234567890" level="INFO" thread="main">
            <log4j:Message><![CDATA[First message]]></log4j:Message>
        </log4j:Event>
        <log4j:Event logger="Logger2" timestamp="1234567891" level="WARN" thread="worker">
            <log4j:Message><![CDATA[Second message]]></log4j:Message>
        </log4j:Event>"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        let items = parser.parse_available().unwrap();
        assert_eq!(items.len(), 2);

        match &items[0] {
            ParsedItem::LogEntry(entry) => {
                assert_eq!(entry.logger, "Logger1");
                assert_eq!(entry.level, LogEntryLevel::Info);
                assert_eq!(entry.message.trim(), "First message");
            }
            _ => panic!("Expected first LogEntry"),
        }

        match &items[1] {
            ParsedItem::LogEntry(entry) => {
                assert_eq!(entry.logger, "Logger2");
                assert_eq!(entry.level, LogEntryLevel::Warn);
                assert_eq!(entry.message.trim(), "Second message");
            }
            _ => panic!("Expected second LogEntry"),
        }
    }

    #[test]
    fn test_mixed_xml_and_plain_text() {
        let input = r#"Plain text line 1
        <log4j:Event logger="Logger1" timestamp="1234567890" level="INFO" thread="main">
            <log4j:Message><![CDATA[XML message]]></log4j:Message>
        </log4j:Event>
        Plain text line 2"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        let items = parser.parse_available().unwrap();
        assert_eq!(items.len(), 3);

        match &items[0] {
            ParsedItem::PlainText(text) => {
                assert_eq!(text.trim(), "Plain text line 1");
            }
            _ => panic!("Expected first PlainText"),
        }

        match &items[1] {
            ParsedItem::LogEntry(entry) => {
                assert_eq!(entry.message.trim(), "XML message");
            }
            _ => panic!("Expected LogEntry"),
        }

        match &items[2] {
            ParsedItem::PlainText(text) => {
                assert_eq!(text.trim(), "Plain text line 2");
            }
            _ => panic!("Expected second PlainText"),
        }
    }

    #[test]
    fn test_split_tag() {
        let first_chunk = b"<log4";
        let second_chunk =
            b"j:Event logger=\"Logger1\" timestamp=\"1234567890\" level=\"INFO\" thread=\"main\">\
            <log4j:Message><![CDATA[Test message]]></log4j:Message>\
        </log4j:Event>";

        let mut parser = LogParser::new();

        // Feed first chunk
        parser.feed(first_chunk);
        let result = parser.parse_next().unwrap();
        match result {
            Some(ParsedItem::Partial(_)) => {}
            _ => panic!("Expected Partial for split tag"),
        }

        // Feed second chunk
        parser.feed(second_chunk);
        let result = parser.parse_next().unwrap();
        match result {
            Some(ParsedItem::LogEntry(entry)) => {
                assert_eq!(entry.logger, "Logger1");
                assert_eq!(entry.message.trim(), "Test message");
            }
            _ => panic!("Expected complete LogEntry after split tag"),
        }
    }

    #[test]
    fn test_split_attribute() {
        let first_chunk = b"<log4j:Event logger=\"Log";
        let second_chunk = b"ger1\" timestamp=\"1234567890\" level=\"INFO\" thread=\"main\">\
            <log4j:Message><![CDATA[Test message]]></log4j:Message>\
        </log4j:Event>";

        let mut parser = LogParser::new();

        parser.feed(first_chunk);
        let result = parser.parse_next().unwrap();
        match result {
            Some(ParsedItem::Partial(_)) => {}
            _ => panic!("Expected Partial for split attribute"),
        }

        parser.feed(second_chunk);
        let result = parser.parse_next().unwrap();
        match result {
            Some(ParsedItem::LogEntry(entry)) => {
                assert_eq!(entry.logger, "Logger1");
            }
            _ => panic!("Expected complete LogEntry after split attribute"),
        }
    }

    #[test]
    fn test_split_cdata() {
        let first_chunk = b"<log4j:Event logger=\"Logger1\" timestamp=\"1234567890\" level=\"INFO\" thread=\"main\">\
            <log4j:Message><![CDATA[Test mess";
        let second_chunk = b"age]]></log4j:Message></log4j:Event>";

        let mut parser = LogParser::new();

        parser.feed(first_chunk);
        let result = parser.parse_next().unwrap();
        match result {
            Some(ParsedItem::Partial(_)) => {}
            _ => panic!("Expected Partial for split CDATA"),
        }

        parser.feed(second_chunk);
        let result = parser.parse_next().unwrap();
        match result {
            Some(ParsedItem::LogEntry(entry)) => {
                assert_eq!(entry.message, "Test message");
            }
            _ => panic!("Expected complete LogEntry after split CDATA"),
        }
    }

    // #[test]
    // fn test_multiple_split_events() {
    //     let chunks = vec![
    //         String::from("<log4j:Event logger=\"Logger1\" times"),
    //         String::from("tamp=\"1234567890\" level=\"INFO\" thread=\"main\">"),
    //         String::from("<log4j:Message><![CDATA[First message]]></log4j:Message></log4j:Event>"),
    //         String::from("<log4j:Event logger=\"Logger2\" timestamp=\"1234567891\" level=\"WARN\" "),
    //         String::from("thread=\"worker\"><log4j:Message><![CDATA[Second message]]></log4j:Message></log4j:Event>"),
    //     ];

    //     let mut parser = LogParser::new();
    //     let mut entries = Vec::new();

    //     for chunk in chunks {
    //         parser.feed(chunk.as_bytes());
    //         while let Ok(Some(item)) = parser.parse_next() {
    //             match item {
    //                 ParsedItem::LogEntry(entry) => entries.push(entry),
    //                 ParsedItem::Partial(_) => {}
    //                 _ => panic!("Unexpected item type"),
    //             }
    //         }
    //     }

    //     assert_eq!(entries.len(), 2);
    //     assert_eq!(entries[0].message.trim(), "First message");
    //     assert_eq!(entries[1].message.trim(), "Second message");
    // }

    #[test]
    fn test_invalid_log_level() {
        let input = r#"<log4j:Event logger="Logger1" timestamp="1234567890" level="INVALID" thread="main">
            <log4j:Message><![CDATA[Test message]]></log4j:Message>
        </log4j:Event>"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        match parser.parse_next() {
            Err(ParserError::InvalidLogLevel(_)) => {}
            _ => panic!("Expected InvalidLogLevel error"),
        }
    }

    #[test]
    fn test_missing_required_attributes() {
        let inputs = &[
            // Missing logger
            r#"<log4j:Event logger="" timestamp="1234567890" level="INFO" thread="main">
                <log4j:Message><![CDATA[Test message]]></log4j:Message>
            </log4j:Event>"#,
            // Missing timestamp
            r#"<log4j:Event logger="Logger1" timestamp="" level="INFO" thread="main">
                <log4j:Message><![CDATA[Test message]]></log4j:Message>
            </log4j:Event>"#,
        ];

        for input in inputs {
            let mut parser = LogParser::new();
            parser.feed(input.as_bytes());

            match parser.parse_next() {
                Err(ParserError::MissingAttribute(_)) => {}
                _ => panic!("Expected MissingAttribute error"),
            }
        }
    }

    #[test]
    fn test_invalid_timestamp() {
        let input = r#"<log4j:Event logger="Logger1" timestamp="not_a_number" level="INFO" thread="main">
            <log4j:Message><![CDATA[Test message]]></log4j:Message>
        </log4j:Event>"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        match parser.parse_next() {
            Err(ParserError::TimestampError(_)) => {}
            _ => panic!("Expected TimestampError error"),
        }
    }

    #[test]
    fn test_missing_message() {
        let input = r#"<log4j:Event logger="Logger1" timestamp="1234567890" level="INFO" thread="main">
        </log4j:Event>"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        match parser.parse_next() {
            Err(ParserError::MissingAttribute(attr)) => {
                assert_eq!(attr, "message");
            }
            _ => panic!("Expected MissingAttribute error for message"),
        }
    }

    #[test]
    fn test_nested_cdata() {
        let input = r#"<log4j:Event logger="Logger1" timestamp="1234567890" level="INFO" thread="main">
            <log4j:Message><![CDATA[Outer <![CDATA[Inner]]> message]]></log4j:Message>
        </log4j:Event>"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        match parser.parse_next().unwrap() {
            Some(ParsedItem::LogEntry(entry)) => {
                assert_eq!(entry.message.trim(), "Outer <![CDATA[Inner]]> message");
            }
            _ => panic!("Expected LogEntry"),
        }
    }

    #[test]
    fn test_whitespace_handling() {
        let input = r#"
            <log4j:Event    logger="Logger1"     timestamp="1234567890"    level="INFO"   thread="main"   >
                <log4j:Message>   <![CDATA[   Test message with spaces   ]]>   </log4j:Message>
            </log4j:Event>
            "#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        match parser.parse_next().unwrap() {
            Some(ParsedItem::LogEntry(entry)) => {
                assert_eq!(entry.logger, "Logger1");
                assert_eq!(entry.message, "      Test message with spaces      ");
                // Test the trimmed version separately
                assert_eq!(entry.message.trim(), "Test message with spaces");
            }
            _ => panic!("Expected LogEntry"),
        }
    }

    #[test]
    fn test_whitespace_variations() {
        let test_cases = vec![
            (
                // Simple spaces
                r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
                    <log4j:Message><![CDATA[ Simple ]]></log4j:Message>
                </log4j:Event>"#,
                " Simple ",
            ),
            (
                // Mixed whitespace
                r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
                    <log4j:Message><![CDATA[  Tabs    and    spaces  ]]></log4j:Message>
                </log4j:Event>"#,
                "  Tabs    and    spaces  ",
            ),
            (
                // Newlines
                r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
                    <log4j:Message><![CDATA[
                        Multiline
                        message
                    ]]></log4j:Message>
                </log4j:Event>"#,
                "\n                    Multiline\n                    message\n                ",
            ),
            (
                // Empty content
                r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
                    <log4j:Message><![CDATA[]]></log4j:Message>
                </log4j:Event>"#,
                "",
            ),
            (
                // Only whitespace
                r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
                    <log4j:Message><![CDATA[     ]]></log4j:Message>
                </log4j:Event>"#,
                "     ",
            ),
        ];

        for (input, expected_message) in test_cases {
            let mut parser = LogParser::new();
            parser.feed(input.as_bytes());

            match parser.parse_next().unwrap() {
                Some(ParsedItem::LogEntry(entry)) => {
                    assert_eq!(entry.message, expected_message);
                }
                _ => panic!("Expected LogEntry"),
            }
        }
    }

    #[test]
    fn test_whitespace_in_attributes() {
        let input = r#"<log4j:Event logger="  Logger with spaces  " timestamp="1234567890" level="INFO" thread="  Main Thread  ">
            <log4j:Message><![CDATA[Test]]></log4j:Message>
        </log4j:Event>"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        match parser.parse_next().unwrap() {
            Some(ParsedItem::LogEntry(entry)) => {
                // Attributes should have their whitespace trimmed
                assert_eq!(entry.logger, "Logger with spaces");
                assert_eq!(entry.thread_name, "Main Thread");
            }
            _ => panic!("Expected LogEntry"),
        }
    }

    #[test]
    fn test_empty_input() {
        let mut parser = LogParser::new();
        parser.feed(b"");

        match parser.parse_next().unwrap() {
            None => {}
            _ => panic!("Expected None for empty input"),
        }
    }

    #[test]
    fn test_only_whitespace() {
        let mut parser = LogParser::new();
        parser.feed(b"    \n    \t    \r\n    ");

        match parser.parse_next().unwrap() {
            None => {}
            _ => panic!("Expected None for whitespace-only input"),
        }
    }

    #[test]
    fn test_partial_then_malformed() {
        let mut parser = LogParser::new();

        // Feed partial tag
        parser.feed(b"<log");
        match parser.parse_next().unwrap() {
            Some(ParsedItem::Partial(_)) => {}
            _ => panic!("Expected Partial for partial tag"),
        }

        // Feed malformed continuation
        parser.feed(b"bad>");
        match parser.parse_next().unwrap() {
            Some(ParsedItem::PlainText(_)) => {}
            _ => panic!("Expected PlainText for malformed input"),
        }
    }

    #[test]
    fn test_all_log_levels() {
        let levels = vec![
            ("TRACE", LogEntryLevel::Trace),
            ("DEBUG", LogEntryLevel::Debug),
            ("INFO", LogEntryLevel::Info),
            ("WARN", LogEntryLevel::Warn),
            ("ERROR", LogEntryLevel::Error),
        ];

        for (level_str, expected_level) in levels {
            let input = format!(
                r#"<log4j:Event logger="Logger1" timestamp="1234567890" level="{}" thread="main">
                    <log4j:Message><![CDATA[Test message]]></log4j:Message>
                </log4j:Event>"#,
                level_str
            );

            let mut parser = LogParser::new();
            parser.feed(input.as_bytes());

            match parser.parse_next().unwrap() {
                Some(ParsedItem::LogEntry(entry)) => {
                    assert_eq!(entry.level, expected_level);
                }
                _ => panic!("Expected LogEntry"),
            }
        }
    }

    #[test]
    fn test_special_characters_in_attributes() {
        let input = r#"<log4j:Event logger="Logger&lt;&gt;" timestamp="1234567890" level="INFO" thread="thread&amp;name">
            <log4j:Message><![CDATA[Test message]]></log4j:Message>
        </log4j:Event>"#;

        let mut parser = LogParser::new();
        parser.feed(input.as_bytes());

        match parser.parse_next().unwrap() {
            Some(ParsedItem::LogEntry(entry)) => {
                assert_eq!(entry.logger, "Logger<>");
                assert_eq!(entry.thread_name, "thread&name");
            }
            _ => panic!("Expected LogEntry"),
        }
    }
}
