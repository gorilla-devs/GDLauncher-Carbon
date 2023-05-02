use std::ops::Range;

pub struct GameLog {
    // buffer holding the full log
    log: String,
    lines: Vec<Option<InternalLogEntry>>,
    last_entry: usize,
    last_was_terminated: bool,
}

// Note: Option<LogEntry> shares a repr with LogEntry due to enum optimization
pub struct InternalLogEntry {
    type_: EntryType,
    start: usize,
    end: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LogEntry<'a> {
    type_: EntryType,
    start_line: usize,
    text: &'a str,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EntryType {
    StdOut,
    StdErr,
    // more entries once log levels are handled
}

impl InternalLogEntry {
    pub fn retreive<'l>(&self, log: &'l GameLog) -> &'l str {
        &log.log[self.start..self.end]
    }
}

impl GameLog {
    pub fn new() -> Self {
        Self {
            log: String::new(),
            lines: Vec::new(),
            last_entry: 0,
            last_was_terminated: true,
        }
    }

    /// Push new text to the log.
    ///
    /// # Note
    /// If this entry's type matches that of the last entry and
    /// the last entry did not end in \n they will be merged.
    pub fn push(&mut self, type_: EntryType, mut text: &str) {
        fn push_newlines(self_: &mut GameLog, text: &str) {
            let mut newline = false;
            let mut newline_count = 0;

            for c in text.chars() {
                newline = c == '\n';

                if newline {
                    newline_count += 1;
                }
            }

            self_.last_was_terminated = newline;

            if newline {
                self_.lines.extend((1..newline_count).map(|_| None));
            }
        }

        let mut newline = false;
        let mut newline_count = 0;

        for c in text.chars() {
            newline = c == '\n';

            if newline {
                newline_count += 1;
            }
        }

        if newline {
            text = &text[0..text.len() - 1];
        }

        if !self.last_was_terminated {
            if let Some(Some(last)) = self.lines.get_mut(self.last_entry) {
                if last.type_ == type_ {
                    if text.is_empty() {
                        return;
                    }

                    self.log.push_str(text);
                    last.end = self.log.len();

                    self.last_was_terminated = newline;

                    if newline {
                        self.lines.extend((1..newline_count).map(|_| None));
                    }

                    return;
                }
            }
        }

        let start = self.log.len();
        self.log.push_str(text);
        let end = self.log.len();

        self.last_entry = self.lines.len(); // len == last + 1
        self.lines
            .push(Some(InternalLogEntry { type_, start, end }));

        self.last_was_terminated = newline;

        if newline {
            self.lines.extend((1..newline_count).map(|_| None));
        }
    }

    /// Get the first log entry before the given line
    pub fn get_entry(&self, line: usize) -> Option<LogEntry> {
        for i in (0..=line).rev() {
            let Some(entry) = self.lines.get(i) else { return None };

            if let Some(entry) = entry {
                return Some(LogEntry {
                    type_: entry.type_,
                    start_line: i,
                    text: &self.log[entry.start..entry.end],
                });
            }
        }

        return None;
    }

    /// Get a region of log entries containing the given start and end lines
    pub fn get_region(&self, lines: Range<usize>) -> Vec<LogEntry> {
        let mut entries = Vec::<LogEntry>::new();

        for i in (0..lines.end).rev() {
            let Some(entry) = self.lines.get(i) else { continue };

            if let Some(entry) = entry {
                entries.push(LogEntry {
                    type_: entry.type_,
                    start_line: i,
                    text: &self.log[entry.start..entry.end],
                });

                if i <= lines.start {
                    break;
                }
            }
        }

        entries.reverse();
        return entries;
    }
}

#[cfg(test)]
mod test {
    use super::{EntryType, GameLog, LogEntry};

    #[test]
    fn push() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing\n");
        assert_eq!(
            log.get_entry(0),
            Some(LogEntry {
                type_: EntryType::StdOut,
                start_line: 0,
                text: "testing"
            }),
        );
    }

    #[test]
    fn region() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing1\n");
        log.push(EntryType::StdOut, "testing2\n");
        assert_eq!(
            log.get_region(0..2),
            vec![
                LogEntry {
                    type_: EntryType::StdOut,
                    start_line: 0,
                    text: "testing1"
                },
                LogEntry {
                    type_: EntryType::StdOut,
                    start_line: 1,
                    text: "testing2"
                },
            ],
        );
    }

    #[test]
    fn line_merging() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing1");
        log.push(EntryType::StdOut, "testing2\n");
        assert_eq!(
            log.get_entry(0),
            Some(LogEntry {
                type_: EntryType::StdOut,
                start_line: 0,
                text: "testing1testing2"
            }),
        );

        log.push(EntryType::StdOut, "testing3");
        log.push(EntryType::StdErr, "testing4\n");
        assert_eq!(
            log.get_region(0..3),
            vec![
                LogEntry {
                    type_: EntryType::StdOut,
                    start_line: 0,
                    text: "testing1testing2"
                },
                LogEntry {
                    type_: EntryType::StdOut,
                    start_line: 1,
                    text: "testing3"
                },
                LogEntry {
                    type_: EntryType::StdErr,
                    start_line: 2,
                    text: "testing4"
                },
            ],
        );
    }

    #[test]
    fn multiline_entry() {
        let mut log = GameLog::new();
        log.push(EntryType::StdOut, "testing1\ntesting2\n");

        let entry = LogEntry {
            type_: EntryType::StdOut,
            start_line: 0,
            text: "testing1\ntesting2",
        };

        assert_eq!(log.get_entry(0), Some(entry));
        assert_eq!(log.get_entry(1), Some(entry));
        assert_eq!(log.get_region(0..2), vec![entry]);
        assert_eq!(log.get_region(0..1), vec![entry]);
        assert_eq!(log.get_region(1..2), vec![entry]);
    }
}
