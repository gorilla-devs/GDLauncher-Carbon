use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

/// Generate a server.properties file content from server settings
pub fn generate_properties(
    port: i32,
    motd: &str,
    max_players: i32,
    online_mode: bool,
) -> String {
    let mut props = BTreeMap::new();
    props.insert("server-port", port.to_string());
    props.insert("motd", motd.to_string());
    props.insert("max-players", max_players.to_string());
    props.insert("online-mode", online_mode.to_string());
    props.insert("enable-command-block", "true".to_string());
    props.insert("spawn-protection", "0".to_string());

    let mut output = String::from("#Minecraft server properties\n");
    for (key, value) in &props {
        output.push_str(&format!("{}={}\n", key, value));
    }
    output
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
pub fn update_properties(
    existing_content: &str,
    updates: &BTreeMap<String, String>,
) -> String {
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
