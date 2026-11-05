//! `servers/<shortpath>/server.json`: the launcher's own settings for one
//! server, kept beside its `server/` data folder so the Minecraft server never
//! sees it.
//!
//! The database stays the source of truth. The file only lets a server come
//! back after its row is lost: a database reset (2.0.40's after a downgrade,
//! or this build's own startup error screens), a snapshot older than the
//! server, or a deleted `gdl_conf.db`. Instances get the same from their
//! `instance.json`.
//!
//! Readers ignore fields they do not know, so a later build may add fields
//! under the same `version`. A change older builds would misread bumps
//! `version`, and they then skip the file.

use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::server::{RestoredServer, ServerRow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const RECORD_FILE_NAME: &str = "server.json";
pub const RECORD_VERSION: u32 = 1;

/// Every stored setting of a server row except what the folder or the
/// library already gives: the shortpath is the folder's name, and the row
/// id, index, library position, last start and icon revision are the
/// library's own bookkeeping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRecord {
    pub version: u32,
    pub name: String,
    pub server_type: String,
    pub game_version: String,
    pub modloader_type: Option<String>,
    pub modloader_version: Option<String>,
    pub port: i32,
    pub motd: String,
    pub max_players: i32,
    pub online_mode: bool,
    pub xmx: i32,
    pub xms: i32,
    pub extra_java_args: String,
    pub auto_restart: bool,
    pub date_created: DateTime<Utc>,
    pub provider_type: String,
    pub hosted_server_id: Option<String>,
    pub modpack_platform: Option<String>,
    pub modpack_project_id: Option<String>,
    pub modpack_file_id: Option<String>,
    pub favorite: bool,
    /// The name of the folder the server is in; `None` for the default
    /// group, whose name a fresh database may not share.
    pub group: Option<String>,
}

impl ServerRecord {
    pub fn from_row(row: &ServerRow, group: Option<String>) -> Self {
        Self {
            version: RECORD_VERSION,
            name: row.name.clone(),
            server_type: row.server_type.clone(),
            game_version: row.game_version.clone(),
            modloader_type: row.modloader_type.clone(),
            modloader_version: row.modloader_version.clone(),
            port: row.port,
            motd: row.motd.clone(),
            max_players: row.max_players,
            online_mode: row.online_mode,
            xmx: row.xmx,
            xms: row.xms,
            extra_java_args: row.extra_java_args.clone(),
            auto_restart: row.auto_restart,
            date_created: row.date_created.with_timezone(&Utc),
            provider_type: row.provider_type.clone(),
            hosted_server_id: row.hosted_server_id.clone(),
            modpack_platform: row.modpack_platform.clone(),
            modpack_project_id: row.modpack_project_id.clone(),
            modpack_file_id: row.modpack_file_id.clone(),
            favorite: row.favorite,
            group,
        }
    }

    pub fn into_restored(self, shortpath: String, icon_revision: Option<i32>) -> RestoredServer {
        RestoredServer {
            name: self.name,
            shortpath,
            favorite: self.favorite,
            server_type: self.server_type,
            game_version: self.game_version,
            port: self.port,
            motd: self.motd,
            max_players: self.max_players,
            online_mode: self.online_mode,
            xmx: self.xmx,
            xms: self.xms,
            extra_java_args: self.extra_java_args,
            auto_restart: self.auto_restart,
            date_created: DbDateTime(self.date_created.into()),
            provider_type: self.provider_type,
            hosted_server_id: self.hosted_server_id,
            icon_revision,
            modloader_type: self.modloader_type,
            modloader_version: self.modloader_version,
            modpack_platform: self.modpack_platform,
            modpack_project_id: self.modpack_project_id,
            modpack_file_id: self.modpack_file_id,
            // A blank name is no folder the user could have made.
            group: self.group.filter(|name| !name.trim().is_empty()),
        }
    }

    pub fn to_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(self)
    }
}

pub fn record_path(server_root: &Path) -> PathBuf {
    server_root.join(RECORD_FILE_NAME)
}

/// What a record file holds. Anything but `Valid` leaves its folder alone.
#[derive(Debug)]
pub enum ParsedRecord {
    Valid(ServerRecord),
    Empty,
    Malformed(String),
    /// Written by a later build in a format this one does not know.
    Newer(u64),
    /// Well-formed, but without what a server needs.
    Incomplete(&'static str),
}

impl std::fmt::Display for ParsedRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid(_) => write!(f, "valid"),
            Self::Empty => write!(f, "the file is empty"),
            Self::Malformed(error) => write!(f, "the file cannot be read: {error}"),
            Self::Newer(version) => write!(
                f,
                "a newer GDLauncher wrote it (version {version}, this build reads up to {RECORD_VERSION})"
            ),
            Self::Incomplete(what) => write!(f, "the file has no {what}"),
        }
    }
}

pub fn parse_record(bytes: &[u8]) -> ParsedRecord {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return ParsedRecord::Empty;
    }
    let value: serde_json::Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(error) => return ParsedRecord::Malformed(error.to_string()),
    };
    match value.get("version").and_then(serde_json::Value::as_u64) {
        Some(version) if version > u64::from(RECORD_VERSION) => {
            return ParsedRecord::Newer(version);
        }
        Some(version) if version >= 1 => {}
        _ => return ParsedRecord::Malformed("missing or invalid \"version\"".into()),
    }
    let record: ServerRecord = match serde_json::from_value(value) {
        Ok(record) => record,
        Err(error) => return ParsedRecord::Malformed(error.to_string()),
    };
    if record.name.trim().is_empty() {
        return ParsedRecord::Incomplete("server name");
    }
    if record.game_version.trim().is_empty() {
        return ParsedRecord::Incomplete("game version");
    }
    if record.server_type.trim().is_empty() {
        return ParsedRecord::Incomplete("server type");
    }
    ParsedRecord::Valid(record)
}

/// A folder under `servers/` with a valid record and no row.
#[derive(Debug)]
pub struct UnlistedServer {
    pub shortpath: String,
    pub record: ServerRecord,
    pub has_icon: bool,
}

/// Lists the folders under `servers_dir` that hold a valid record and belong
/// to none of `shortpaths`. A folder matches a row when its name is the
/// row's shortpath or when `servers/<shortpath>` opens that same folder, as
/// it does on a file system that ignores case or Unicode normalization, or
/// when either of them is a link to the other: the rest of the server code
/// only ever reaches a folder through that path. A link to a folder (a
/// server kept on another drive) is followed, as the instance scan follows
/// one, and a folder two entries lead to is listed once, under its own name
/// when it has one under `servers_dir`. A folder that cannot be checked is
/// left alone, so a server is never listed twice. Blocking; nothing here
/// fails the caller.
pub fn find_unlisted_servers(servers_dir: &Path, shortpaths: &[String]) -> Vec<UnlistedServer> {
    let entries = match std::fs::read_dir(servers_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return vec![],
        Err(error) => {
            tracing::warn!(
                "Not looking for servers to restore: cannot list {}: {error}",
                servers_dir.display()
            );
            return vec![];
        }
    };
    let mut candidates = vec![];
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(
                    "Stopped looking for servers to restore in {}: {error}",
                    servers_dir.display()
                );
                break;
            }
        };
        // Not followed: a symbolic link or a Windows junction is a link.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let is_link = file_type.is_symlink();
        if file_type.is_dir() || is_link {
            candidates.push((is_link, entry.file_name(), entry.path()));
        }
    }
    // Real folders first, so a link to one of them never takes its place.
    candidates.sort();

    let names: HashSet<&str> = shortpaths.iter().map(String::as_str).collect();
    let mut listed: Option<Vec<same_file::Handle>> = None;
    let mut taken: Vec<(same_file::Handle, String)> = vec![];
    let mut unlisted = vec![];
    for (is_link, file_name, root) in candidates {
        let Some(shortpath) = file_name.to_str().map(str::to_owned) else {
            tracing::info!("Not restoring {}: its name is not UTF-8", root.display());
            continue;
        };
        if names.contains(shortpath.as_str()) {
            continue;
        }
        if is_link {
            match std::fs::metadata(&root) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => continue,
                Err(error) => {
                    tracing::info!(
                        "Not restoring {}: it is a link to a folder that cannot be opened: {error}",
                        root.display()
                    );
                    continue;
                }
            }
        }
        let bytes = match std::fs::read(record_path(&root)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                tracing::warn!(
                    "Not restoring {}: cannot read its {RECORD_FILE_NAME}: {error}",
                    root.display()
                );
                continue;
            }
        };
        let record = match parse_record(&bytes) {
            ParsedRecord::Valid(record) => record,
            other => {
                tracing::warn!(
                    "Not restoring {}: {other} ({RECORD_FILE_NAME})",
                    root.display()
                );
                continue;
            }
        };
        let listed = listed.get_or_insert_with(|| {
            shortpaths
                .iter()
                .filter_map(|shortpath| {
                    same_file::Handle::from_path(servers_dir.join(shortpath)).ok()
                })
                .collect()
        });
        let handle = match same_file::Handle::from_path(&root) {
            Ok(handle) => handle,
            Err(error) => {
                tracing::warn!("Not restoring {}: {error}", root.display());
                continue;
            }
        };
        if listed.contains(&handle) {
            if is_link {
                tracing::info!(
                    "Not restoring {}: it is a link to the folder of a server in the library",
                    root.display()
                );
            }
            continue;
        }
        if let Some((_, other)) = taken.iter().find(|(taken, _)| *taken == handle) {
            tracing::info!(
                "Not restoring {}: it leads to the same folder as servers/{other}, which is restored",
                root.display()
            );
            continue;
        }
        taken.push((handle, shortpath.clone()));
        let has_icon = root.join("icon.png").is_file();
        unlisted.push(UnlistedServer {
            shortpath,
            record,
            has_icon,
        });
    }
    unlisted.sort_by(|a, b| {
        (a.record.date_created, &a.shortpath).cmp(&(b.record.date_created, &b.shortpath))
    });
    unlisted
}

/// Names each server in `unlisted` that has the name and creation time of a
/// server in `rows`, or of one listed before it, after its folder as well:
/// "Survival (Survival_1700000000000 - Copy)". Such a folder is a copy of
/// another (a backup made by copying it, say), and without this the library
/// would show two servers nobody can tell apart, one of which a delete
/// wipes. Servers that only share a name were created at different times
/// and keep it. `unlisted` is in the order `find_unlisted_servers` gives,
/// so of two lost folders the original, whose name the copy's extends, keeps
/// the plain name.
pub fn name_copies_after_their_folders(unlisted: &mut [UnlistedServer], rows: &[ServerRow]) {
    let mut seen: HashSet<(String, i64)> = rows
        .iter()
        .map(|row| (row.name.clone(), row.date_created.timestamp_millis()))
        .collect();
    for server in unlisted {
        let key = (
            server.record.name.clone(),
            server.record.date_created.timestamp_millis(),
        );
        if !seen.insert(key) {
            server.record.name = format!("{} ({})", server.record.name, server.shortpath);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    fn row() -> ServerRow {
        ServerRow {
            id: 7,
            name: "Survival with friends".into(),
            shortpath: "Survival with friends_1700000000000".into(),
            favorite: true,
            index: 3,
            library_position: Some(4),
            group_id: 2,
            server_type: "modded".into(),
            game_version: "1.20.1".into(),
            port: 25570,
            motd: "Welcome \"home\"".into(),
            max_players: 8,
            online_mode: false,
            xmx: 6144,
            xms: 2048,
            extra_java_args: "-XX:+UseG1GC".into(),
            auto_restart: true,
            date_created: FixedOffset::east_opt(2 * 3600)
                .unwrap()
                .timestamp_millis_opt(1_700_000_123_456)
                .unwrap(),
            last_started: Some(Utc.timestamp_millis_opt(1_700_100_000_000).unwrap().into()),
            provider_type: "local".into(),
            hosted_server_id: Some("hosted-1".into()),
            icon_revision: Some(5),
            modloader_type: Some("fabric".into()),
            modloader_version: Some("0.15.11".into()),
            modpack_platform: Some("modrinth".into()),
            modpack_project_id: Some("1KVo5zza".into()),
            modpack_file_id: Some("abcdEFGH".into()),
        }
    }

    fn valid(bytes: &[u8]) -> ServerRecord {
        match parse_record(bytes) {
            ParsedRecord::Valid(record) => record,
            other => panic!("expected a valid record, got: {other}"),
        }
    }

    #[test]
    fn the_record_round_trips_every_stored_setting() {
        let row = row();
        let record = ServerRecord::from_row(&row, Some("Friends".into()));
        let parsed = valid(&record.to_json().unwrap());
        assert_eq!(parsed, record);

        let restored = parsed.into_restored(row.shortpath.clone(), Some(1));
        assert_eq!(
            restored,
            RestoredServer {
                name: row.name.clone(),
                shortpath: row.shortpath.clone(),
                favorite: row.favorite,
                server_type: row.server_type.clone(),
                game_version: row.game_version.clone(),
                port: row.port,
                motd: row.motd.clone(),
                max_players: row.max_players,
                online_mode: row.online_mode,
                xmx: row.xmx,
                xms: row.xms,
                extra_java_args: row.extra_java_args.clone(),
                auto_restart: row.auto_restart,
                date_created: DbDateTime(row.date_created),
                provider_type: row.provider_type.clone(),
                hosted_server_id: row.hosted_server_id.clone(),
                icon_revision: Some(1),
                modloader_type: row.modloader_type.clone(),
                modloader_version: row.modloader_version.clone(),
                modpack_platform: row.modpack_platform.clone(),
                modpack_project_id: row.modpack_project_id.clone(),
                modpack_file_id: row.modpack_file_id.clone(),
                group: Some("Friends".into()),
            }
        );
        assert_eq!(
            restored.date_created.0.timestamp_millis(),
            row.date_created.timestamp_millis()
        );
    }

    #[test]
    fn a_server_in_the_default_group_and_without_a_pack_round_trips() {
        let mut row = row();
        row.modloader_type = None;
        row.modloader_version = None;
        row.modpack_platform = None;
        row.modpack_project_id = None;
        row.modpack_file_id = None;
        row.hosted_server_id = None;
        let record = ServerRecord::from_row(&row, None);
        assert_eq!(valid(&record.to_json().unwrap()), record);
    }

    #[test]
    fn fields_a_later_build_adds_under_the_same_version_are_ignored() {
        let record = ServerRecord::from_row(&row(), None);
        let mut value = serde_json::to_value(&record).unwrap();
        value["somethingNew"] = serde_json::json!({ "nested": true });
        assert_eq!(valid(&serde_json::to_vec(&value).unwrap()), record);
    }

    #[test]
    fn empty_malformed_newer_and_incomplete_records_are_refused() {
        assert!(matches!(parse_record(b""), ParsedRecord::Empty));
        assert!(matches!(parse_record(b" \n\t"), ParsedRecord::Empty));
        assert!(matches!(
            parse_record(b"{\"version\": 1, \"name\""),
            ParsedRecord::Malformed(_)
        ));
        assert!(matches!(parse_record(b"[]"), ParsedRecord::Malformed(_)));

        let record = ServerRecord::from_row(&row(), None);
        let mut value = serde_json::to_value(&record).unwrap();
        value["version"] = serde_json::json!(RECORD_VERSION + 1);
        assert!(matches!(
            parse_record(&serde_json::to_vec(&value).unwrap()),
            ParsedRecord::Newer(v) if v == u64::from(RECORD_VERSION) + 1
        ));
        for version in [
            serde_json::json!(0),
            serde_json::json!("1"),
            serde_json::Value::Null,
        ] {
            value["version"] = version;
            assert!(matches!(
                parse_record(&serde_json::to_vec(&value).unwrap()),
                ParsedRecord::Malformed(_)
            ));
        }
        value.as_object_mut().unwrap().remove("version");
        assert!(matches!(
            parse_record(&serde_json::to_vec(&value).unwrap()),
            ParsedRecord::Malformed(_)
        ));

        let mut value = serde_json::to_value(&record).unwrap();
        value.as_object_mut().unwrap().remove("port");
        assert!(matches!(
            parse_record(&serde_json::to_vec(&value).unwrap()),
            ParsedRecord::Malformed(_)
        ));
        let mut value = serde_json::to_value(&record).unwrap();
        value["name"] = serde_json::json!("  ");
        assert!(matches!(
            parse_record(&serde_json::to_vec(&value).unwrap()),
            ParsedRecord::Incomplete(_)
        ));
        let mut value = serde_json::to_value(&record).unwrap();
        value["gameVersion"] = serde_json::json!("");
        assert!(matches!(
            parse_record(&serde_json::to_vec(&value).unwrap()),
            ParsedRecord::Incomplete(_)
        ));
    }

    fn write(dir: &Path, name: &str, contents: Option<&[u8]>) {
        let root = dir.join(name);
        std::fs::create_dir_all(root.join("server")).unwrap();
        if let Some(contents) = contents {
            std::fs::write(record_path(&root), contents).unwrap();
        }
    }

    #[test]
    fn only_folders_with_a_valid_record_and_no_row_are_listed() {
        let dir = tempfile::tempdir().unwrap();
        let servers = dir.path();
        let record = ServerRecord::from_row(&row(), None);
        let json = record.to_json().unwrap();
        let mut newer = serde_json::to_value(&record).unwrap();
        newer["version"] = serde_json::json!(RECORD_VERSION + 1);
        let newer = serde_json::to_vec(&newer).unwrap();

        write(servers, "listed", Some(&json));
        write(servers, "lost", Some(&json));
        write(servers, "no-record", None);
        write(servers, "empty", Some(b""));
        write(servers, "malformed", Some(b"{not json"));
        write(servers, "newer", Some(&newer));
        std::fs::write(servers.join("stray-file.json"), &json).unwrap();
        std::fs::write(servers.join("lost").join("icon.png"), b"png").unwrap();

        let unlisted = find_unlisted_servers(servers, &["listed".into()]);
        assert_eq!(
            unlisted
                .iter()
                .map(|server| server.shortpath.as_str())
                .collect::<Vec<_>>(),
            ["lost"]
        );
        assert_eq!(unlisted[0].record, record);
        assert!(unlisted[0].has_icon);

        assert!(find_unlisted_servers(&servers.join("missing"), &[]).is_empty());
    }

    /// A row whose shortpath reaches the folder under another spelling (a
    /// file system that ignores case) keeps that folder from being listed as
    /// a second server.
    #[test]
    fn a_folder_its_row_reaches_under_another_spelling_is_not_listed() {
        let dir = tempfile::tempdir().unwrap();
        let servers = dir.path();
        let json = ServerRecord::from_row(&row(), None).to_json().unwrap();
        write(servers, "Survival_1", Some(&json));
        let case_insensitive = servers.join("SURVIVAL_1").is_dir();

        let unlisted = find_unlisted_servers(servers, &["SURVIVAL_1".into()]);
        if case_insensitive {
            assert!(unlisted.is_empty(), "{unlisted:?}");
        } else {
            assert_eq!(unlisted.len(), 1);
        }
    }

    /// A server kept on another drive, its folder under `servers/` replaced
    /// by a link, comes back like any other.
    #[cfg(unix)]
    #[test]
    fn a_link_to_a_server_folder_elsewhere_is_listed() {
        let dir = tempfile::tempdir().unwrap();
        let servers = dir.path().join("servers");
        let record = ServerRecord::from_row(&row(), None);
        write(dir.path(), "elsewhere", Some(&record.to_json().unwrap()));
        std::fs::create_dir_all(&servers).unwrap();
        std::os::unix::fs::symlink(dir.path().join("elsewhere"), servers.join("Big_1")).unwrap();

        let unlisted = find_unlisted_servers(&servers, &[]);
        assert_eq!(unlisted.len(), 1, "{unlisted:?}");
        assert_eq!(unlisted[0].shortpath, "Big_1");
        assert_eq!(unlisted[0].record, record);
    }

    /// A link never lists a folder a second time: not one a row already
    /// reaches, not one another entry lists, and not one it leads to under
    /// its real name. A link that leads nowhere is skipped.
    #[cfg(unix)]
    #[test]
    fn a_folder_two_entries_lead_to_is_listed_once() {
        let dir = tempfile::tempdir().unwrap();
        let servers = dir.path().join("servers");
        let json = ServerRecord::from_row(&row(), None).to_json().unwrap();
        write(&servers, "listed_1", Some(&json));
        write(&servers, "lost_2", Some(&json));
        write(dir.path(), "elsewhere", Some(&json));
        let link = |target: &Path, name: &str| {
            std::os::unix::fs::symlink(target, servers.join(name)).unwrap();
        };
        link(&servers.join("listed_1"), "a_link_to_listed");
        // Sorts before the folder it leads to, and still loses to it.
        link(&servers.join("lost_2"), "a_link_to_lost");
        link(&dir.path().join("elsewhere"), "b_first_link");
        link(&dir.path().join("elsewhere"), "c_second_link");
        link(&dir.path().join("gone"), "d_dangling");

        let unlisted = find_unlisted_servers(&servers, &["listed_1".into()]);
        assert_eq!(
            unlisted
                .iter()
                .map(|server| server.shortpath.as_str())
                .collect::<Vec<_>>(),
            ["b_first_link", "lost_2"]
        );
    }

    /// A row whose folder is a link keeps the folder it leads to, under
    /// `servers/`, from being listed as a second server.
    #[cfg(unix)]
    #[test]
    fn a_folder_a_listed_link_leads_to_is_not_listed() {
        let dir = tempfile::tempdir().unwrap();
        let servers = dir.path().join("servers");
        let json = ServerRecord::from_row(&row(), None).to_json().unwrap();
        write(&servers, "real_1", Some(&json));
        std::os::unix::fs::symlink(servers.join("real_1"), servers.join("listed_2")).unwrap();

        assert!(find_unlisted_servers(&servers, &["listed_2".into()]).is_empty());
    }

    fn unlisted(shortpath: &str, record: &ServerRecord) -> UnlistedServer {
        UnlistedServer {
            shortpath: shortpath.into(),
            record: record.clone(),
            has_icon: false,
        }
    }

    /// A copy of a server, beside its original in the library or lost with
    /// it, is told apart by its folder name. Servers that only share a name
    /// keep it.
    #[test]
    fn copies_are_named_after_their_folders() {
        let original = row();
        let record = ServerRecord::from_row(&original, None);
        let mut other_server = record.clone();
        other_server.date_created += chrono::Duration::milliseconds(1);

        let mut beside = [
            unlisted("Survival_1 - Copy", &record),
            unlisted("Survival_2", &other_server),
        ];
        name_copies_after_their_folders(&mut beside, std::slice::from_ref(&original));
        assert_eq!(
            beside
                .iter()
                .map(|s| s.record.name.as_str())
                .collect::<Vec<_>>(),
            [
                "Survival with friends (Survival_1 - Copy)",
                "Survival with friends"
            ]
        );
        // Only the name changes.
        assert_eq!(
            ServerRecord {
                name: record.name.clone(),
                ..beside[0].record.clone()
            },
            record
        );

        let mut both_lost = [
            unlisted("Survival_1", &record),
            unlisted("Survival_1 - Copy", &record),
        ];
        name_copies_after_their_folders(&mut both_lost, &[]);
        assert_eq!(
            both_lost
                .iter()
                .map(|s| s.record.name.as_str())
                .collect::<Vec<_>>(),
            [
                "Survival with friends",
                "Survival with friends (Survival_1 - Copy)"
            ]
        );
    }
}
