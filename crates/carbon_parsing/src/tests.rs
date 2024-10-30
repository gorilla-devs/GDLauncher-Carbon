use crate::log::{LogEntryLevel, LogParser, ParsedItem, ParserError};

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
    let first_chunk =
        b"<log4j:Event logger=\"Logger1\" timestamp=\"1234567890\" level=\"INFO\" thread=\"main\">\
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
            v => panic!("Expected MissingAttribute error {:?}", v),
        }
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

// #[test]
// fn test_whitespace_variations() {
//     let test_cases = vec![
//         (
//             // Simple spaces
//             r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
//                     <log4j:Message><![CDATA[ Simple ]]></log4j:Message>
//                 </log4j:Event>"#,
//             " Simple ",
//         ),
//         (
//             // Mixed whitespace
//             r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
//                     <log4j:Message><![CDATA[  Tabs    and    spaces  ]]></log4j:Message>
//                 </log4j:Event>"#,
//             "  Tabs    and    spaces  ",
//         ),
//         (
//             // Newlines
//             r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
//                     <log4j:Message><![CDATA[
//                         Multiline
//                         message
//                     ]]></log4j:Message>
//                 </log4j:Event>"#,
//             "\n                    Multiline\n                    message\n                ",
//         ),
//         (
//             // Empty content
//             r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
//                     <log4j:Message><![CDATA[]]></log4j:Message>
//                 </log4j:Event>"#,
//             "",
//         ),
//         (
//             // Only whitespace
//             r#"<log4j:Event logger="L1" timestamp="1" level="INFO" thread="t1">
//                     <log4j:Message><![CDATA[     ]]></log4j:Message>
//                 </log4j:Event>"#,
//             "     ",
//         ),
//     ];

//     for (input, expected_message) in test_cases {
//         let mut parser = LogParser::new();
//         parser.feed(input.as_bytes());

//         match parser.parse_next().unwrap() {
//             Some(ParsedItem::LogEntry(entry)) => {
//                 assert_eq!(entry.message, expected_message);
//             }
//             _ => panic!("Expected LogEntry"),
//         }
//     }
// }

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
        v => panic!("Expected Partial for partial tag {:?}", v),
    }

    // Feed malformed continuation
    parser.feed(b"bad>");
    match parser.parse_next().unwrap() {
        Some(ParsedItem::PlainText(_)) => {}
        v => panic!("Expected PlainText for malformed input {:?}", v),
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

// #[test]
// fn test_fuzzy_parsing_variations() {
//     let log_text = r#"Some plain text before
// <log4j:Event logger="FuzzyLogger" timestamp="1234567890" level="INFO" thread="thread1">
//     <log4j:Message><![CDATA[First message with special chars: & < > " ']]></log4j:Message>
// </log4j:Event>
// Some text between logs...
// <log4j:Event logger="AnotherLogger" timestamp="1234567891" level="ERROR" thread="thread2">
//     <log4j:Message><![CDATA[Second message
// with multiple
// lines]]></log4j:Message>
// </log4j:Event>
// And some text after"#;

//     // Helper function to verify parsed results
//     fn verify_results(items: Vec<ParsedItem>) {
//         assert_eq!(items.len(), 5);

//         match &items[0] {
//             ParsedItem::PlainText(text) => assert_eq!(text.trim(), "Some plain text before"),
//             _ => panic!("Expected plain text"),
//         }

//         match &items[1] {
//             ParsedItem::LogEntry(entry) => {
//                 assert_eq!(entry.logger, "FuzzyLogger");
//                 assert_eq!(entry.timestamp, 1234567890);
//                 assert_eq!(entry.level, LogEntryLevel::Info);
//                 assert_eq!(entry.thread_name, "thread1");
//                 assert_eq!(
//                     entry.message,
//                     "First message with special chars: & < > \" '"
//                 );
//             }
//             _ => panic!("Expected log entry"),
//         }

//         match &items[2] {
//             ParsedItem::PlainText(text) => assert_eq!(text.trim(), "Some text between logs..."),
//             _ => panic!("Expected plain text"),
//         }

//         match &items[3] {
//             ParsedItem::LogEntry(entry) => {
//                 assert_eq!(entry.logger, "AnotherLogger");
//                 assert_eq!(entry.timestamp, 1234567891);
//                 assert_eq!(entry.level, LogEntryLevel::Error);
//                 assert_eq!(entry.thread_name, "thread2");
//                 assert_eq!(entry.message, "Second message\nwith multiple\nlines");
//             }
//             _ => panic!("Expected log entry"),
//         }

//         match &items[4] {
//             ParsedItem::PlainText(text) => assert_eq!(text.trim(), "And some text after"),
//             _ => panic!("Expected plain text"),
//         }
//     }

//     // Test 1: Feed entire text at once
//     let mut parser = LogParser::new();
//     parser.feed(log_text.as_bytes());
//     let results = parser.parse_available().unwrap();
//     verify_results(results);

//     // Test 2: Feed character by character
//     let mut parser = LogParser::new();
//     for c in log_text.chars() {
//         parser.feed(c.to_string().as_bytes());
//         let _ = parser.parse_available();
//     }
//     let results = parser.parse_available().unwrap();
//     verify_results(results);

//     // Test 3: Feed in random chunks
//     let mut parser = LogParser::new();
//     let mut remaining = log_text.as_bytes().to_vec();
//     let mut rng = rand::thread_rng();

//     while !remaining.is_empty() {
//         let chunk_size = rand::Rng::gen_range(&mut rng, 1..=remaining.len());
//         let chunk = remaining.drain(..chunk_size).collect::<Vec<_>>();
//         parser.feed(&chunk);
//         let _ = parser.parse_available();
//     }
//     let results = parser.parse_available().unwrap();
//     verify_results(results);

//     // Test 4: Feed in XML tag-aware chunks
//     let mut parser = LogParser::new();
//     for chunk in log_text.split_inclusive('>') {
//         parser.feed(chunk.as_bytes());
//         let _ = parser.parse_available();
//     }
//     let results = parser.parse_available().unwrap();
//     verify_results(results);
// }
