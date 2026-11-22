//! Turns a world's in-game name into a folder name that is valid on Windows,
//! macOS and Linux and not taken by another world.

/// Longest folder name we create, in bytes.
const MAX_BYTES: usize = 100;

/// Names Windows reserves as devices, with or without an extension.
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "CLOCK$", "CONIN$", "CONOUT$", "COM0", "COM1", "COM2", "COM3",
    "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
    "LPT6", "LPT7", "LPT8", "LPT9",
];

fn is_dot_or_space(c: char) -> bool {
    c == '.' || c == ' '
}

/// Cuts `s` to at most `max` bytes at a character boundary.
fn cut_to_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

pub(super) fn safe_folder_name(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c < '\u{20}' || c == '\u{7f}' => '_',
            c => c,
        })
        .collect();

    let mut text = replaced.trim().to_string();
    if text.starts_with('.') {
        text.replace_range(..1, "_");
    }
    let text = text.trim_end_matches(is_dot_or_space);
    let mut text = cut_to_bytes(text, MAX_BYTES)
        .trim_end_matches(is_dot_or_space)
        .to_string();

    // Checked last so that the cut cannot leave a reserved name behind.
    let stem = text
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        text.insert(0, '_');
        text = cut_to_bytes(&text, MAX_BYTES)
            .trim_end_matches(is_dot_or_space)
            .to_string();
    }

    if text.is_empty() {
        "World".to_string()
    } else {
        text
    }
}

/// Picks a folder name for `wanted` that no other folder in `taken` has,
/// ignoring case. `current` is the world's own folder and does not count.
///
/// `wanted` is expected to be a `safe_folder_name` result, so at most
/// `MAX_BYTES` bytes.
pub(super) fn free_folder_name(wanted: &str, taken: &[String], current: Option<&str>) -> String {
    debug_assert!(
        wanted.len() <= MAX_BYTES,
        "wanted must be a safe_folder_name result"
    );
    let is_free = |candidate: &str| {
        let candidate = candidate.to_lowercase();
        !taken
            .iter()
            .filter(|t| Some(t.as_str()) != current)
            .any(|t| t.to_lowercase() == candidate)
    };

    if is_free(wanted) {
        return wanted.to_string();
    }

    for n in 2usize.. {
        let suffix = format!(" ({n})");
        let base = cut_to_bytes(wanted, MAX_BYTES.saturating_sub(suffix.len()));
        let candidate = format!("{base}{suffix}");
        if is_free(&candidate) {
            return candidate;
        }
    }
    unreachable!("an unbounded range always yields a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_folder_name_table() {
        let long = "é".repeat(300);
        let almost = format!("{}é", "a".repeat(99));
        let cases: Vec<(String, String)> = vec![
            ("My World".into(), "My World".into()),
            ("a:b/c\\d*e?f\"g<h>i|j".into(), "a_b_c_d_e_f_g_h_i_j".into()),
            ("  spaced  ".into(), "spaced".into()),
            ("name...".into(), "name".into()),
            ("...".into(), "_".into()),
            ("".into(), "World".into()),
            ("   ".into(), "World".into()),
            (".hidden".into(), "_hidden".into()),
            ("CON".into(), "_CON".into()),
            ("con.txt".into(), "_con.txt".into()),
            ("COM1".into(), "_COM1".into()),
            ("Console".into(), "Console".into()),
            ("CONIN$".into(), "_CONIN$".into()),
            ("conout$.log".into(), "_conout$.log".into()),
            ("COM0".into(), "_COM0".into()),
            ("lpt0".into(), "_lpt0".into()),
            ("COM10".into(), "COM10".into()),
            ("tab\tname".into(), "tab_name".into()),
            ("日本語 😀".into(), "日本語 😀".into()),
            (long, "é".repeat(50)),
            (almost, "a".repeat(99)),
            (format!("con{}x", " ".repeat(100)), "_con".into()),
            (format!("NUL{}.x", " ".repeat(97)), "_NUL".into()),
            ("con .txt".into(), "_con .txt".into()),
            (
                format!("COM1{}", "a".repeat(96)),
                format!("COM1{}", "a".repeat(96)),
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(safe_folder_name(&input), expected, "input {input:?}");
        }
        let long_com = safe_folder_name(&format!("COM1{}", "a".repeat(96)));
        assert_eq!(long_com.len(), 100);
        assert!(long_com.starts_with("COM1a"));
    }

    #[test]
    fn free_folder_name_table() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(free_folder_name("World", &[], None), "World");
        assert_eq!(free_folder_name("World", &s(&["world"]), None), "World (2)");
        assert_eq!(
            free_folder_name("World", &s(&["World", "World (2)"]), None),
            "World (3)"
        );
        assert_eq!(
            free_folder_name("world", &s(&["World"]), Some("World")),
            "world"
        );
        assert_eq!(
            free_folder_name("World", &s(&["World", "Other"]), Some("Other")),
            "World (2)"
        );
        assert_eq!(free_folder_name("É", &s(&["é"]), None), "É (2)");

        let full = "é".repeat(50);
        let result = free_folder_name(&full, &[full.clone()], None);
        assert!(result.ends_with(" (2)"), "{result:?}");
        assert!(result.len() <= 100, "{} bytes", result.len());
        assert_eq!(result, format!("{} (2)", "é".repeat(48)));
    }
}
