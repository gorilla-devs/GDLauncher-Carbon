//! Turns a world's in-game name into a folder name that is valid on Windows,
//! macOS and Linux and not taken by another world.

/// Longest folder name we create, in bytes.
const MAX_BYTES: usize = 100;

/// Names Windows reserves as devices, with or without an extension.
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "CLOCK$", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
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

#[cfg_attr(not(test), allow(dead_code))]
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
    let mut text = text.trim_end_matches(is_dot_or_space).to_string();

    let stem = text.split('.').next().unwrap_or_default();
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        text.insert(0, '_');
    }

    let cut = cut_to_bytes(&text, MAX_BYTES).trim_end_matches(is_dot_or_space);
    if cut.is_empty() {
        "World".to_string()
    } else {
        cut.to_string()
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn free_folder_name(wanted: &str, taken: &[String], current: Option<&str>) -> String {
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
            ("tab\tname".into(), "tab_name".into()),
            ("日本語 😀".into(), "日本語 😀".into()),
            (long, "é".repeat(50)),
            (almost, "a".repeat(99)),
        ];
        for (input, expected) in cases {
            assert_eq!(safe_folder_name(&input), expected, "input {input:?}");
        }
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
    }
}
