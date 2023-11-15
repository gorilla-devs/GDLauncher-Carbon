//! This module provides support for parsing log4j messages.

use nom::{
    branch::alt,
    bytes::streaming::{tag, take_until},
    character::streaming::{anychar, char, multispace0, u64},
    combinator::{map, value},
    error::ParseError,
    multi::{count, many_till},
    sequence::{delimited, preceded, separated_pair, tuple},
    IResult,
};

/// Represents a parsed log4j message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LogEntry<'a> {
    /// The name of the logger.
    pub logger: &'a str,
    /// The log level of the entry.
    pub level: LogEntryLevel,
    /// The time the event was logged.
    pub timestamp: u64,
    /// The name of the thread.
    pub thread_name: &'a str,
    /// The log message.
    pub message: &'a str,
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

/// A combinator that removes whitespace before and after the `inner` parser.
fn whitespace<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

/// Parses a log4j event.
pub fn parse_log_entry(input: &str) -> IResult<&str, LogEntry> {
    let (o, (attributes, _, message)) = preceded(
        multispace0,
        delimited(
            tag("<log4j:Event"),
            tuple((attributes, tag(">"), whitespace(message))),
            tag("</log4j:Event>"),
        ),
    )(input)?;

    let Attributes {
        logger,
        level,
        timestamp,
        thread_name,
    } = attributes;

    Ok((
        o,
        LogEntry {
            logger,
            level,
            timestamp,
            thread_name,
            message,
        },
    ))
}

/// The attributes of a log event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Attributes<'a> {
    pub logger: &'a str,
    pub level: LogEntryLevel,
    pub timestamp: u64,
    pub thread_name: &'a str,
}

/// Parses the attributes of the event.
fn attributes(input: &str) -> IResult<&str, Attributes> {
    let (o, attributes) = count(whitespace(attribute), 4)(input)?;

    /// Macro to extract a field. Reduces boilerplate.
    macro_rules! extract_attribute {
        ($field:ident) => {{
            // Used to fail if we have repeating attributes
            let err = nom::Err::Error(nom::error::Error::from_error_kind(
                o,
                nom::error::ErrorKind::Alt,
            ));

            let Attribute::$field(field) = attributes
                .iter()
                .copied()
                .find(|attr| matches!(attr, Attribute::$field(_)))
                .ok_or(err)?
            else {
                unreachable!();
            };

            field
        }};
    }

    Ok((
        o,
        Attributes {
            logger: extract_attribute!(Logger),
            level: extract_attribute!(Level),
            timestamp: extract_attribute!(Timestamp),
            thread_name: extract_attribute!(ThreadName),
        },
    ))
}

/// The possible types of attributes a log event can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attribute<'a> {
    Logger(&'a str),
    Level(LogEntryLevel),
    Timestamp(u64),
    ThreadName(&'a str),
}

fn attribute(input: &str) -> IResult<&str, Attribute> {
    alt((attr_logger, attr_timestamp, attr_level, attr_thread))(input)
}

fn attr_logger(input: &str) -> IResult<&str, Attribute> {
    map(
        separated_pair(
            tag("logger"),
            whitespace(char('=')),
            map(quoted_string, Attribute::Logger),
        ),
        |(_, attr)| attr,
    )(input)
}

fn attr_timestamp(input: &str) -> IResult<&str, Attribute> {
    map(
        separated_pair(
            tag("timestamp"),
            whitespace(char('=')),
            delimited(char('"'), map(u64, Attribute::Timestamp), char('"')),
        ),
        |(_, attr)| attr,
    )(input)
}

fn attr_level(input: &str) -> IResult<&str, Attribute> {
    map(
        separated_pair(
            tag("level"),
            whitespace(char('=')),
            delimited(char('"'), map(level, Attribute::Level), char('"')),
        ),
        |(_, attr)| attr,
    )(input)
}

fn attr_thread(input: &str) -> IResult<&str, Attribute> {
    map(
        separated_pair(
            tag("thread"),
            whitespace(char('=')),
            map(quoted_string, Attribute::ThreadName),
        ),
        |(_, attr)| attr,
    )(input)
}

/// Parses a quoted string, i.e., "I am a quoted string".
///
/// TODO: This fails when we have escaped `'`, i.e. `\"` in the middle of the string.
/// "I \"quote this\"" will result in exactly that: I \"quote this\"
fn quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), take_until("\""), char('"'))(input)
}

/// Parses a [`LogEntryLevel`].
fn level(input: &str) -> IResult<&str, LogEntryLevel> {
    alt((
        value(LogEntryLevel::Trace, tag("TRACE")),
        value(LogEntryLevel::Debug, tag("DEBUG")),
        value(LogEntryLevel::Info, tag("INFO")),
        value(LogEntryLevel::Warn, tag("WARN")),
        value(LogEntryLevel::Error, tag("ERROR")),
    ))(input)
}

/// Parses the message of the event.
fn message(input: &str) -> IResult<&str, &str> {
    delimited(
        tag("<log4j:Message>"),
        whitespace(delimited(tag("<![CDATA["), take_until("]]>"), tag("]]>"))),
        tag("</log4j:Message>"),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_message() {
        message(
            r#"<log4j:Message>
                <![CDATA[192 Datafixer optimizations took 1128 milliseconds]]>
            </log4j:Message>"#,
        )
        .unwrap();
    }

    #[test]
    fn parse_attributes() {
        let (_, attributes) = attributes(
            r#"
            logger="com.mojang.datafixers.DataFixerBuilder"
            timestamp="1699556020363"
            level="INFO"
            thread="Datafixer Bootstrap"            
            >
            "#,
        )
        .unwrap();

        assert_eq!(
            attributes,
            Attributes {
                logger: "com.mojang.datafixers.DataFixerBuilder",
                level: LogEntryLevel::Info,
                timestamp: 1699556020363,
                thread_name: "Datafixer Bootstrap"
            }
        );
    }

    #[test]
    fn parse_logger_attribute() {
        let (_, attr) = attr_logger(r#"logger="com.mojang.datafixers.DataFixerBuilder""#).unwrap();

        assert_eq!(
            attr,
            Attribute::Logger("com.mojang.datafixers.DataFixerBuilder")
        );
    }

    #[test]
    fn parse_level_attribute() {
        let (_, attr) = attr_level(r#"level="INFO""#).unwrap();

        assert_eq!(attr, Attribute::Level(LogEntryLevel::Info));
    }

    #[test]
    fn parse_timestamp_attribute() {
        let (_, attr) = attr_timestamp(r#"timestamp="1699556020363""#).unwrap();

        assert_eq!(attr, Attribute::Timestamp(1699556020363));
    }

    #[test]
    fn parse_thread_attribute() {
        let (_, attr) = attr_thread(r#"thread="Datafixer Bootstrap""#).unwrap();

        assert_eq!(attr, Attribute::ThreadName("Datafixer Bootstrap"));
    }

    #[test]
    fn parse_quoted_string() {
        let (_, res) = quoted_string(r#""I am a quoted string""#).unwrap();

        assert_eq!(res, "I am a quoted string");
    }

    #[test]
    fn parse_single_entry() {
        let (_, entry) = parse_log_entry(
            r#"
            <log4j:Event
                logger="com.mojang.datafixers.DataFixerBuilder"
                timestamp="1699556020363"
                level="INFO"
                thread="Datafixer Bootstrap"
            >
                <log4j:Message>
                    <![CDATA[192 Datafixer optimizations took 1128 milliseconds]]>
                </log4j:Message>
            </log4j:Event>
            "#,
        )
        .unwrap();

        assert_eq!(
            entry,
            LogEntry {
                logger: "com.mojang.datafixers.DataFixerBuilder".into(),
                level: LogEntryLevel::Info,
                timestamp: 1699556020363,
                thread_name: "Datafixer Bootstrap".into(),
                message: "192 Datafixer optimizations took 1128 milliseconds".into(),
            }
        );
    }

    #[test]
    fn parse_sample_log_entries() {
        let mut input = include_str!("../sample_log.xml");

        while let Ok((o, _)) = parse_log_entry(input) {
            input = o;
        }

        assert_eq!(input, "\nexit code: 1");
    }
}
