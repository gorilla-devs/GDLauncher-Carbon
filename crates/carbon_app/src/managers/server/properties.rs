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
