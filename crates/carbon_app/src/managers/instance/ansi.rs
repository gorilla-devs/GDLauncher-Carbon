//! Taking terminal colour codes out of a game's output.

use vte::{Parser, Perform};

/// Removes colour codes and other terminal control sequences from output
/// that arrives in pieces.
///
/// A game's output is read a piece at a time, and a piece ends wherever the
/// read did: in the middle of a character of several bytes, or of a colour
/// code. What is unfinished at the end of one piece is kept and completed by
/// the next, so neither is damaged by where the read happened to stop.
///
/// Line breaks and tabs are kept (a stack trace is indented with tabs);
/// every other control character, the carriage return included, is dropped.
///
/// Nothing is carried over a line break: a control sequence that never ends
/// (a stray `ESC ]`, say) hides the rest of its own line at most, not the
/// output that follows.
pub struct AnsiStripper {
    parser: Parser,
    /// The first bytes of a character that the last piece ended inside.
    unfinished: Vec<u8>,
}

impl AnsiStripper {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            unfinished: Vec::new(),
        }
    }

    /// The text of the next piece of output, without its control sequences.
    pub fn strip(&mut self, piece: &[u8]) -> String {
        // A character cut by the end of the piece waits here for its other
        // bytes instead of going to the parser in two halves: the parser's
        // own joining of the halves (vte 0.14.1) loses the character after.
        let mut bytes = std::mem::take(&mut self.unfinished);
        bytes.extend_from_slice(piece);
        self.unfinished = bytes.split_off(start_of_unfinished_character(&bytes));

        let mut text = Text(String::with_capacity(bytes.len()));

        for line in bytes.split_inclusive(|&byte| byte == b'\n') {
            match line.split_last() {
                Some((b'\n', content)) => {
                    self.parser.advance(&mut text, content);
                    text.0.push('\n');
                    self.parser = Parser::new();
                }
                _ => self.parser.advance(&mut text, line),
            }
        }

        text.0
    }
}

/// Where the character that `bytes` ends in the middle of starts, or the
/// length of `bytes` when it ends on a whole character (or on bytes that
/// could never become one).
fn start_of_unfinished_character(bytes: &[u8]) -> usize {
    let tail = bytes.len().saturating_sub(3);

    for start in (tail..bytes.len()).rev() {
        let length = match bytes[start] {
            // A byte from the middle of a character: its first byte is before it.
            0x80..=0xBF => continue,
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => break,
        };
        if start + length > bytes.len() {
            return start;
        }
        break;
    }

    bytes.len()
}

struct Text(String);

impl Perform for Text {
    fn print(&mut self, c: char) {
        self.0.push(c);
    }

    fn execute(&mut self, byte: u8) {
        if byte == b'\t' {
            self.0.push('\t');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text of an output read in the given pieces.
    fn read(pieces: &[&[u8]]) -> String {
        let mut stripper = AnsiStripper::new();
        pieces.iter().map(|piece| stripper.strip(piece)).collect()
    }

    /// The text of an output for every place a read could have stopped in it.
    fn read_split_everywhere(output: &[u8]) -> Vec<String> {
        (0..=output.len())
            .map(|at| read(&[&output[..at], &output[at..]]))
            .collect()
    }

    #[test]
    fn a_character_cut_by_a_read_is_whole() {
        let output = "Игрок 张伟 joined 🎮\n";
        for text in read_split_everywhere(output.as_bytes()) {
            assert_eq!(text, output);
        }
    }

    #[test]
    fn only_a_character_that_can_still_be_finished_waits_for_the_next_piece() {
        assert_eq!(start_of_unfinished_character(b""), 0);
        assert_eq!(start_of_unfinished_character(b"abc"), 3);
        assert_eq!(start_of_unfinished_character("aй".as_bytes()), 3);
        assert_eq!(start_of_unfinished_character(&"aй".as_bytes()[..2]), 1);
        assert_eq!(start_of_unfinished_character(&"a张".as_bytes()[..3]), 1);
        assert_eq!(start_of_unfinished_character(&"a🎮".as_bytes()[..4]), 1);
        assert_eq!(start_of_unfinished_character("a🎮".as_bytes()), 5);
        // Never the start of a character, so nothing to wait for.
        assert_eq!(start_of_unfinished_character(b"a\xff"), 2);
        assert_eq!(start_of_unfinished_character(b"a\x80\x80\x80"), 4);
    }

    #[test]
    fn a_colour_code_cut_by_a_read_leaves_nothing_behind() {
        let output = b"\x1b[1;31mred text\x1b[0m plain \x1b[38;5;208morange\x1b[m\n";
        for text in read_split_everywhere(output) {
            assert_eq!(text, "red text plain orange\n");
        }
    }

    #[test]
    fn a_read_one_byte_at_a_time_gives_the_same_text() {
        let output = "\x1b[32m[Сервер]\x1b[0m\tготов\r\n".as_bytes();
        let pieces: Vec<&[u8]> = output.chunks(1).collect();
        assert_eq!(read(&pieces), "[Сервер]\tготов\n");
    }

    #[test]
    fn tabs_and_line_breaks_are_kept_and_other_control_characters_dropped() {
        assert_eq!(
            read(&[b"java.lang.Error: x\r\n\tat a.B.c(B.java:1)\nname\tvalue\x07\x08\n"]),
            "java.lang.Error: x\n\tat a.B.c(B.java:1)\nname\tvalue\n"
        );
    }

    #[test]
    fn a_control_sequence_that_never_ends_hides_its_own_line_only() {
        for stray in ["\x1b]", "\x1bP", "\x1b_", "\x1b["] {
            let output = format!("before {stray}lost\nnext line\nand the one after\n");
            for text in read_split_everywhere(output.as_bytes()) {
                assert!(text.starts_with("before "), "{stray:?}: {text:?}");
                assert!(
                    text.ends_with("\nnext line\nand the one after\n"),
                    "{stray:?}: {text:?}"
                );
                assert_eq!(text.lines().count(), 3, "{stray:?}: {text:?}");
            }
        }
    }

    #[test]
    fn bytes_that_are_no_text_do_not_cost_the_text_around_them() {
        let text = read(&[b"good \xff\xfe text\n", b"next\n"]);
        assert!(text.starts_with("good "));
        assert!(text.ends_with(" text\nnext\n"));
    }
}
