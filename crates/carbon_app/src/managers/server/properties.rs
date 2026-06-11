use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

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

/// Update specific properties in an existing server.properties file
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
                lines.push(format!("{}={}", key, new_value));
                updated_keys.insert(key.to_string());
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
}
