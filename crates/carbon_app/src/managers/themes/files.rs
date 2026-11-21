use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;

use super::error::ThemeError;

#[derive(Clone)]
pub struct ThemeLimits {
    pub max_total_bytes: u64,
    pub max_files: usize,
    pub max_css_bytes: u64,
    pub max_manifest_bytes: u64,
    /// Each `.txt` file (a font's licence, say).
    pub max_text_bytes: u64,
}

impl ThemeLimits {
    pub const DEFAULT: ThemeLimits = ThemeLimits {
        max_total_bytes: 52_428_800,
        max_files: 2_000,
        max_css_bytes: 1_048_576,
        max_manifest_bytes: 262_144,
        max_text_bytes: 262_144,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Manifest,
    Css,
    Image,
    Font,
    Audio,
    Video,
    /// A `.txt` file, such as a font's licence: kept with the theme, but never
    /// served and never usable from the manifest or the CSS.
    Text,
}

impl FileKind {
    pub fn is_asset(self) -> bool {
        matches!(
            self,
            FileKind::Image | FileKind::Font | FileKind::Audio | FileKind::Video
        )
    }
}

fn extension(rel: &str) -> Option<String> {
    let name = rel.rsplit('/').next()?;
    let (_, ext) = name.rsplit_once('.')?;
    Some(ext.to_ascii_lowercase())
}

pub fn file_kind(rel: &str) -> Option<FileKind> {
    let ext = extension(rel)?;
    match ext.as_str() {
        "json" => (rel == "manifest.json").then_some(FileKind::Manifest),
        "css" => Some(FileKind::Css),
        "png" | "jpg" | "jpeg" | "webp" | "avif" | "gif" | "svg" => Some(FileKind::Image),
        "woff2" | "woff" | "ttf" | "otf" => Some(FileKind::Font),
        "ogg" | "mp3" | "wav" => Some(FileKind::Audio),
        "webm" | "mp4" => Some(FileKind::Video),
        "txt" => Some(FileKind::Text),
        _ => None,
    }
}

/// Content type for asset files only (not the manifest or CSS).
pub fn content_type(rel: &str) -> Option<&'static str> {
    Some(match extension(rel)?.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "ogg" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "webm" => "video/webm",
        "mp4" => "video/mp4",
        _ => return None,
    })
}

/// True for characters that must never reach a message or a dialog from an
/// untrusted file: control characters, the line and paragraph separators, and
/// the Unicode format characters (category Cf), which can reorder how text reads
/// (U+202E and the other bidi controls) or hide inside it (zero-width marks).
///
/// The Cf ranges are listed by hand to avoid a dependency, from Unicode 16.0
/// (the list is the same for 15.1):
/// U+00AD, U+0600-0605, U+061C, U+06DD, U+070F, U+0890-0891, U+08E2, U+180E,
/// U+200B-200F, U+2028-2029 (not Cf, but line breaks), U+202A-202E, U+2060-206F,
/// U+FEFF, U+FFF9-FFFB, U+110BD, U+110CD, U+13430-1343F, U+1BCA0-1BCA3,
/// U+1D173-1D17A, U+E0001, U+E0020-E007F.
///
/// The tag characters (U+E0020-E007F) stay stripped even though emoji
/// subdivision flags (England, Scotland, Wales) use them; that costs only those
/// three flags.
pub fn is_unsafe_text_char(c: char) -> bool {
    c.is_control()
        || matches!(
            c as u32,
            0x00AD
                | 0x0600..=0x0605
                | 0x061C
                | 0x06DD
                | 0x070F
                | 0x0890..=0x0891
                | 0x08E2
                | 0x180E
                | 0x200B..=0x200F
                | 0x2028..=0x202E
                | 0x2060..=0x206F
                | 0xFEFF
                | 0xFFF9..=0xFFFB
                | 0x110BD
                | 0x110CD
                | 0x13430..=0x1343F
                | 0x1BCA0..=0x1BCA3
                | 0x1D173..=0x1D17A
                | 0xE0001
                | 0xE0020..=0xE007F
        )
}

/// Zero-width joiner and non-joiner. Emoji sequences (a bear plus a joiner plus
/// a snowflake) and Persian and Indic scripts need them, and they can't reorder
/// text, so display fields keep them. Messages and logs still strip them.
fn is_joiner(c: char) -> bool {
    matches!(c, '\u{200C}' | '\u{200D}')
}

/// Untrusted display text (a theme's name, author, description) with every
/// `is_unsafe_text_char` character removed except the joiners, no cap.
pub fn clean_text(value: &str) -> String {
    value
        .chars()
        .filter(|c| is_joiner(*c) || !is_unsafe_text_char(*c))
        .collect()
}

/// True when nothing visible is left: the text is empty once whitespace and
/// joiners are removed, so a field can't be made of joiners alone.
pub fn is_blank_text(value: &str) -> bool {
    value.chars().all(|c| c.is_whitespace() || is_joiner(c))
}

/// Untrusted text for an error message: control and format characters removed,
/// length capped.
pub fn shown(value: &str) -> String {
    const MAX: usize = 100;
    let mut kept = value.chars().filter(|c| !is_unsafe_text_char(*c));
    let mut out: String = kept.by_ref().take(MAX).collect();
    if kept.next().is_some() {
        out.push('…');
    }
    out
}

/// Like `shown`, for a parser's own message (serde's "invalid type: string
/// \"...\", expected a map"), where the useful part is at the end: when it is
/// longer than 100 kept characters, keeps the first 60, `…` and the last 35.
pub fn shown_head_tail(value: &str) -> String {
    let kept: Vec<char> = value.chars().filter(|c| !is_unsafe_text_char(*c)).collect();
    if kept.len() <= 100 {
        return kept.into_iter().collect();
    }
    let mut out: String = kept[..60].iter().collect();
    out.push('…');
    out.extend(&kept[kept.len() - 35..]);
    out
}

/// Windows device names, which can't be used as a file name (with or without an
/// extension, in any case), plus the superscript digit variants Windows also reserves.
fn is_reserved_name(comp: &str) -> bool {
    let stem = comp.split('.').next().unwrap_or("").trim_end_matches(' ');
    let lower = stem.to_lowercase();
    if matches!(lower.as_str(), "con" | "prn" | "aux" | "nul") {
        return true;
    }
    let mut chars = lower.chars();
    let prefix: String = chars.by_ref().take(3).collect();
    if prefix != "com" && prefix != "lpt" {
        return false;
    }
    match (chars.next(), chars.next()) {
        (Some(c), None) => matches!(c, '1'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}'),
        _ => false,
    }
}

/// Normalizes an untrusted relative path to `/`-separated form, or `None` if it
/// is unsafe (traversal, absolute, backslashes, colons, control characters, too long,
/// Windows reserved names, or a component ending in a dot or a space).
pub fn normalize_rel_path(raw: &str) -> Option<String> {
    if raw.is_empty()
        || raw.len() > 512
        || raw.starts_with('/')
        || raw
            .chars()
            .any(|c| (c as u32) < 0x20 || c == '\u{7f}' || c == '\\' || c == ':')
    {
        return None;
    }

    let mut parts = Vec::new();
    for comp in raw.split('/') {
        match comp {
            "" | "." => {}
            ".." => return None,
            c if c.len() > 255 => return None,
            c if c.ends_with('.') || c.ends_with(' ') || is_reserved_name(c) => return None,
            c => parts.push(c),
        }
    }

    if parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}

/// OS and editor junk that is skipped rather than rejected. `.` and `..` are never
/// ignored: they are traversal and `normalize_rel_path` refuses them.
pub fn is_ignored_entry(rel: &str) -> bool {
    let mut last = "";
    for comp in rel.split('/') {
        if comp == "." || comp == ".." {
            continue;
        }
        if comp.starts_with('.') || comp == "__MACOSX" {
            return true;
        }
        if !comp.is_empty() {
            last = comp;
        }
    }
    last.eq_ignore_ascii_case("Thumbs.db")
        || last.eq_ignore_ascii_case("desktop.ini")
        || is_editor_side_file(last)
}

/// Temporary and backup files that editors leave beside a file being saved
/// (Emacs, Vim, JetBrains), so a live edit never breaks the active theme.
fn is_editor_side_file(name: &str) -> bool {
    name.ends_with('~')
        || (name.len() >= 2 && name.starts_with('#') && name.ends_with('#'))
        || name.starts_with(".#")
        || name == "4913"
        || name.ends_with("___jb_tmp___")
        || name.ends_with("___jb_old___")
}

/// Case-insensitive file systems (Windows, macOS) silently merge names that differ
/// only by case, in files and in folders. Takes normalized relative file paths and
/// returns a message naming the first clash.
pub(crate) fn check_case_collisions<'a>(
    rels: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    let mut seen_files = HashSet::new();
    let mut folders: HashMap<String, String> = HashMap::new();
    for rel in rels {
        if !seen_files.insert(rel.to_lowercase()) {
            return Err(format!("the theme has two files named {}", shown(rel)));
        }
        for (at, _) in rel.match_indices('/') {
            let folder = &rel[..at];
            match folders.entry(folder.to_lowercase()) {
                Entry::Occupied(e) if e.get() != folder => {
                    return Err(format!("the theme has two folders named {}", shown(folder)));
                }
                Entry::Occupied(_) => {}
                Entry::Vacant(e) => {
                    e.insert(folder.to_string());
                }
            }
        }
    }
    if let Some(clash) = seen_files.iter().find(|f| folders.contains_key(*f)) {
        return Err(format!(
            "the theme has a file and a folder named {}",
            shown(clash)
        ));
    }
    Ok(())
}

/// A `.txt` file in a theme is at most `max_text_bytes` long.
pub(crate) fn check_text_size(rel: &str, len: u64, limits: &ThemeLimits) -> Result<(), String> {
    if file_kind(rel) == Some(FileKind::Text) && len > limits.max_text_bytes {
        return Err(format!(
            "{} is larger than {} KB, the most a text file in a theme can be",
            shown(rel),
            limits.max_text_bytes / 1024
        ));
    }
    Ok(())
}

pub fn is_valid_theme_id(id: &str) -> bool {
    !id.is_empty()
        && !is_reserved_name(id)
        && id.len() <= 64
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[derive(Debug)]
pub struct ThemeFiles {
    /// Relative path to byte length.
    pub files: BTreeMap<String, u64>,
    pub fingerprint: u64,
}

impl ThemeFiles {
    pub fn kind_of(&self, rel: &str) -> Option<FileKind> {
        if self.files.contains_key(rel) {
            file_kind(rel)
        } else {
            None
        }
    }
}

fn invalid(msg: String) -> Failure {
    Failure::Theme(ThemeError::Invalid(msg))
}

/// Why reading a theme failed: a defect in the theme, or a disk problem that
/// says nothing about the theme. Internal: only `ThemeError` is public, so the
/// installer can turn `Io` into a plain error while everyone else sees
/// "invalid".
#[derive(Debug)]
pub(crate) enum Failure {
    Theme(ThemeError),
    Io(String),
}

impl Failure {
    pub(crate) fn into_theme_error(self) -> ThemeError {
        match self {
            Failure::Theme(e) => e,
            Failure::Io(m) => ThemeError::Invalid(m),
        }
    }

    pub(crate) fn into_anyhow(self) -> anyhow::Error {
        match self {
            Failure::Theme(e) => e.into(),
            Failure::Io(m) => anyhow::anyhow!(m),
        }
    }
}

impl From<ThemeError> for Failure {
    fn from(e: ThemeError) -> Self {
        Failure::Theme(e)
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Theme(e) => e.fmt(f),
            Failure::Io(m) => f.write_str(m),
        }
    }
}

/// A walkdir error as text without the absolute path its own Display embeds.
fn walk_failure(dir: &Path, e: &walkdir::Error) -> Failure {
    let at = e
        .path()
        .and_then(|p| p.strip_prefix(dir).ok())
        .map(|p| shown(&p.to_string_lossy().replace('\\', "/")))
        .filter(|p| !p.is_empty());
    let kind = e
        .io_error()
        .map(|io| io.kind().to_string())
        .unwrap_or_else(|| "unknown error".to_string());
    Failure::Io(match at {
        Some(at) => format!("couldn't read {at} in the theme folder: {kind}"),
        None => format!("couldn't read the theme folder: {kind}"),
    })
}

pub fn scan_theme_dir(dir: &Path, limits: &ThemeLimits) -> Result<ThemeFiles, ThemeError> {
    scan_theme_dir_checked(dir, limits).map_err(Failure::into_theme_error)
}

pub(crate) fn scan_theme_dir_checked(
    dir: &Path,
    limits: &ThemeLimits,
) -> Result<ThemeFiles, Failure> {
    let rel_of = |path: &Path| -> Option<String> {
        let rel = path.strip_prefix(dir).ok()?;
        let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
        Some(parts?.join("/"))
    };

    let walker = walkdir::WalkDir::new(dir)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0 || !rel_of(e.path()).is_some_and(|rel| is_ignored_entry(&rel))
        });

    let mut files = BTreeMap::new();
    let mut hashed: Vec<(String, u64, u128)> = Vec::new();
    let mut total: u64 = 0;

    for entry in walker {
        let entry = entry.map_err(|e| walk_failure(dir, &e))?;
        if entry.depth() == 0 {
            continue;
        }

        let rel = rel_of(entry.path())
            .and_then(|r| normalize_rel_path(&r))
            .ok_or_else(|| {
                let rel = entry.path().strip_prefix(dir).unwrap_or(entry.path());
                invalid(format!(
                    "this file name isn't allowed: {}",
                    shown(&rel.to_string_lossy().replace('\\', "/"))
                ))
            })?;

        let ft = entry.file_type();
        if ft.is_symlink() {
            return Err(invalid(format!("symlinks aren't allowed: {rel}")));
        }
        if ft.is_dir() {
            continue;
        }
        if !ft.is_file() || file_kind(&rel).is_none() {
            return Err(invalid(format!("this file type isn't allowed: {rel}")));
        }

        if files.len() + 1 > limits.max_files {
            return Err(invalid(format!(
                "the theme has more than {} files",
                limits.max_files
            )));
        }

        let meta = entry.metadata().map_err(|e| {
            let kind = e
                .io_error()
                .map(|io| io.kind().to_string())
                .unwrap_or_else(|| "unknown error".to_string());
            Failure::Io(format!("couldn't read {rel}: {kind}"))
        })?;
        let len = meta.len();
        check_text_size(&rel, len, limits).map_err(invalid)?;
        total = total.saturating_add(len);
        if total > limits.max_total_bytes {
            return Err(invalid(format!(
                "the theme is larger than {} bytes",
                limits.max_total_bytes
            )));
        }

        let mtime = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        hashed.push((rel.clone(), len, mtime));
        files.insert(rel, len);
    }

    if !files.contains_key("manifest.json") {
        return Err(invalid("manifest.json is missing".to_string()));
    }
    check_case_collisions(files.keys().map(String::as_str)).map_err(invalid)?;

    hashed.sort();
    let mut hasher = DefaultHasher::new();
    hashed.hash(&mut hasher);

    Ok(ThemeFiles {
        files,
        fingerprint: hasher.finish(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn clean_text_keeps_joiners_but_not_bidi_controls() {
        let bear = "\u{1F43B}\u{200D}\u{2744}\u{FE0F} Polar Night";
        assert_eq!(clean_text(bear), bear);
        assert_eq!(clean_text("a\u{200C}b"), "a\u{200C}b");
        assert_eq!(clean_text("a\u{202E}b\u{200B}c\u{200E}d"), "abcd");
        assert_eq!(shown(bear), "\u{1F43B}\u{2744}\u{FE0F} Polar Night");
        assert!(!shown("a\u{202E}b").contains('\u{202E}'));
        assert!(is_blank_text(" \u{200D}\u{200C} "));
        assert!(!is_blank_text("\u{200D}a"));
    }

    #[test]
    fn shown_drops_format_characters() {
        let s = shown(
            "a\u{202e}b\u{200b}c\u{200f}d\u{2066}e\u{2069}f\u{feff}g\u{ad}h\u{e0041}i\u{2028}j",
        );
        assert_eq!(s, "abcdefghij");
        // the cap counts what is kept, not what was removed
        let long = format!("\u{202e}{}", "x".repeat(150));
        assert_eq!(shown(&long).chars().count(), 101);
        assert!(shown(&long).ends_with('…'));
    }

    #[test]
    fn normalizes_relative_paths() {
        assert_eq!(
            normalize_rel_path("./assets/a.png").as_deref(),
            Some("assets/a.png")
        );
        assert_eq!(
            normalize_rel_path("assets/My Font.woff2").as_deref(),
            Some("assets/My Font.woff2")
        );
        assert_eq!(
            normalize_rel_path("assets/❄.svg").as_deref(),
            Some("assets/❄.svg")
        );
    }

    #[test]
    fn rejects_unsafe_paths() {
        for p in [
            "../x.png",
            "assets/../../x",
            "/etc/passwd",
            "C:/x.png",
            "assets\\x.png",
            "",
            "a\u{0}b",
        ] {
            assert_eq!(normalize_rel_path(p), None, "{p:?}");
        }
        assert_eq!(normalize_rel_path(&"a".repeat(513)), None);
        assert_eq!(normalize_rel_path(&"a".repeat(256)), None);
    }

    #[test]
    fn traversal_is_not_an_ignored_entry() {
        for p in ["../x.png", "a/../../x.png"] {
            assert!(!is_ignored_entry(p), "{p}");
            assert_eq!(normalize_rel_path(p), None, "{p}");
        }
        assert!(is_ignored_entry(".DS_Store"));
        assert!(is_ignored_entry("__MACOSX/._a.png"));
        assert!(is_ignored_entry("a/thumbs.DB"));
        assert!(is_ignored_entry("Desktop.ini"));
        assert!(!is_ignored_entry("assets/a.png"));
    }

    #[test]
    fn classifies_files() {
        assert_eq!(file_kind("manifest.json"), Some(FileKind::Manifest));
        assert_eq!(file_kind("assets/manifest.json"), None);
        assert_eq!(file_kind("theme.css"), Some(FileKind::Css));
        assert_eq!(file_kind("assets/A.PNG"), Some(FileKind::Image));
        assert_eq!(file_kind("assets/x.woff2"), Some(FileKind::Font));
        assert_eq!(file_kind("a.ogg"), Some(FileKind::Audio));
        assert_eq!(file_kind("a.mp4"), Some(FileKind::Video));
        assert_eq!(file_kind("x.js"), None);
        assert_eq!(file_kind("x.html"), None);
        assert_eq!(file_kind("noext"), None);
        assert_eq!(content_type("a.svg"), Some("image/svg+xml"));
        assert_eq!(content_type("a.JPG"), Some("image/jpeg"));
        assert_eq!(content_type("theme.css"), None);
        assert!(FileKind::Font.is_asset());
        assert!(!FileKind::Css.is_asset());
    }

    #[test]
    fn validates_theme_ids() {
        for ok in ["silent-night", "a1"] {
            assert!(is_valid_theme_id(ok), "{ok}");
        }
        let long = "a".repeat(65);
        for bad in ["", "Silent", "-a", "a-", "a--b", "a_b", long.as_str()] {
            assert!(!is_valid_theme_id(bad), "{bad}");
        }
        assert!(is_valid_theme_id(&"a".repeat(64)));
    }

    #[test]
    fn refuses_windows_reserved_and_trailing_dot_or_space() {
        for p in [
            "con",
            "CON.png",
            "assets/nul.svg",
            "assets/Aux.tar.png",
            "prn.css",
            "com1.png",
            "COM9.png",
            "lpt1.png",
            "a/LPT9.woff",
            "com\u{b9}.png",
            "lpt\u{b2}",
            "com\u{b3}.png",
            "a.",
            "assets/a.png.",
            "assets /a.png",
            "assets/a.png ",
        ] {
            assert_eq!(normalize_rel_path(p), None, "{p:?}");
        }
        for p in [
            "console.png",
            "com0.png",
            "com10.png",
            "assets/a b.png",
            "lpt.png",
        ] {
            assert!(normalize_rel_path(p).is_some(), "{p:?}");
        }
        for bad in ["con", "nul", "com1", "lpt9", "aux", "prn"] {
            assert!(!is_valid_theme_id(bad), "{bad}");
        }
        assert!(is_valid_theme_id("console"));
    }

    fn valid_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        fs::write(p.join("manifest.json"), "{}").unwrap();
        fs::write(p.join("theme.css"), "a{}").unwrap();
        fs::create_dir(p.join("assets")).unwrap();
        fs::write(p.join("assets/a.png"), "x").unwrap();
        fs::write(p.join(".DS_Store"), "x").unwrap();
        fs::create_dir(p.join("__MACOSX")).unwrap();
        fs::write(p.join("__MACOSX/._a.png"), "x").unwrap();
        fs::write(p.join("Thumbs.db"), "x").unwrap();
        d
    }

    #[test]
    fn scans_a_valid_folder_and_ignores_junk() {
        let d = valid_dir();
        let t = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT).unwrap();
        let keys: Vec<_> = t.files.keys().cloned().collect();
        assert_eq!(keys, ["assets/a.png", "manifest.json", "theme.css"]);
        assert_eq!(t.kind_of("theme.css"), Some(FileKind::Css));
        assert_eq!(t.kind_of("nope.css"), None);
    }

    #[test]
    fn refuses_disallowed_files() {
        let d = valid_dir();
        fs::write(d.path().join("evil.js"), "x").unwrap();
        let e = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT).unwrap_err();
        assert!(e.to_string().contains("evil.js"), "{e}");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinks() {
        let d = valid_dir();
        std::os::unix::fs::symlink("/etc/hosts", d.path().join("assets/l.png")).unwrap();
        let e = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT).unwrap_err();
        assert!(e.to_string().contains("symlink"), "{e}");
    }

    #[test]
    fn enforces_count_and_size_limits() {
        let d = valid_dir();
        let l = ThemeLimits {
            max_files: 2,
            ..ThemeLimits::DEFAULT
        };
        assert!(scan_theme_dir(d.path(), &l).is_err());
        let d = valid_dir();
        fs::write(d.path().join("assets/big.png"), [0u8; 20]).unwrap();
        let l = ThemeLimits {
            max_total_bytes: 10,
            ..ThemeLimits::DEFAULT
        };
        assert!(scan_theme_dir(d.path(), &l).is_err());
    }

    #[test]
    fn keeps_a_licence_text_file_but_never_as_an_asset() {
        assert_eq!(file_kind("fonts/OFL.txt"), Some(FileKind::Text));
        assert_eq!(file_kind("LICENSE.TXT"), Some(FileKind::Text));
        assert!(!FileKind::Text.is_asset());
        assert_eq!(content_type("fonts/OFL.txt"), None);

        let d = valid_dir();
        fs::create_dir(d.path().join("fonts")).unwrap();
        fs::write(d.path().join("fonts/OFL.txt"), "SIL Open Font License").unwrap();
        let files = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT).unwrap();
        assert_eq!(files.kind_of("fonts/OFL.txt"), Some(FileKind::Text));
    }

    #[test]
    fn refuses_a_text_file_over_its_size_cap() {
        let limits = ThemeLimits::DEFAULT;
        assert_eq!(limits.max_text_bytes, 256 * 1024);
        let d = valid_dir();
        let at_cap = vec![b'a'; limits.max_text_bytes as usize];
        fs::write(d.path().join("OFL.txt"), &at_cap).unwrap();
        assert!(scan_theme_dir(d.path(), &limits).is_ok());

        fs::write(d.path().join("OFL.txt"), [at_cap, vec![b'a']].concat()).unwrap();
        let e = scan_theme_dir(d.path(), &limits).unwrap_err().to_string();
        assert!(e.contains("OFL.txt is larger than 256 KB"), "{e}");
    }

    #[test]
    fn fingerprint_changes_when_a_file_changes() {
        let d = valid_dir();
        let a = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT)
            .unwrap()
            .fingerprint;
        let b = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT)
            .unwrap()
            .fingerprint;
        assert_eq!(a, b);
        fs::write(d.path().join("theme.css"), "a{color:red}").unwrap();
        let c = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT)
            .unwrap()
            .fingerprint;
        assert_ne!(a, c);
    }

    #[test]
    fn missing_manifest_is_invalid() {
        let d = valid_dir();
        fs::remove_file(d.path().join("manifest.json")).unwrap();
        let e = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT).unwrap_err();
        assert!(e.to_string().contains("manifest.json is missing"), "{e}");
    }

    #[test]
    fn editor_side_files_are_ignored() {
        for n in [
            "theme.css~",
            "a/b.png~",
            "#theme.css#",
            ".#theme.css",
            "4913",
            "assets/4913",
            "x.css___jb_tmp___",
            "x.css___jb_old___",
        ] {
            assert!(is_ignored_entry(n), "{n}");
        }
        for n in [
            "theme.css",
            "a#b.css",
            "#a.css",
            "49130",
            "a/4913.png",
            "a~b.css",
        ] {
            assert!(!is_ignored_entry(n), "{n}");
        }
    }

    #[test]
    fn scan_errors_never_show_absolute_paths() {
        let d = valid_dir();
        // DEL is a legal file name character on every platform but is refused
        // by the name check, and is stripped from the shown name.
        fs::write(d.path().join("assets/bad\u{7f}name.png"), "x").unwrap();
        let e = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT)
            .unwrap_err()
            .to_string();
        assert!(e.contains("assets/badname.png"), "{e}");
        assert!(!e.contains(&*d.path().to_string_lossy()), "{e}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn hand_made_folders_with_case_collisions_are_refused() {
        let d = valid_dir();
        fs::create_dir_all(d.path().join("Assets")).unwrap();
        fs::write(d.path().join("Assets/b.png"), "x").unwrap();
        let e = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT)
            .unwrap_err()
            .to_string();
        assert!(e.contains("two folders named"), "{e}");

        let d = valid_dir();
        fs::write(d.path().join("assets/Icon.png"), "x").unwrap();
        fs::write(d.path().join("assets/icon.png"), "x").unwrap();
        let e = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT)
            .unwrap_err()
            .to_string();
        assert!(e.contains("two files named"), "{e}");
    }

    #[test]
    fn every_allowed_asset_extension_has_a_content_type() {
        for ext in [
            "png", "jpg", "jpeg", "webp", "avif", "gif", "svg", "woff2", "woff", "ttf", "otf",
            "ogg", "mp3", "wav", "webm", "mp4",
        ] {
            let name = format!("a.{ext}");
            assert!(file_kind(&name).is_some_and(FileKind::is_asset), "{ext}");
            assert!(content_type(&name).is_some(), "{ext}");
        }
        for ext in ["json", "css", "exe", "html", "js", "txt"] {
            assert_eq!(content_type(&format!("a.{ext}")), None, "{ext}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_folder_errors_never_show_absolute_paths() {
        use std::os::unix::fs::PermissionsExt;
        let d = valid_dir();
        let locked = d.path().join("assets");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let r = scan_theme_dir(d.path(), &ThemeLimits::DEFAULT);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        // Running as root can read it anyway, which leaves nothing to check.
        if let Err(e) = r {
            let e = e.to_string();
            assert!(!e.contains(&*d.path().to_string_lossy()), "{e}");
        }
    }

    #[test]
    fn walk_errors_are_shown_without_the_absolute_path() {
        let d = tempfile::tempdir().unwrap();
        let missing = d.path().join("gone");
        let err = walkdir::WalkDir::new(&missing)
            .into_iter()
            .next()
            .unwrap()
            .unwrap_err();
        // walkdir's own text is the thing to avoid.
        assert!(err.to_string().contains(&*missing.to_string_lossy()));
        let shown_text = walk_failure(d.path(), &err).to_string();
        assert!(shown_text.contains("gone"), "{shown_text}");
        assert!(
            !shown_text.contains(&*d.path().to_string_lossy()),
            "{shown_text}"
        );
        let shown_text = walk_failure(Path::new("/elsewhere"), &err).to_string();
        assert!(
            !shown_text.contains(&*d.path().to_string_lossy()),
            "{shown_text}"
        );
    }
}
