//! Defines platform-specific helpers.

/// Evaluates to `\r\n` on windows, and `\n` on everything else.
pub const LINE_ENDING: &str = if cfg!(windows) { "\r\n" } else { "\n" };
