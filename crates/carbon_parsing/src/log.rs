//! This module provides support for parsing log4j messages.

use nom::{
    branch::alt,
    bytes::streaming::{tag, take_until},
    character::streaming::{anychar, char, multispace0, u64},
    combinator::{map, value},
    error::ParseError,
    multi::{count, many_till},
    sequence::{delimited, separated_pair, tuple},
    IResult,
};

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
    let (o, (attributes, _, message)) = whitespace(delimited(
        tag("<log4j:Event"),
        tuple((attributes, tag(">"), whitespace(message))),
        tag("</log4j:Event>"),
    ))(input)?;

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
            message: message.into(),
        },
    ))
}

/// The attributes of a log event.
struct Attributes {
    pub logger: String,
    pub level: LogEntryLevel,
    pub timestamp: u64,
    pub thread_name: String,
}

/// Parses the attributes of the event.
fn attributes(input: &str) -> IResult<&str, Attributes> {
    let (o, attributes) = count(attribute, 4)(input)?;

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
                .find(|attr| matches!(attr, Attribute::$field(_)))
                .ok_or(err)?
            else {
                unreachable!();
            };

            field.to_owned()
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
enum Attribute {
    Logger(String),
    Level(LogEntryLevel),
    Timestamp(u64),
    ThreadName(String),
}

fn attribute(input: &str) -> IResult<&str, Attribute> {
    whitespace(alt((
        separated_pair(
            tag("logger"),
            whitespace(char('=')),
            delimited(char('"'), map(quoted_string, Attribute::Logger), char('"')),
        ),
        separated_pair(
            tag("timestamp"),
            whitespace(char('=')),
            delimited(char('"'), map(u64, Attribute::Timestamp), char('"')),
        ),
        separated_pair(
            tag("level"),
            whitespace(char('=')),
            delimited(char('"'), map(level, Attribute::Level), char('"')),
        ),
        separated_pair(
            tag("thread"),
            whitespace(char('=')),
            delimited(
                char('"'),
                map(quoted_string, Attribute::ThreadName),
                char('"'),
            ),
        ),
    )))(input)?;

    todo!()
}

/// Parses a quoted string, i.e., "I am a quoted string".
///
/// Note, this function will output escaped characters literally, as in,
/// "I \"quote this\"" will result in exactly that: I \"quote this\"
fn quoted_string(input: &str) -> IResult<&str, String> {
    let (input, (res, _)) =
        delimited(char('"'), many_till(anychar, end_of_string), char('"'))(input)?;

    Ok((input, res.into_iter().collect()))
}

/// Finds the end of a quoted string, i.e., ignores `\"`.
fn end_of_string(input: &str) -> IResult<&str, &str> {
    match char::<_, nom::error::Error<_>>('"')(input) {
        Ok((o, _)) => return Ok((o, "\"")),
        Err(nom::Err::Incomplete(n)) => return Err(nom::Err::Incomplete(n)),
        _ => {}
    }

    tag::<_, _, nom::error::Error<_>>("\\\"")(input)
        .and_then(|_| Err(nom::Err::Error(nom::error::Error::from_char(input, '\"'))))
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
