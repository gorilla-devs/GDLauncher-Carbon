use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::warn;

/// Whether a server.properties key or value contains a raw newline. A `\n`
/// embedded in a value turns one `key=value` pair into two lines once the
/// file's lines are joined, letting an attacker-controlled value inject an
/// arbitrary second property line (e.g. a hidden `online-mode=false`
/// disabling Mojang auth). The API boundary (`update_server_properties`'s
/// caller) is expected to reject these outright; this is the defensive,
/// last-line-of-defense check for whatever reaches this file format.
fn has_control_chars(s: &str) -> bool {
    s.contains('\n') || s.contains('\r')
}

/// Generate a server.properties file content from server settings
pub fn generate_properties(
    port: i32,
    motd: &str,
    max_players: i32,
    online_mode: bool,
    game_version: &str,
) -> String {
    let mut props = BTreeMap::new();
    props.insert("server-port", port.to_string());
    props.insert("motd", motd.to_string());
    props.insert("max-players", max_players.to_string());
    props.insert("online-mode", online_mode.to_string());
    props.insert("enable-command-block", "true".to_string());
    props.insert("spawn-protection", "0".to_string());

    // Pre-1.14 servers regularly blow past the watchdog's 60s single-tick
    // limit while generating the world on first boot, and the watchdog then
    // kills the server. Disable it for those versions.
    if is_pre_1_14(game_version) {
        props.insert("max-tick-time", "-1".to_string());
    }

    let mut output = String::from("#Minecraft server properties\n");
    for (key, value) in &props {
        output.push_str(&format!("{}={}\n", key, value));
    }
    output
}

/// Whether the given release version (e.g. "1.12.2") is below 1.14. Returns
/// false for snapshots and other non-release version strings.
fn is_pre_1_14(game_version: &str) -> bool {
    let mut parts = game_version.split('.');
    let (Some(major), Some(minor)) = (parts.next(), parts.next()) else {
        return false;
    };
    let (Ok(major), Ok(minor)) = (major.parse::<u32>(), minor.parse::<u32>()) else {
        return false;
    };
    major == 1 && minor < 14
}

/// Parse a server.properties file into a key-value map
pub fn parse_properties(content: &str) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            props.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    props
}

/// `server.properties` bytes as text the way the server decodes them: UTF-8,
/// else ISO-8859-1, which Minecraft before 1.20 always uses and later versions
/// fall back to. `true` with the text when it was ISO-8859-1.
pub fn decode(bytes: Vec<u8>) -> (String, bool) {
    match String::from_utf8(bytes) {
        Ok(text) => (text, false),
        Err(error) => (
            error.into_bytes().into_iter().map(char::from).collect(),
            true,
        ),
    }
}

/// `text` in the encoding [`decode`] found. A character ISO-8859-1 cannot hold
/// is written as the `\uXXXX` escape the server reads back as that character.
fn encode(text: &str, latin1: bool) -> Vec<u8> {
    if !latin1 {
        return text.as_bytes().to_vec();
    }
    let mut bytes = Vec::with_capacity(text.len());
    for c in text.chars() {
        match u8::try_from(u32::from(c)) {
            Ok(byte) => bytes.push(byte),
            Err(_) => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    bytes.extend(format!("\\u{unit:04X}").into_bytes());
                }
            }
        }
    }
    bytes
}

const JAVA_WHITESPACE: [char; 3] = [' ', '\t', '\x0c'];

/// The properties in `content` as `java.util.Properties.load` reads them,
/// which is how the server reads `server.properties`: `#` and `!` comments,
/// `=`, `:` or whitespace between key and value, lines continued by a
/// trailing `\`, and `\` escapes. The server saves a world named `World #2`
/// as `level-name=World \#2`, and older versions save `Größe` as
/// `Gr\u00F6\u00DFe`. [`parse_properties`] keeps the text as it is written,
/// for the Properties tab.
pub fn java_properties(content: &str) -> BTreeMap<String, String> {
    let content = content.replace("\r\n", "\n");
    let mut lines = content.split(['\n', '\r']);
    let mut props = BTreeMap::new();
    while let Some(line) = lines.next() {
        let line = line.trim_start_matches(JAVA_WHITESPACE);
        if line.is_empty() || line.starts_with(['#', '!']) {
            continue;
        }
        let mut logical = line.to_owned();
        while logical.chars().rev().take_while(|c| *c == '\\').count() % 2 == 1 {
            logical.pop();
            match lines.next() {
                Some(next) => logical.push_str(next.trim_start_matches(JAVA_WHITESPACE)),
                None => break,
            }
        }
        let (key, value) = split_java_line(&logical);
        props.insert(java_unescape(key), java_unescape(value));
    }
    props
}

/// The key and the value of one logical properties line, still escaped.
fn split_java_line(line: &str) -> (&str, &str) {
    let mut escaped = false;
    let mut separator = None;
    for (index, c) in line.char_indices() {
        if !escaped && (c == '=' || c == ':' || JAVA_WHITESPACE.contains(&c)) {
            separator = Some((index, c));
            break;
        }
        escaped = c == '\\' && !escaped;
    }
    let Some((index, c)) = separator else {
        return (line, "");
    };
    let key = &line[..index];
    let mut rest = &line[index + c.len_utf8()..];
    let mut has_separator = c == '=' || c == ':';
    loop {
        rest = rest.trim_start_matches(JAVA_WHITESPACE);
        match rest.strip_prefix(['=', ':']) {
            Some(after) if !has_separator => {
                has_separator = true;
                rest = after;
            }
            _ => return (key, rest),
        }
    }
}

/// `\uXXXX`, `\t`, `\n`, `\r`, `\f` and `\` before any other character, as
/// `Properties.load` decodes them. A malformed `\u`, which the server refuses
/// to load, is read as `u` and what follows.
fn java_unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut units: Vec<u16> = Vec::new();
    let mut chars = text.chars();
    // A character outside the BMP is two escapes, one per UTF-16 unit.
    let flush = |units: &mut Vec<u16>, out: &mut String| {
        out.extend(
            char::decode_utf16(units.drain(..)).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)),
        );
    };
    while let Some(c) = chars.next() {
        if c != '\\' {
            flush(&mut units, &mut out);
            out.push(c);
            continue;
        }
        let Some(next) = chars.next() else {
            break;
        };
        if next == 'u' {
            let hex: String = chars.clone().take(4).collect();
            if hex.len() == 4 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Ok(unit) = u16::from_str_radix(&hex, 16) {
                    chars.nth(3);
                    units.push(unit);
                    continue;
                }
            }
        }
        flush(&mut units, &mut out);
        out.push(match next {
            't' => '\t',
            'n' => '\n',
            'r' => '\r',
            'f' => '\x0c',
            other => other,
        });
    }
    flush(&mut units, &mut out);
    out
}

/// What the server reads as `value` when GDLauncher writes `key=value` (see
/// [`update_properties`]), or `None` when a `\` in it is not one of the
/// escapes the server writes itself (`\#`, `\!`, `\=`, `\:`, `\ `,
/// `\\`, `\uXXXX`): the server would drop it, so `worlds\survival` would
/// name `worldssurvival`.
pub fn written_value(value: &str) -> Option<String> {
    let mut rest = value;
    while let Some(index) = rest.find('\\') {
        let after = &rest[index + 1..];
        let escape = after.chars().next();
        let known = match escape {
            Some('#' | '!' | '=' | ':' | ' ' | '\\') => true,
            Some('u') => after
                .get(1..5)
                .is_some_and(|hex| hex.chars().all(|c| c.is_ascii_hexdigit())),
            _ => false,
        };
        if !known {
            return None;
        }
        rest = &after[escape.map_or(0, char::len_utf8)..];
    }
    java_properties(&format!("k={value}")).remove("k")
}

/// Update specific properties in an existing server.properties file. Any key
/// or value containing `\n`/`\r` is dropped rather than written — the API
/// boundary is expected to reject these before they ever reach here, so a
/// dropped entry means that boundary was bypassed.
pub fn update_properties(existing_content: &str, updates: &BTreeMap<String, String>) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut updated_keys = std::collections::HashSet::new();

    for line in existing_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            lines.push(line.to_string());
            continue;
        }

        if let Some((key, _)) = trimmed.split_once('=') {
            let key = key.trim();
            if let Some(new_value) = updates.get(key) {
                if has_control_chars(key) || has_control_chars(new_value) {
                    warn!(
                        "Refusing to write server.properties key `{key}`: value contains a newline"
                    );
                    lines.push(line.to_string());
                } else {
                    lines.push(format!("{}={}", key, new_value));
                    updated_keys.insert(key.to_string());
                }
            } else {
                lines.push(line.to_string());
            }
        } else {
            lines.push(line.to_string());
        }
    }

    // Append any new keys that weren't in the original file
    for (key, value) in updates {
        if !updated_keys.contains(key.as_str()) {
            if has_control_chars(key) || has_control_chars(value) {
                warn!("Refusing to write server.properties key `{key}`: value contains a newline");
                continue;
            }
            lines.push(format!("{}={}", key, value));
        }
    }

    lines.join("\n") + "\n"
}

/// Write server.properties to disk
pub async fn write_properties(path: &Path, content: &str) -> Result<()> {
    tokio::fs::write(path, content)
        .await
        .context("Failed to write server.properties")
}

/// Read server.properties from disk
pub async fn read_properties(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read server.properties")?;
    Ok(parse_properties(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_watchdog_disabled_only_below_1_14() {
        assert!(is_pre_1_14("1.7.10"));
        assert!(is_pre_1_14("1.8"));
        assert!(is_pre_1_14("1.12.2"));
        assert!(is_pre_1_14("1.13.2"));

        assert!(!is_pre_1_14("1.14"));
        assert!(!is_pre_1_14("1.14.4"));
        assert!(!is_pre_1_14("1.20.1"));

        // Snapshots and other non-release strings keep the default watchdog
        assert!(!is_pre_1_14("18w50a"));
        assert!(!is_pre_1_14("unknown"));

        let props = parse_properties(&generate_properties(25565, "motd", 20, true, "1.12.2"));
        assert_eq!(props.get("max-tick-time").map(String::as_str), Some("-1"));

        let props = parse_properties(&generate_properties(25565, "motd", 20, true, "1.20.1"));
        assert_eq!(props.get("max-tick-time"), None);
    }

    #[test]
    fn update_properties_rejects_newline_injection_in_an_existing_key() {
        let existing = "#Minecraft server properties\nmotd=Hello\nonline-mode=true\n";
        let mut updates = BTreeMap::new();
        updates.insert(
            "motd".to_string(),
            "Hacked\r\nonline-mode=false".to_string(),
        );

        let updated = update_properties(existing, &updates);

        // The malicious value must not have produced a second `online-mode`
        // line — the original `online-mode=true` must be the only one.
        let online_mode_lines: Vec<&str> = updated
            .lines()
            .filter(|l| l.trim_start().starts_with("online-mode"))
            .collect();
        assert_eq!(online_mode_lines, vec!["online-mode=true"]);
        assert!(!updated.contains("online-mode=false"));

        // The whole malicious update is dropped, not partially applied —
        // motd is left at its previous value.
        assert!(updated.contains("motd=Hello"));
        assert!(!updated.contains("Hacked"));
    }

    #[test]
    fn update_properties_rejects_newline_injection_in_a_new_key() {
        let existing = "#Minecraft server properties\nmotd=Hello\n";
        let mut updates = BTreeMap::new();
        updates.insert(
            "new-key".to_string(),
            "value\nonline-mode=false".to_string(),
        );

        let updated = update_properties(existing, &updates);

        assert!(!updated.contains("online-mode=false"));
        assert!(!updated.contains("new-key="));
    }

    #[test]
    fn update_properties_rejects_newline_in_the_key_itself() {
        let existing = "#Minecraft server properties\nmotd=Hello\n";
        let mut updates = BTreeMap::new();
        updates.insert("evil\nonline-mode".to_string(), "false".to_string());

        let updated = update_properties(existing, &updates);

        assert!(!updated.contains("online-mode=false"));
    }

    #[test]
    fn update_properties_from_empty_content_sanitizes_new_file_writes() {
        // `update_server_properties`'s create-new-file branch calls
        // `update_properties("", pairs)` so a from-scratch write goes through
        // the same control-char filter as an update to an existing file.
        let mut updates = BTreeMap::new();
        updates.insert("motd".to_string(), "x\nonline-mode=false".to_string());

        let updated = update_properties("", &updates);

        // The malicious value is dropped outright rather than partially
        // applied — the fresh file ends up empty, not holding the sanitized
        // `motd` plus a second, injected `online-mode` line.
        assert_eq!(updated, "\n");
        assert!(!updated.contains("online-mode"));

        // A clean value alongside a malicious one still lands as exactly one
        // sanitized line — the filter drops only the offending pair.
        let mut mixed = BTreeMap::new();
        mixed.insert("motd".to_string(), "x\nonline-mode=false".to_string());
        mixed.insert("max-players".to_string(), "10".to_string());

        let updated = update_properties("", &mixed);
        assert_eq!(updated.lines().collect::<Vec<_>>(), vec!["max-players=10"]);
    }

    #[test]
    fn update_properties_still_applies_clean_values() {
        let existing = "#Minecraft server properties\nmotd=Hello\n";
        let mut updates = BTreeMap::new();
        updates.insert("motd".to_string(), "A perfectly normal MOTD".to_string());
        updates.insert("max-players".to_string(), "10".to_string());

        let updated = update_properties(existing, &updates);
        let props = parse_properties(&updated);

        assert_eq!(
            props.get("motd").map(String::as_str),
            Some("A perfectly normal MOTD")
        );
        assert_eq!(props.get("max-players").map(String::as_str), Some("10"));
    }
}

/// Finalize pack properties before publishing the effective port to the DB.
///
/// A `server-port` the server cannot use (blank, not a number, out of range:
/// a pack's placeholder, or a hand edit) is set to `fallback_port`, the port
/// GDLauncher shows for the server, rather than failing the install at its
/// last step; the server would otherwise listen on its default port while
/// GDLauncher showed another. A file that is not UTF-8 is read and written as
/// ISO-8859-1, as the server reads it.
pub async fn finalize_pack_properties(
    path: &Path,
    requested_port: Option<i32>,
    fallback_port: i32,
    game_version: &str,
    repair: bool,
) -> Result<i32> {
    if !path.exists() {
        write_properties(
            path,
            &generate_properties(fallback_port, "A Minecraft Server", 20, true, game_version),
        )
        .await?;
    }
    let bytes = tokio::fs::read(path)
        .await
        .context("Failed to read server.properties")?;
    let (current, latin1) = decode(bytes);
    let set_port = |port: i32| {
        let updates = BTreeMap::from([("server-port".into(), port.to_string())]);
        let content = encode(&update_properties(&current, &updates), latin1);
        async move {
            tokio::fs::write(path, content)
                .await
                .context("Failed to write server.properties")
        }
    };
    if let Some(port) = requested_port.filter(|_| !repair) {
        super::resolve_server_port(Some(port))?;
        set_port(port).await?;
        return Ok(port);
    }
    match parse_properties(&current).get("server-port") {
        None => Ok(fallback_port),
        Some(value) => match super::validate_server_port_patch(value) {
            Ok(port) => Ok(port),
            Err(error) => {
                warn!(
                    "server.properties has an unusable server-port ({error}); setting it to {fallback_port}"
                );
                set_port(fallback_port).await?;
                Ok(fallback_port)
            }
        },
    }
}

#[cfg(test)]
mod task07_port_tests {
    use super::*;
    #[tokio::test]
    async fn task07_pack_port_preserves_unspecified_explicit_and_repair_intent() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("server.properties");
        std::fs::write(
            &path,
            "#operator\nserver-port=25577\nlevel-name=AdventureMap\n",
        )
        .unwrap();
        assert_eq!(
            finalize_pack_properties(&path, None, 25565, "1.21", false)
                .await
                .unwrap(),
            25577
        );
        assert_eq!(
            finalize_pack_properties(&path, Some(25588), 25588, "1.21", false)
                .await
                .unwrap(),
            25588
        );
        assert_eq!(
            read_properties(&path).await.unwrap()["server-port"],
            "25588"
        );
        assert_eq!(
            finalize_pack_properties(&path, Some(25599), 25599, "1.21", true)
                .await
                .unwrap(),
            25588
        );
        assert_eq!(
            read_properties(&path).await.unwrap()["level-name"],
            "AdventureMap"
        );
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            finalize_pack_properties(&path, None, 25565, "1.21", false)
                .await
                .unwrap(),
            25565
        );
    }
}

#[cfg(test)]
mod java_properties_tests {
    use super::*;

    /// The server saves `server.properties` with `Properties.store`, which
    /// escapes `#`, `!`, `=`, `:` and a leading space in values, and before
    /// 1.20 every non-ASCII character as `\uXXXX`. GDLauncher reads the world
    /// name the way the server does, so it finds the same folder.
    #[test]
    fn values_are_read_as_the_server_reads_them() {
        let content = "#Minecraft server properties\r\n\
             ! a comment too\n\
             level-name=World \\#2\n\
             motd=Hardcore\\!\n\
             a\\=b=c\\=d\n\
             accented=Gr\\u00F6\\u00DFe\n\
             emoji=\\uD83D\\uDE00\n\
             colon:value\n\
             spaced   value with spaces \n\
             leading=\\ space\n\
             continued=first \\\n    second\n\
             trailing=ends\\\\\n\
             malformed=\\uZZ\n\
             empty=\n";
        let props = java_properties(content);
        assert_eq!(props["level-name"], "World #2");
        assert_eq!(props["motd"], "Hardcore!");
        assert_eq!(props["a=b"], "c=d");
        assert_eq!(props["accented"], "Größe");
        assert_eq!(props["emoji"], "\u{1F600}");
        assert_eq!(props["colon"], "value");
        assert_eq!(props["spaced"], "value with spaces ");
        assert_eq!(props["leading"], " space");
        assert_eq!(props["continued"], "first second");
        assert_eq!(props["trailing"], "ends\\");
        assert_eq!(props["malformed"], "uZZ");
        assert_eq!(props["empty"], "");
        assert!(!props.keys().any(|key| key.contains("comment")));
        // The Properties tab keeps showing what is written.
        assert_eq!(parse_properties(content)["level-name"], "World \\#2");
        assert_eq!(written_value("World \\#3").unwrap(), "World #3");
        assert_eq!(written_value("Gr\\u00F6\\\\x").unwrap(), "Gr\u{F6}\\x");
        assert_eq!(written_value("Plain").unwrap(), "Plain");
        assert_eq!(written_value("worlds\\survival"), None);
        assert_eq!(written_value("bad\\u12"), None);
        assert_eq!(written_value("trailing\\"), None);
    }

    #[test]
    fn a_file_that_is_not_utf8_is_read_as_latin1() {
        let (text, latin1) = decode(b"level-name=Gr\xF6\xDFe\n".to_vec());
        assert!(latin1);
        assert_eq!(java_properties(&text)["level-name"], "Größe");
        assert_eq!(encode(&text, true), b"level-name=Gr\xF6\xDFe\n");
        assert_eq!(encode("\u{1F600}", true), b"\\uD83D\\uDE00");
        let (text, latin1) = decode("level-name=Größe\n".as_bytes().to_vec());
        assert!(!latin1);
        assert_eq!(java_properties(&text)["level-name"], "Größe");
    }

    /// A blank, placeholder or out-of-range `server-port` (in a pack's own
    /// file, or edited by hand since the server last ran) is set to the port
    /// GDLauncher shows instead of failing the install or Repair at its last
    /// step, and a file that is not UTF-8 keeps its encoding.
    #[tokio::test]
    async fn an_unusable_port_is_set_to_the_fallback_instead_of_failing() {
        for (value, repair) in [
            ("", false),
            ("", true),
            ("25565 # default", false),
            ("abc", true),
            ("70000", false),
        ] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("server.properties");
            std::fs::write(&path, format!("motd=Hi\nserver-port={value}\n")).unwrap();
            assert_eq!(
                finalize_pack_properties(&path, None, 25570, "1.21", repair)
                    .await
                    .unwrap(),
                25570,
                "{value:?} repair={repair}"
            );
            let props = read_properties(&path).await.unwrap();
            assert_eq!(props["server-port"], "25570", "{value:?}");
            assert_eq!(props["motd"], "Hi");
        }

        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("server.properties");
        std::fs::write(&path, b"motd=Gr\xF6\xDFe\nserver-port=\n").unwrap();
        assert_eq!(
            finalize_pack_properties(&path, None, 25570, "1.21", false)
                .await
                .unwrap(),
            25570
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"motd=Gr\xF6\xDFe\nserver-port=25570\n"
        );
        assert_eq!(
            finalize_pack_properties(&path, Some(25580), 25580, "1.21", false)
                .await
                .unwrap(),
            25580
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"motd=Gr\xF6\xDFe\nserver-port=25580\n"
        );
    }
}
