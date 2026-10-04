use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{trace, warn};

use super::{
    GET_IMPORT_SCAN_STATUS, ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter,
    InternalImportEntry, InvalidImportEntry,
    common::{
        DeclaredModLoader, ImportedSettings, apply_imported_settings, copy_game_dir,
        datetime_from_epoch_secs_or_millis, dedup_paths, dir_name, first_existing_dir,
        group_for_import, json_i64, json_string, parse_loader_type, positive_u32, read_json_value,
        resolve_standard_version, seconds_played_from_i64, trimmed_non_empty,
    },
};
use crate::{
    api::translation::Translation,
    domain::{
        instance::info::{GameVersion, ModLoaderType, Modpack, ModrinthModpack},
        vtask::VisualTaskId,
    },
    managers::{AppInner, instance::InstanceVersionSource},
};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    name: String,
    icon_path: Option<PathBuf>,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
    modpack: Option<Modpack>,
    seconds_played: Option<u32>,
    last_played: Option<DateTime<Utc>>,
    settings: ImportedSettings,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.name,
        }
    }
}

#[derive(Debug)]
pub struct ModrinthAppImporter {
    state: RwLock<ImporterState<Importable>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ProfileRecord {
    name: Option<String>,
    icon_path: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
    linked_project_id: Option<String>,
    linked_version_id: Option<String>,
    last_played: Option<DateTime<Utc>>,
    seconds_played: Option<u32>,
    settings: ImportedSettings,
}

impl ModrinthAppImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path() -> anyhow::Result<Option<PathBuf>> {
        let Some(settings_dir) = first_existing_dir(settings_dir_candidates()) else {
            return Ok(None);
        };

        Ok(Some(profiles_dir_for_settings_dir(&settings_dir).await))
    }

    async fn scan_profile(
        &self,
        path: PathBuf,
        record: Option<ProfileRecord>,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        if !path.is_dir() {
            return Ok(None);
        }

        let filename = dir_name(&path);

        let record = match record {
            Some(record) => Some(record),
            None => match read_json_value(&path.join("profile.json")).await {
                Some(json) => Some(record_from_profile_json(&json)),
                None => None,
            },
        };

        let Some(record) = record else {
            trace!("No metadata for Modrinth profile at {path:?}, skipping");
            return Ok(None);
        };

        let Some(minecraft_version) = record
            .game_version
            .clone()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        else {
            warn!("Modrinth profile at {path:?} has no game version");
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        let name = record
            .name
            .clone()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| filename.clone());

        let loaders = loaders_from_record(&record);

        let modpack = match (&record.linked_project_id, &record.linked_version_id) {
            (Some(project_id), Some(version_id))
                if !project_id.trim().is_empty() && !version_id.trim().is_empty() =>
            {
                Some(Modpack::Modrinth(ModrinthModpack {
                    project_id: project_id.trim().to_string(),
                    version_id: version_id.trim().to_string(),
                }))
            }
            _ => None,
        };

        let icon_path = record
            .icon_path
            .as_deref()
            .and_then(|icon| resolve_icon_path(&path, icon));

        let mut settings = record.settings;
        settings.skip_missing_java_path();

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename,
            path,
            name,
            icon_path,
            minecraft_version,
            loaders,
            modpack,
            seconds_played: record.seconds_played,
            last_played: record.last_played,
            settings,
        })))
    }
}

#[async_trait::async_trait]
impl InstanceImporter for ModrinthAppImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        let scan_path = if scan_path.is_file() {
            match scan_path.parent() {
                Some(parent) => parent.to_path_buf(),
                None => return Ok(()),
            }
        } else {
            scan_path
        };

        if !scan_path.is_dir() {
            return Ok(());
        }

        let layout = Layout::detect(&scan_path);
        let records = load_records(&layout).await;

        if let Some(profile) = &layout.single_profile {
            let record = records.get(&dir_name(profile)).cloned();
            if let Some(entry) = self.scan_profile(profile.clone(), record).await? {
                self.state.write().await.set_single(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            }
            return Ok(());
        }

        let mut dir = match super::common::scan_directory(&layout.profiles_dir).await {
            Ok(dir) => dir,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(metadata_error.unwrap_or_else(|| e.into()));
            }
            Err(e) => return Err(e.into()),
        };

        while let Some(entry) = dir.next_entry().await? {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                continue;
            }

            let record = records.get(&dir_name(&entry_path)).cloned();
            if let Some(entry) = self.scan_profile(entry_path, record).await? {
                self.state.write().await.push_multi(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            }
        }

        Ok(())
    }

    async fn get_status(&self) -> ImportScanStatus {
        self.state.read().await.clone().into()
    }

    async fn begin_import(
        &self,
        app: &Arc<AppInner>,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        trace!(?index, ?name, "Beginning Modrinth App import");

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index {index}"))?;

        let icon = match &instance.icon_path {
            Some(icon_path) => app
                .instance_manager()
                .load_icon(icon_path.clone())
                .await
                .ok(),
            None => None,
        };

        let standard_version =
            resolve_standard_version(app, &instance.minecraft_version, &instance.loaders).await;
        let version = GameVersion::Standard(standard_version);

        let instance_version_source = match &instance.modpack {
            Some(modpack) => {
                InstanceVersionSource::ModpackWithKnownVersion(version, modpack.clone(), true)
            }
            None => InstanceVersionSource::Version(version),
        };

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                copy_game_dir(&instance.path, &instance_path, |path| {
                    !is_launcher_file(path)
                })
                .await?;
                Ok(())
            }
        };

        let group = group_for_import(app, instance.settings.group.as_deref()).await?;

        let id = app
            .instance_manager()
            .create_instance_ext(
                group,
                name.unwrap_or_else(|| instance.name.clone()),
                icon,
                instance.seconds_played,
                instance.last_played,
                instance_version_source,
                instance.settings.notes.clone().unwrap_or_default(),
                initializer,
            )
            .await?;

        apply_imported_settings(app, id, &instance.settings).await;

        app.instance_manager()
            .prepare_game(id, None, None, true)
            .await
            .map(|r| r.1)
    }
}

fn is_launcher_file(relative: &Path) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();
    first == "profile.json" || first == "natives"
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Layout {
    profiles_dir: PathBuf,
    settings_dir: Option<PathBuf>,
    single_profile: Option<PathBuf>,
}

impl Layout {
    fn detect(path: &Path) -> Self {
        let parent = path.parent();
        let parent_is_profiles = parent.is_some_and(|p| dir_name(p) == "profiles");
        if path.join("profile.json").is_file()
            || (parent_is_profiles && !path.join("profiles").is_dir())
        {
            let profiles_dir = parent
                .map(Path::to_path_buf)
                .unwrap_or_else(|| path.to_path_buf());
            return Self {
                settings_dir: profiles_dir.parent().map(Path::to_path_buf),
                profiles_dir,
                single_profile: Some(path.to_path_buf()),
            };
        }

        if path.join("profiles").is_dir() || path.join("app.db").is_file() {
            return Self {
                profiles_dir: path.join("profiles"),
                settings_dir: Some(path.to_path_buf()),
                single_profile: None,
            };
        }

        Self {
            profiles_dir: path.to_path_buf(),
            settings_dir: parent
                .filter(|p| p.join("app.db").is_file() || dir_name(path) == "profiles")
                .map(Path::to_path_buf),
            single_profile: None,
        }
    }
}

/// A custom profiles location leaves `app.db` behind in the default data directory.
async fn load_records(layout: &Layout) -> HashMap<String, ProfileRecord> {
    let mut db_candidates = Vec::new();
    if let Some(settings_dir) = &layout.settings_dir {
        db_candidates.push(settings_dir.join("app.db"));
    }
    for dir in settings_dir_candidates() {
        db_candidates.push(dir.join("app.db"));
    }

    for db_path in db_candidates {
        if !db_path.is_file() {
            continue;
        }

        let db_path_clone = db_path.clone();
        let result =
            tokio::task::spawn_blocking(move || read_database_records(&db_path_clone)).await;

        match result {
            Ok(Ok(records)) if !records.is_empty() => return records,
            Ok(Ok(_)) => trace!("Modrinth database {db_path:?} lists no profiles"),
            Ok(Err(e)) => warn!("Cannot read Modrinth database {db_path:?}: {e}"),
            Err(e) => warn!("Modrinth database reader panicked: {e}"),
        }
    }

    HashMap::new()
}

/// Reads a copy (with WAL/SHM) so a running app or a read-only location doesn't get in the way.
fn read_database_records(db_path: &Path) -> anyhow::Result<HashMap<String, ProfileRecord>> {
    let tmp = tempfile::tempdir()?;
    let copy = tmp.path().join("app.db");
    std::fs::copy(db_path, &copy)?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", db_path.to_string_lossy()));
        if sidecar.is_file() {
            let _ = std::fs::copy(&sidecar, tmp.path().join(format!("app.db{suffix}")));
        }
    }

    let conn = rusqlite::Connection::open(&copy)?;
    let records = read_records_from_connection(&conn)?;
    drop(conn);
    Ok(records)
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
    .unwrap_or(false)
}

fn read_records_from_connection(
    conn: &rusqlite::Connection,
) -> anyhow::Result<HashMap<String, ProfileRecord>> {
    let mut records = HashMap::new();

    if table_exists(conn, "instances") {
        match read_instances_table(conn) {
            Ok(found) => records.extend(found),
            Err(e) => warn!("Cannot read Modrinth `instances` table: {e}"),
        }
    }

    if table_exists(conn, "profiles") {
        match read_profiles_table(conn) {
            Ok(found) => {
                for (path, record) in found {
                    records.entry(path).or_insert(record);
                }
            }
            Err(e) => warn!("Cannot read Modrinth `profiles` table: {e}"),
        }
    }

    Ok(records)
}

fn read_instances_table(
    conn: &rusqlite::Connection,
) -> anyhow::Result<HashMap<String, ProfileRecord>> {
    let has_links = table_exists(conn, "instance_links");
    let has_content_sets = table_exists(conn, "instance_content_sets");

    let mut overrides: HashMap<String, ImportedSettings> = HashMap::new();
    if table_exists(conn, "instance_launch_overrides") {
        match read_launch_overrides(conn) {
            Ok(found) => overrides = found,
            Err(e) => warn!("Cannot read Modrinth launch overrides: {e}"),
        }
    }
    let mut groups: HashMap<String, String> = HashMap::new();
    if table_exists(conn, "instance_groups") {
        match read_instance_groups(conn) {
            Ok(found) => groups = found,
            Err(e) => warn!("Cannot read Modrinth instance groups: {e}"),
        }
    }

    let mut sql = String::from(
        "SELECT i.path, i.name, i.icon_path, i.last_played, i.submitted_time_played, i.recent_time_played",
    );
    if has_content_sets {
        sql.push_str(", cs.game_version, cs.loader, cs.loader_version");
    } else {
        sql.push_str(", NULL, NULL, NULL");
    }
    if has_links {
        sql.push_str(", l.modrinth_project_id, l.modrinth_version_id");
    } else {
        sql.push_str(", NULL, NULL");
    }
    sql.push_str(", i.id FROM instances i");
    if has_content_sets {
        sql.push_str(
            " LEFT JOIN instance_content_sets cs ON cs.id = COALESCE(i.applied_content_set_id, (SELECT id FROM instance_content_sets WHERE instance_id = i.id ORDER BY created LIMIT 1))",
        );
    }
    if has_links {
        sql.push_str(" LEFT JOIN instance_links l ON l.instance_id = i.id");
    }

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            ProfileRecord {
                name: row.get(1)?,
                icon_path: row.get(2)?,
                last_played: row
                    .get::<_, Option<i64>>(3)?
                    .and_then(datetime_from_epoch_secs_or_millis),
                seconds_played: seconds_played_from_i64(
                    row.get::<_, Option<i64>>(4)?.unwrap_or(0)
                        + row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                ),
                game_version: row.get(6)?,
                loader: row.get(7)?,
                loader_version: row.get(8)?,
                linked_project_id: row.get(9)?,
                linked_version_id: row.get(10)?,
                settings: ImportedSettings::default(),
            },
            row.get::<_, Option<String>>(11)?,
        ))
    })?;

    let mut records = HashMap::new();
    for row in rows {
        let (path, mut record, id) = row?;
        if let Some(id) = id {
            if let Some(settings) = overrides.remove(&id) {
                record.settings = settings;
            }
            record.settings.group = groups.remove(&id);
        }
        if let Some(path) = path {
            records.insert(path, record);
        }
    }
    Ok(records)
}

/// `{"java_path", "extra_launch_args": [..], "memory": {"maximum"}, "game_resolution": [w, h], "hooks": {"pre_launch", "wrapper", "post_exit"}}`
fn read_launch_overrides(
    conn: &rusqlite::Connection,
) -> anyhow::Result<HashMap<String, ImportedSettings>> {
    let mut stmt =
        conn.prepare("SELECT instance_id, json(overrides) FROM instance_launch_overrides")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;

    let mut overrides = HashMap::new();
    for row in rows {
        let (id, json) = row?;
        let Some(json) = json.and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
        else {
            continue;
        };
        overrides.insert(id, settings_from_overrides_json(&json));
    }
    Ok(overrides)
}

fn read_instance_groups(conn: &rusqlite::Connection) -> anyhow::Result<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT instance_id, group_name FROM instance_groups")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut groups = HashMap::new();
    for row in rows {
        let (id, name) = row?;
        let name = name.trim().to_string();
        if !name.is_empty() {
            // An instance can be in several groups; the first is as good as any.
            groups.entry(id).or_insert(name);
        }
    }
    Ok(groups)
}

fn settings_from_overrides_json(json: &serde_json::Value) -> ImportedSettings {
    let text = |value: Option<&serde_json::Value>| {
        trimmed_non_empty(value.and_then(|v| v.as_str()).map(str::to_string))
    };
    let positive = |value: Option<&serde_json::Value>| {
        positive_u32(value.and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))))
    };

    let hooks = json.get("hooks").filter(|h| h.is_object());

    let mut settings = ImportedSettings {
        java_path: text(json.get("java_path"))
            .or_else(|| text(json.pointer("/java/override_version/path"))),
        memory_max_mb: positive(json.pointer("/memory/maximum")),
        game_resolution: json
            .get("game_resolution")
            .or_else(|| json.get("resolution"))
            .and_then(|r| r.as_array())
            .and_then(|r| Some((positive(r.first())?, positive(r.get(1))?))),
        pre_launch_hook: text(hooks.and_then(|h| h.get("pre_launch"))),
        wrapper_command: text(hooks.and_then(|h| h.get("wrapper"))),
        post_exit_hook: text(hooks.and_then(|h| h.get("post_exit"))),
        ..Default::default()
    };

    let args: Vec<String> = json
        .get("extra_launch_args")
        .or_else(|| json.pointer("/java/extra_arguments"))
        .and_then(|a| a.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if !args.is_empty() {
        settings.absorb_jvm_args(&args.join(" "));
    }

    settings
}

fn read_profiles_table(
    conn: &rusqlite::Connection,
) -> anyhow::Result<HashMap<String, ProfileRecord>> {
    let mut stmt = conn.prepare(
        "SELECT path, name, icon_path, game_version, mod_loader, mod_loader_version, linked_project_id, linked_version_id, last_played, submitted_time_played, recent_time_played, override_java_path, override_extra_launch_args, override_mc_memory_max, override_mc_game_resolution_x, override_mc_game_resolution_y, override_hook_pre_launch, override_hook_wrapper, override_hook_post_exit, groups FROM profiles",
    )?;
    let rows = stmt.query_map([], |row| {
        let mut settings = ImportedSettings {
            java_path: trimmed_non_empty(row.get(11)?),
            memory_max_mb: positive_u32(row.get(13)?),
            game_resolution: match (positive_u32(row.get(14)?), positive_u32(row.get(15)?)) {
                (Some(w), Some(h)) => Some((w, h)),
                _ => None,
            },
            pre_launch_hook: trimmed_non_empty(row.get(16)?),
            wrapper_command: trimmed_non_empty(row.get(17)?),
            post_exit_hook: trimmed_non_empty(row.get(18)?),
            group: row
                .get::<_, Option<String>>(19)?
                .and_then(|g| serde_json::from_str::<Vec<String>>(&g).ok())
                .and_then(|g| {
                    g.into_iter()
                        .map(|n| n.trim().to_string())
                        .find(|n| !n.is_empty())
                }),
            ..Default::default()
        };
        if let Some(args) = row
            .get::<_, Option<String>>(12)?
            .and_then(|a| serde_json::from_str::<Vec<String>>(&a).ok())
            .filter(|a| !a.is_empty())
        {
            settings.absorb_jvm_args(&args.join(" "));
        }

        Ok((
            row.get::<_, Option<String>>(0)?,
            ProfileRecord {
                name: row.get(1)?,
                icon_path: row.get(2)?,
                game_version: row.get(3)?,
                loader: row.get(4)?,
                loader_version: row.get(5)?,
                linked_project_id: row.get(6)?,
                linked_version_id: row.get(7)?,
                last_played: row
                    .get::<_, Option<i64>>(8)?
                    .and_then(datetime_from_epoch_secs_or_millis),
                seconds_played: seconds_played_from_i64(
                    row.get::<_, Option<i64>>(9)?.unwrap_or(0)
                        + row.get::<_, Option<i64>>(10)?.unwrap_or(0),
                ),
                settings,
            },
        ))
    })?;

    let mut records = HashMap::new();
    for row in rows {
        let (path, record) = row?;
        if let Some(path) = path {
            records.insert(path, record);
        }
    }
    Ok(records)
}

fn record_from_profile_json(json: &serde_json::Value) -> ProfileRecord {
    let metadata = json
        .get("metadata")
        .filter(|m| m.is_object())
        .unwrap_or(json);

    let loader_version = metadata.get("loader_version").and_then(|lv| match lv {
        serde_json::Value::Object(_) => json_string(lv, "id"),
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    });

    let linked = metadata.get("linked_data").filter(|l| l.is_object());

    let last_played = metadata.get("last_played").and_then(|lp| match lp {
        serde_json::Value::String(s) => DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.with_timezone(&Utc)),
        serde_json::Value::Number(n) => n.as_i64().and_then(datetime_from_epoch_secs_or_millis),
        _ => None,
    });

    let mut settings = settings_from_overrides_json(json);
    settings.group = metadata
        .get("groups")
        .and_then(|g| g.as_array())
        .and_then(|g| {
            g.iter()
                .filter_map(|n| n.as_str())
                .map(str::trim)
                .find(|n| !n.is_empty())
                .map(str::to_string)
        });

    ProfileRecord {
        name: json_string(metadata, "name"),
        icon_path: json_string(metadata, "icon").or_else(|| json_string(metadata, "icon_path")),
        game_version: json_string(metadata, "game_version"),
        loader: json_string(metadata, "loader"),
        loader_version,
        linked_project_id: linked.and_then(|l| json_string(l, "project_id")),
        linked_version_id: linked.and_then(|l| json_string(l, "version_id")),
        last_played,
        seconds_played: seconds_played_from_i64(
            json_i64(metadata, "submitted_time_played").unwrap_or(0)
                + json_i64(metadata, "recent_time_played").unwrap_or(0),
        ),
        settings,
    }
}

fn loaders_from_record(record: &ProfileRecord) -> Vec<DeclaredModLoader> {
    let Some(loader) = record.loader.as_deref().map(str::trim) else {
        return Vec::new();
    };
    if loader.is_empty() || loader.eq_ignore_ascii_case("vanilla") {
        return Vec::new();
    }
    let Some(type_) = parse_loader_type(loader) else {
        warn!("Unknown Modrinth loader `{loader}`, importing as vanilla");
        return Vec::new();
    };
    let Some(version) = record
        .loader_version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        warn!("Modrinth profile declares {loader} without a version, importing as vanilla");
        return Vec::new();
    };

    let version = if type_ == ModLoaderType::Forge {
        super::common::normalize_forge_version(
            record.game_version.as_deref().unwrap_or(""),
            version,
        )
    } else {
        version.to_string()
    };

    vec![DeclaredModLoader::new(type_, version)]
}

/// Absolute paths into `icons/`, or relative ones from older versions.
fn resolve_icon_path(profile_dir: &Path, icon: &str) -> Option<PathBuf> {
    let icon = icon.trim();
    if icon.is_empty() {
        return None;
    }

    let raw = PathBuf::from(icon);
    if raw.is_absolute() {
        return raw.is_file().then_some(raw);
    }

    let mut candidates = vec![profile_dir.join(&raw)];
    if let Some(profiles_dir) = profile_dir.parent() {
        candidates.push(profiles_dir.join(&raw));
        if let Some(settings_dir) = profiles_dir.parent() {
            candidates.push(settings_dir.join("icons").join(&raw));
            candidates.push(settings_dir.join(&raw));
        }
    }

    candidates.into_iter().find(|p| p.is_file())
}

async fn profiles_dir_for_settings_dir(settings_dir: &Path) -> PathBuf {
    let db_path = settings_dir.join("app.db");
    if db_path.is_file() {
        let custom = tokio::task::spawn_blocking(move || read_custom_dir(&db_path))
            .await
            .ok()
            .flatten();
        if let Some(custom) = custom {
            let profiles = PathBuf::from(custom).join("profiles");
            if profiles.is_dir() {
                return profiles;
            }
        }
    }

    if let Some(json) = read_json_value(&settings_dir.join("settings.json")).await {
        if let Some(custom) = json_string(&json, "loaded_config_dir") {
            let profiles = PathBuf::from(custom).join("profiles");
            if profiles.is_dir() {
                return profiles;
            }
        }
    }

    settings_dir.join("profiles")
}

fn read_custom_dir(db_path: &Path) -> Option<String> {
    let tmp = tempfile::tempdir().ok()?;
    let copy = tmp.path().join("app.db");
    std::fs::copy(db_path, &copy).ok()?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", db_path.to_string_lossy()));
        if sidecar.is_file() {
            let _ = std::fs::copy(&sidecar, tmp.path().join(format!("app.db{suffix}")));
        }
    }

    let conn = rusqlite::Connection::open(&copy).ok()?;
    if !table_exists(&conn, "settings") {
        return None;
    }
    conn.query_row("SELECT custom_dir FROM settings LIMIT 1", [], |row| {
        row.get::<_, Option<String>>(0)
    })
    .ok()
    .flatten()
    .filter(|dir| !dir.trim().is_empty())
}

fn settings_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(dir) = std::env::var_os("THESEUS_CONFIG_DIR") {
        candidates.push(PathBuf::from(dir));
    }

    let Some(basedirs) = directories::BaseDirs::new() else {
        return candidates;
    };

    candidates.push(basedirs.data_dir().join("ModrinthApp"));
    candidates.push(
        basedirs
            .home_dir()
            .join(".var/app/com.modrinth.ModrinthApp/data/ModrinthApp"),
    );
    candidates.push(basedirs.config_dir().join("ModrinthApp"));
    candidates.push(basedirs.config_dir().join("com.modrinth.theseus"));
    candidates.push(basedirs.data_dir().join("com.modrinth.theseus"));
    candidates.push(
        basedirs
            .home_dir()
            .join(".var/app/com.modrinth.theseus/config/com.modrinth.theseus"),
    );

    dedup_paths(candidates)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn modrinth_settings_dir_candidates_are_unique() {
        super::super::common::assert_paths_unique(&settings_dir_candidates());
    }

    fn new_schema_db(path: &Path) {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE settings (custom_dir TEXT NULL, prev_custom_dir TEXT NULL);
            INSERT INTO settings (custom_dir) VALUES (NULL);
            CREATE TABLE instances (
                id TEXT NOT NULL, path TEXT NOT NULL, applied_content_set_id TEXT NULL,
                install_stage TEXT NOT NULL, launcher_feature_version TEXT NOT NULL,
                update_channel TEXT NOT NULL DEFAULT 'release',
                name TEXT NOT NULL, icon_path TEXT NULL,
                created INTEGER NOT NULL, modified INTEGER NOT NULL, last_played INTEGER NULL,
                submitted_time_played INTEGER NOT NULL DEFAULT 0,
                recent_time_played INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (id)
            );
            CREATE TABLE instance_content_sets (
                id TEXT NOT NULL, instance_id TEXT NOT NULL, name TEXT NOT NULL,
                source_kind TEXT NOT NULL, status TEXT NOT NULL,
                game_version TEXT NOT NULL, protocol_version INTEGER NULL,
                loader TEXT NOT NULL, loader_version TEXT NULL,
                created INTEGER NOT NULL, modified INTEGER NOT NULL,
                PRIMARY KEY (id)
            );
            CREATE TABLE instance_links (
                instance_id TEXT NOT NULL, link_kind TEXT NOT NULL,
                modrinth_project_id TEXT NULL, modrinth_version_id TEXT NULL,
                PRIMARY KEY (instance_id)
            );
            INSERT INTO instances VALUES ('i1', 'Fabulously Optimized', 'cs1', 'installed', 'v1', 'release',
                'Fabulously Optimized', '/nonexistent/icons/fo.png', 1, 1, 1700000000, 100, 20);
            INSERT INTO instance_content_sets VALUES ('cs1', 'i1', 'Default', 'modrinth_modpack', 'available',
                '1.20.4', NULL, 'fabric', '0.15.6', 1, 1);
            INSERT INTO instance_links VALUES ('i1', 'modrinth_modpack', '1KVo5zza', 'abcd1234');
            CREATE TABLE instance_launch_overrides (
                instance_id TEXT NOT NULL, overrides JSONB NOT NULL, PRIMARY KEY (instance_id)
            );
            INSERT INTO instance_launch_overrides VALUES ('i1', jsonb('{"java_path":"/opt/java/bin/java","extra_launch_args":["-XX:+UseG1GC"],"custom_env_vars":null,"memory":{"maximum":6144},"force_fullscreen":null,"game_resolution":[1280,720],"hooks":{"pre_launch":"echo pre","wrapper":"","post_exit":""}}'));
            CREATE TABLE instance_groups (
                instance_id TEXT NOT NULL, group_name TEXT NOT NULL, PRIMARY KEY (instance_id, group_name)
            );
            INSERT INTO instance_groups VALUES ('i1', 'Performance');
            -- An instance whose applied set is missing: fall back to any set it has
            INSERT INTO instances VALUES ('i2', 'Vanilla', NULL, 'installed', 'v1', 'release',
                'Plain', NULL, 1, 1, NULL, 0, 0);
            INSERT INTO instance_content_sets VALUES ('cs2', 'i2', 'Default', 'local', 'available',
                '1.21', NULL, 'vanilla', NULL, 1, 1);
            "#,
        )
        .unwrap();
    }

    fn old_schema_db(path: &Path) {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE profiles (
                path TEXT NOT NULL, install_stage TEXT NOT NULL, name TEXT NOT NULL,
                icon_path TEXT NULL, game_version TEXT NOT NULL, mod_loader TEXT NOT NULL,
                mod_loader_version TEXT NULL, groups JSONB NOT NULL,
                linked_project_id TEXT NULL, linked_version_id TEXT NULL, locked INTEGER NULL,
                created INTEGER NOT NULL, modified INTEGER NOT NULL, last_played INTEGER NULL,
                submitted_time_played INTEGER NOT NULL DEFAULT 0,
                recent_time_played INTEGER NOT NULL DEFAULT 0,
                override_java_path TEXT NULL,
                override_extra_launch_args JSONB NOT NULL,
                override_custom_env_vars JSONB NOT NULL,
                override_mc_memory_max INTEGER NULL,
                override_mc_force_fullscreen INTEGER NULL,
                override_mc_game_resolution_x INTEGER NULL,
                override_mc_game_resolution_y INTEGER NULL,
                override_hook_pre_launch TEXT NULL,
                override_hook_wrapper TEXT NULL,
                override_hook_post_exit TEXT NULL
            );
            INSERT INTO profiles VALUES ('ATM9', 'installed', 'All the Mods 9', NULL, '1.20.1', 'forge',
                '1.20.1-47.2.0', '["Kitchen sink"]', NULL, NULL, NULL, 1, 1, 1700000000, 60, 0,
                '/usr/bin/java', '["-XX:+UseZGC"]', '[]', 8192, NULL, NULL, NULL, NULL, 'gamemoderun', NULL);
            "#,
        )
        .unwrap();
    }

    #[test]
    fn new_schema_database_is_read() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("app.db");
        new_schema_db(&db);

        let records = read_database_records(&db).unwrap();
        assert_eq!(records.len(), 2);

        let fo = &records["Fabulously Optimized"];
        assert_eq!(fo.name.as_deref(), Some("Fabulously Optimized"));
        assert_eq!(fo.game_version.as_deref(), Some("1.20.4"));
        assert_eq!(fo.loader.as_deref(), Some("fabric"));
        assert_eq!(fo.loader_version.as_deref(), Some("0.15.6"));
        assert_eq!(fo.linked_project_id.as_deref(), Some("1KVo5zza"));
        assert_eq!(fo.linked_version_id.as_deref(), Some("abcd1234"));
        assert_eq!(fo.last_played.map(|d| d.timestamp()), Some(1_700_000_000));
        assert_eq!(fo.seconds_played, Some(120));
        assert_eq!(
            loaders_from_record(fo),
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.6")]
        );
        assert_eq!(fo.settings.group.as_deref(), Some("Performance"));
        assert_eq!(fo.settings.memory(), Some((6144, 6144)));
        assert_eq!(fo.settings.java_path.as_deref(), Some("/opt/java/bin/java"));
        assert_eq!(fo.settings.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));
        assert_eq!(fo.settings.game_resolution, Some((1280, 720)));
        assert_eq!(fo.settings.pre_launch_hook.as_deref(), Some("echo pre"));
        assert_eq!(fo.settings.wrapper_command, None);

        let plain = &records["Vanilla"];
        assert_eq!(plain.game_version.as_deref(), Some("1.21"));
        assert!(loaders_from_record(plain).is_empty());
        assert_eq!(plain.last_played, None);
        assert_eq!(plain.seconds_played, None);
        assert_eq!(plain.settings, ImportedSettings::default());
    }

    #[test]
    fn old_schema_database_is_read() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("app.db");
        old_schema_db(&db);

        let records = read_database_records(&db).unwrap();
        let atm = &records["ATM9"];
        assert_eq!(atm.name.as_deref(), Some("All the Mods 9"));
        assert_eq!(
            loaders_from_record(atm),
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")]
        );
        assert_eq!(atm.seconds_played, Some(60));
        assert_eq!(atm.settings.group.as_deref(), Some("Kitchen sink"));
        assert_eq!(atm.settings.memory(), Some((8192, 8192)));
        assert_eq!(atm.settings.java_path.as_deref(), Some("/usr/bin/java"));
        assert_eq!(atm.settings.extra_java_args.as_deref(), Some("-XX:+UseZGC"));
        assert_eq!(atm.settings.wrapper_command.as_deref(), Some("gamemoderun"));
        assert_eq!(atm.settings.game_resolution, None);
    }

    #[test]
    fn profile_json_is_read() {
        let json = serde_json::json!({
            "install_stage": "installed",
            "path": "Cobblemon",
            "java": {
                "override_version": { "path": "/j/bin/java", "version": "17", "architecture": "x64" },
                "extra_arguments": ["-XX:+UseG1GC"],
                "custom_env_args": []
            },
            "memory": { "maximum": 4096 },
            "resolution": [1024, 768],
            "hooks": { "pre_launch": "echo", "wrapper": null, "post_exit": null },
            "metadata": {
                "name": "Cobblemon",
                "icon": "icons/cobblemon.png",
                "groups": ["Fun"],
                "game_version": "1.20.1",
                "loader": "fabric",
                "loader_version": { "id": "0.14.22", "url": "...", "stable": true },
                "linked_data": { "project_id": "5FFgwNNP", "version_id": "xyz", "locked": true },
                "date_created": "2023-09-01T00:00:00Z",
                "last_played": "2024-01-02T03:04:05Z",
                "submitted_time_played": 30,
                "recent_time_played": 12
            },
            "projects": {}
        });
        let record = record_from_profile_json(&json);
        assert_eq!(record.name.as_deref(), Some("Cobblemon"));
        assert_eq!(record.icon_path.as_deref(), Some("icons/cobblemon.png"));
        assert_eq!(record.game_version.as_deref(), Some("1.20.1"));
        assert_eq!(record.loader_version.as_deref(), Some("0.14.22"));
        assert_eq!(record.linked_project_id.as_deref(), Some("5FFgwNNP"));
        assert_eq!(record.linked_version_id.as_deref(), Some("xyz"));
        assert_eq!(
            record.last_played.map(|d| d.timestamp()),
            Some(1_704_164_645)
        );
        assert_eq!(record.seconds_played, Some(42));
        assert_eq!(record.settings.group.as_deref(), Some("Fun"));
        assert_eq!(record.settings.java_path.as_deref(), Some("/j/bin/java"));
        assert_eq!(
            record.settings.extra_java_args.as_deref(),
            Some("-XX:+UseG1GC")
        );
        assert_eq!(record.settings.memory(), Some((4096, 4096)));
        assert_eq!(record.settings.game_resolution, Some((1024, 768)));
        assert_eq!(record.settings.pre_launch_hook.as_deref(), Some("echo"));
        assert_eq!(record.settings.wrapper_command, None);

        let record = ProfileRecord {
            loader: Some("forge".into()),
            ..Default::default()
        };
        assert!(loaders_from_record(&record).is_empty());
        let record = ProfileRecord {
            loader: Some("liteloader".into()),
            loader_version: Some("1".into()),
            ..Default::default()
        };
        assert!(loaders_from_record(&record).is_empty());
    }

    #[test]
    fn layouts_are_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let profiles = root.join("profiles");
        let profile = profiles.join("Cobblemon");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(root.join("app.db"), b"").unwrap();

        assert_eq!(
            Layout::detect(root),
            Layout {
                profiles_dir: profiles.clone(),
                settings_dir: Some(root.to_path_buf()),
                single_profile: None,
            }
        );
        assert_eq!(
            Layout::detect(&profiles),
            Layout {
                profiles_dir: profiles.clone(),
                settings_dir: Some(root.to_path_buf()),
                single_profile: None,
            }
        );
        assert_eq!(
            Layout::detect(&profile),
            Layout {
                profiles_dir: profiles.clone(),
                settings_dir: Some(root.to_path_buf()),
                single_profile: Some(profile.clone()),
            }
        );

        let elsewhere = tempfile::tempdir().unwrap();
        let custom = elsewhere.path().join("elsewhere");
        std::fs::create_dir_all(custom.join("Pack")).unwrap();
        assert_eq!(
            Layout::detect(&custom),
            Layout {
                profiles_dir: custom.clone(),
                settings_dir: None,
                single_profile: None,
            }
        );
    }

    #[tokio::test]
    async fn profiles_are_scanned_with_database_and_json_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let profiles = root.join("profiles");
        new_schema_db(&root.join("app.db"));

        let fo = profiles.join("Fabulously Optimized");
        std::fs::create_dir_all(fo.join("mods")).unwrap();

        let legacy = profiles.join("Legacy");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(
            legacy.join("profile.json"),
            serde_json::json!({
                "metadata": { "name": "Legacy Pack", "game_version": "1.19.2", "loader": "quilt",
                              "loader_version": { "id": "0.19.0" } }
            })
            .to_string(),
        )
        .unwrap();

        let unknown = profiles.join("Unknown");
        std::fs::create_dir_all(&unknown).unwrap();

        let layout = Layout::detect(root);
        let records = load_records(&layout).await;
        assert!(records.contains_key("Fabulously Optimized"));

        let importer = ModrinthAppImporter::new();

        let entry = importer
            .scan_profile(fo.clone(), records.get("Fabulously Optimized").cloned())
            .await
            .unwrap()
            .unwrap();
        let InternalImportEntry::Valid(importable) = entry else {
            panic!("expected valid entry");
        };
        assert_eq!(importable.name, "Fabulously Optimized");
        assert_eq!(importable.minecraft_version, "1.20.4");
        assert_eq!(
            importable.modpack,
            Some(Modpack::Modrinth(ModrinthModpack {
                project_id: "1KVo5zza".into(),
                version_id: "abcd1234".into(),
            }))
        );
        assert_eq!(importable.icon_path, None);

        let entry = importer.scan_profile(legacy, None).await.unwrap().unwrap();
        let InternalImportEntry::Valid(importable) = entry else {
            panic!("expected valid entry");
        };
        assert_eq!(importable.name, "Legacy Pack");
        assert_eq!(
            importable.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Quilt, "0.19.0")]
        );

        assert!(
            importer
                .scan_profile(unknown, None)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn icon_paths_are_resolved() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let profile = root.join("profiles").join("P");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::create_dir_all(root.join("icons")).unwrap();
        std::fs::write(root.join("icons").join("a.png"), b"x").unwrap();
        std::fs::write(profile.join("local.png"), b"x").unwrap();

        assert_eq!(
            resolve_icon_path(&profile, "a.png"),
            Some(root.join("icons").join("a.png"))
        );
        assert_eq!(
            resolve_icon_path(&profile, "local.png"),
            Some(profile.join("local.png"))
        );
        assert_eq!(
            resolve_icon_path(
                &profile,
                &root.join("icons").join("a.png").to_string_lossy()
            ),
            Some(root.join("icons").join("a.png"))
        );
        assert_eq!(resolve_icon_path(&profile, "/nonexistent/x.png"), None);
        assert_eq!(resolve_icon_path(&profile, ""), None);
    }

    fn database_with_custom_dir(settings: &Path, custom: &Path) {
        let conn = rusqlite::Connection::open(settings.join("app.db")).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE settings (custom_dir TEXT NULL); INSERT INTO settings VALUES ('{}');",
            custom.to_string_lossy().replace('\'', "''")
        ))
        .unwrap();
    }

    /// A data folder whose database points at an unplugged drive has nothing
    /// to list; "No instance found" alone would hide why.
    #[tokio::test]
    async fn a_missing_configured_folder_is_named() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("ModrinthApp");
        std::fs::create_dir_all(settings.join("profiles").join("Leftover")).unwrap();
        let gone = tmp.path().join("unplugged");
        database_with_custom_dir(&settings, &gone);

        let layout = Layout::detect(&settings);
        let (_, error) = load_records_from_candidates(&layout, vec![settings.join("app.db")]).await;
        let error = format!("{:#}", error.expect("the missing folder was not reported"));
        assert!(error.contains(&gone.display().to_string()), "{error}");
        assert!(error.contains("choose another folder"), "{error}");

        // Also when there is no leftover folder to scan at all.
        std::fs::remove_dir_all(settings.join("profiles")).unwrap();
        let layout = Layout::detect(&settings);
        let (_, error) = load_records_from_candidates(&layout, vec![settings.join("app.db")]).await;
        assert!(error.is_some());

        // A folder that exists but holds no profiles yet is not missing.
        std::fs::create_dir_all(&gone).unwrap();
        let (_, error) = load_records_from_candidates(&layout, vec![settings.join("app.db")]).await;
        assert!(error.is_none(), "{error:?}");
    }

    /// Another install's database pointing somewhere gone says nothing about
    /// the folder the user selected.
    #[tokio::test]
    async fn a_missing_folder_of_another_install_is_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let other = tmp.path().join("OtherInstall");
        std::fs::create_dir_all(&other).unwrap();
        database_with_custom_dir(&other, &tmp.path().join("unplugged"));
        let selected = tmp.path().join("Selected");
        std::fs::create_dir_all(selected.join("profiles")).unwrap();

        let layout = Layout::detect(&selected);
        let (records, error) =
            load_records_from_candidates(&layout, vec![other.join("app.db")]).await;
        assert!(records.is_empty());
        assert!(error.is_none(), "{error:?}");
    }

    #[tokio::test]
    async fn custom_dir_relocates_profiles() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("ModrinthApp");
        std::fs::create_dir_all(&settings).unwrap();
        let custom = tmp.path().join("custom");
        std::fs::create_dir_all(custom.join("profiles")).unwrap();

        let conn = rusqlite::Connection::open(settings.join("app.db")).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE settings (custom_dir TEXT NULL); INSERT INTO settings VALUES ('{}');",
            custom.to_string_lossy().replace('\'', "''")
        ))
        .unwrap();
        drop(conn);

        assert_eq!(
            profiles_dir_for_settings_dir(&settings).await,
            custom.join("profiles")
        );
    }
}

#[cfg(test)]
mod stale_java_tests {
    use super::*;

    #[tokio::test]
    async fn a_java_path_that_is_gone_is_left_out() {
        let t = tempfile::tempdir().unwrap();
        let record = ProfileRecord {
            game_version: Some("1.20.1".into()),
            settings: ImportedSettings {
                java_path: Some("/gdl-no-such-folder/bin/java".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let Some(InternalImportEntry::Valid(entry)) = ModrinthAppImporter::new()
            .scan_profile(t.path().to_path_buf(), Some(record))
            .await
            .unwrap()
        else {
            panic!("a profile with a stale Java path was refused")
        };
        assert_eq!(entry.settings.java_path, None);
        assert_eq!(
            ImportableInstance::from(entry).warnings,
            [Translation::InstanceImportJavaPathSkipped]
        );
    }
}
