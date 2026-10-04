//! One validated server-data-relative world resolver for save protection and addons.
//!
//! Links follow the rule instances follow: the server's data folder, its
//! world and its addon folders may be symlinks, or junctions on Windows, to
//! folders kept elsewhere (a world moved to another drive, a `mods/` shared
//! between servers), and they are followed, as 2.0.40 followed them. Only the
//! entry an action renames, deletes or replaces is checked itself (see
//! [`owned_addon`]), and a pack never writes through a link (see
//! [`Destinations::effective`] and [`SaveProtection`]).
use anyhow::{Context, Result, bail};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
};

/// The world `level-name` selects, read as the server reads
/// `server.properties` (see [`super::properties::java_properties`]).
pub fn selected(data: &Path) -> Result<String> {
    let path = data.join("server.properties");
    let content = match std::fs::read(&path) {
        Ok(bytes) => super::properties::decode(bytes).0,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(
                anyhow::Error::new(error).context(format!("Could not read {}", path.display()))
            );
        }
    };
    let props = super::properties::java_properties(&content);
    let name = props
        .get("level-name")
        .map(String::as_str)
        .unwrap_or("world");
    if validate_local(name).is_err() {
        bail!(
            "The server's world name (level-name in server.properties) is `{name}`, which GDLauncher cannot use as a folder inside the server folder. In your file manager, move or rename the world's folder `{name}` so it sits directly inside the server folder under a plain name (letters, numbers and spaces), then set World Name in the server's Properties tab to that same name and try again. Changing only the World Name makes the server create a new, empty world."
        );
    }
    Ok(name.to_owned())
}

/// A path inside the server folder that means the same on every OS:
/// `/`-separated names, none of them empty, `.` or `..`, and no `\`, `:` or
/// NUL (Windows reads those as separators, drives or streams). Asset keys use
/// the same rule.
pub fn validate_relative(name: &str) -> Result<()> {
    validate(name, &['\\', ':', '\0'])
}

/// [`validate_relative`] for a path only this computer's server folder
/// holds: the world the server uses, an addon in it, a file a pack writes.
/// Outside Windows `:` is an ordinary character in a name: the server keeps
/// the world `SMP: Season 2` (saved as `level-name=SMP\: Season 2`) in a
/// folder by that name, and 2.0.40 wrote pack files so named. Windows reads
/// it as a stream, so refuses it there.
pub fn validate_local(name: &str) -> Result<()> {
    if cfg!(windows) {
        validate_relative(name)
    } else {
        validate(name, &['\\', '\0'])
    }
}

fn validate(name: &str, forbidden: &[char]) -> Result<()> {
    if name.is_empty()
        || name.contains(forbidden)
        || name.starts_with('/')
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        bail!("Unsafe server relative location: {name}");
    }
    Ok(())
}

/// `relative` inside the server's data folder, for a folder GDLauncher reads
/// or puts addons into, or an addon in one. Links on the way are followed
/// (see the module docs).
pub fn folder(data: &Path, relative: &str) -> Result<PathBuf> {
    validate_local(relative)?;
    let mut path = data.to_path_buf();
    path.extend(relative.split('/'));
    Ok(path)
}

/// [`folder`] for a path a pack writes: a link anywhere below the data folder
/// is refused, as what it points to is not part of the server and the save
/// check never looked inside it (see [`SaveProtection::read`]). The data
/// folder itself may be a link.
fn checked_join(data: &Path, relative: &str) -> Result<PathBuf> {
    let path = folder(data, relative)?;
    if let Ok(meta) = std::fs::metadata(data) {
        if !meta.is_dir() {
            bail!("{} is not a folder", data.display());
        }
    }
    let mut current = data.to_path_buf();
    let mut parts = Vec::new();
    for part in relative.split('/') {
        current.push(part);
        parts.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(ThroughLink {
                    link: parts.join("/"),
                    path: current,
                }
                .into());
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(path)
}

/// Why [`checked_join`] refused a path: a link on the way.
#[derive(Debug)]
pub struct ThroughLink {
    /// The link, as a `/`-separated path inside the data folder.
    pub link: String,
    path: PathBuf,
}

impl std::fmt::Display for ThroughLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is a link, and GDLauncher does not write through links",
            self.path.display()
        )
    }
}

impl std::error::Error for ThroughLink {}

pub fn datapacks(data: &Path) -> Result<PathBuf> {
    folder(data, &format!("{}/datapacks", selected(data)?))
}

/// `raw`, a path from a pack, as a `/`-separated path inside `data`; an
/// error when it leaves `data`. Nothing else is checked (see
/// [`normalized_relative`]).
pub fn contained_relative(data: &Path, raw: &str) -> Result<String> {
    let path =
        crate::managers::minecraft::modrinth::secure_path_join(data, raw.replace('\\', "/"))?;
    Ok(path
        .strip_prefix(data)?
        .to_str()
        .context("Non-UTF8 pack location")?
        .replace(std::path::MAIN_SEPARATOR, "/"))
}

/// Normalize in-root archive aliases before applying any ownership policy.
pub fn normalized_relative(data: &Path, raw: &str) -> Result<String> {
    let relative = contained_relative(data, raw)?;
    validate_local(&relative)?;
    Ok(relative)
}

/// The entry `requested` names, by name alone: itself when it is listed or
/// the filesystem finds nothing by that name, else its only case alias.
/// `None` when the names cannot tell.
fn spelling_by_name(requested: &str, entries: &[String], resolves: bool) -> Option<String> {
    if !resolves || entries.iter().any(|entry| entry == requested) {
        return Some(requested.into());
    }
    let mut aliases = entries
        .iter()
        .filter(|entry| entry.eq_ignore_ascii_case(requested));
    match (aliases.next(), aliases.next()) {
        (Some(alias), None) => Some(alias.clone()),
        _ => None,
    }
}

/// The entry the filesystem opens for `requested` when no name tells: a
/// Unicode alias, such as a folder macOS stores decomposed (`é` as `e` and an
/// accent) that is asked for composed, or a name Windows trims or shortens.
fn spelling_by_identity(
    parent: &Path,
    requested: &str,
    entries: &[String],
    same: impl Fn(&Path, &Path) -> bool,
) -> Result<String> {
    let wanted = parent.join(requested);
    let mut matches = entries
        .iter()
        .filter(|entry| same(&parent.join(entry), &wanted));
    match (matches.next(), matches.next()) {
        (Some(entry), None) => Ok(entry.clone()),
        (None, _) => bail!("Filesystem alias cannot be safely classified"),
        _ => bail!("Ambiguous filesystem case alias"),
    }
}

/// The names in `folder` (none when it does not exist). A name that is not
/// Unicode cannot be the spelling of one that is, so it is left out.
fn list(folder: &Path) -> Result<Vec<String>> {
    match std::fs::read_dir(folder) {
        Ok(entries) => {
            let mut names = Vec::new();
            for entry in entries {
                if let Ok(name) = entry?.file_name().into_string() {
                    names.push(name);
                }
            }
            Ok(names)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

/// Resolve existing component spelling only when the actual filesystem accepts
/// the alias. Case-distinct entries stay distinct on case-sensitive volumes.
pub fn effective_destination(data: &Path, raw: &str) -> Result<(String, PathBuf)> {
    Destinations::new(data, filesystem_resolves).effective(raw)
}
/// Only the filesystem's component lookup is injectable; normalization,
/// existing spelling, symlink checks and the returned write path stay shared.
pub(super) type ComponentLookup = fn(&Path, &str, &[String]) -> Result<bool>;
pub(super) fn filesystem_resolves(
    parent: &Path,
    component: &str,
    _entries: &[String],
) -> Result<bool> {
    Ok(parent.join(component).try_exists()?)
}

/// Resolves pack paths to the spelling already on disk. One is used for a
/// whole extraction: each folder is listed once, and again when the
/// filesystem finds a name its listing lacks (a folder the extraction made
/// since, or another spelling of an entry). What a new listing tells about
/// such a name is kept, so a folder is listed again once per name.
pub(super) struct Destinations<'a> {
    data: &'a Path,
    lookup: ComponentLookup,
    listings: HashMap<PathBuf, Vec<String>>,
    /// Paths the filesystem opens as an entry spelled otherwise, with that
    /// entry's name.
    aliases: HashMap<PathBuf, String>,
    /// Links pack files were not written through, to tell the user.
    skipped_links: BTreeSet<String>,
}

impl<'a> Destinations<'a> {
    pub(super) fn new(data: &'a Path, lookup: ComponentLookup) -> Self {
        Self {
            data,
            lookup,
            listings: HashMap::new(),
            aliases: HashMap::new(),
            skipped_links: BTreeSet::new(),
        }
    }

    pub(super) fn data(&self) -> &'a Path {
        self.data
    }

    /// Records that pack files were not written through `link` (see
    /// [`ThroughLink`]).
    pub(super) fn skip_link(&mut self, link: String) {
        self.skipped_links.insert(link);
    }

    /// The links pack files were not written through.
    pub(super) fn into_skipped_links(self) -> BTreeSet<String> {
        self.skipped_links
    }

    /// The on-disk spelling of `raw`, and the path a pack writes it to, which
    /// has no link on the way (see [`checked_join`]).
    pub(super) fn effective(&mut self, raw: &str) -> Result<(String, PathBuf)> {
        let relative = self.spelled(raw)?;
        let path = checked_join(self.data, &relative)?;
        Ok((relative, path))
    }

    /// The on-disk spelling of `raw`, through links on the way.
    fn spelled(&mut self, raw: &str) -> Result<String> {
        let relative = normalized_relative(self.data, raw)?;
        let mut path = self.data.to_path_buf();
        let mut effective = Vec::new();
        for component in relative.split('/') {
            let spelling = self.spelling(&path, component)?;
            path.push(&spelling);
            effective.push(spelling);
        }
        Ok(effective.join("/"))
    }

    fn spelling(&mut self, parent: &Path, component: &str) -> Result<String> {
        let requested = parent.join(component);
        if let Some(alias) = self.aliases.get(&requested) {
            return Ok(alias.clone());
        }
        let mut fresh = false;
        if !self.listings.contains_key(parent) {
            self.listings.insert(parent.to_owned(), list(parent)?);
            fresh = true;
        }
        loop {
            let entries = &self.listings[parent];
            if entries.iter().any(|entry| entry == component)
                || !(self.lookup)(parent, component, entries)?
            {
                // Listed, or nothing by that name: used as asked.
                return Ok(component.to_owned());
            }
            if fresh {
                let spelling = match spelling_by_name(component, entries, true) {
                    Some(spelling) => spelling,
                    None => spelling_by_identity(
                        parent,
                        component,
                        entries,
                        crate::util::publish::same_file,
                    )?,
                };
                self.aliases.insert(requested, spelling.clone());
                return Ok(spelling);
            }
            // The filesystem finds a name the listing lacks: the extraction
            // made it since the folder was listed, or it is another spelling
            // of an entry. Only a new listing tells which; an alias taken from
            // the old one would put a case-distinct entry on a case-sensitive
            // volume into another folder than the archive names.
            self.listings.insert(parent.to_owned(), list(parent)?);
            fresh = true;
        }
    }
}

/// Whether `path` is `folder` or inside it.
fn within(path: &str, folder: &str) -> bool {
    path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

pub struct SaveProtection {
    data: PathBuf,
    /// Worlds: the one `level-name` selects and every folder with a
    /// `level.dat`.
    roots: BTreeSet<String>,
    /// Folders that could not be opened, so may hold a world.
    unreadable: BTreeSet<String>,
}
impl SaveProtection {
    /// Finds the server's worlds: the one `level-name` selects and every
    /// folder with a `level.dat`, however deep, a folder Repair deletes
    /// included. A world's folder, and a top-level save name (see
    /// [`super::modpack::is_save_path`]), is protected whole, so the walk does
    /// not look inside it. Links are not followed: a pack never writes
    /// through one, and Repair removes a linked folder it reinstalls without
    /// touching what it points to. A folder that cannot be opened is
    /// protected as it is. A `level-name` that cannot be used leaves the
    /// default world names and the folders with a `level.dat` protected.
    pub fn read(data: &Path) -> Result<Self> {
        let mut roots = BTreeSet::new();
        match selected(data) {
            Ok(world) => {
                let spelled = Destinations::new(data, filesystem_resolves)
                    .spelled(&world)
                    .unwrap_or_else(|error| {
                        tracing::warn!(%world, "Protecting the selected world as written: {error:#}");
                        world.clone()
                    });
                roots.insert(spelled);
            }
            Err(error) => tracing::warn!(
                "{error:#} Protecting the default world names and every folder with a level.dat."
            ),
        }
        let mut unreadable = BTreeSet::new();
        let mut pending = vec![(data.to_path_buf(), String::new())];
        while let Some((dir, rel)) = pending.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) if rel.is_empty() => {
                    return Err(anyhow::Error::new(error).context(format!(
                        "Could not open the server folder {}",
                        dir.display()
                    )));
                }
                Err(error) => {
                    tracing::warn!(folder = %dir.display(), "Protecting a folder that cannot be opened: {error}");
                    unreadable.insert(rel);
                    continue;
                }
            };
            let mut folders = Vec::new();
            let mut save = false;
            let mut complete = true;
            for entry in entries {
                let Ok(entry) = entry else {
                    complete = false;
                    break;
                };
                let name = entry.file_name().to_string_lossy().into_owned();
                let child = if rel.is_empty() {
                    name.clone()
                } else {
                    format!("{rel}/{name}")
                };
                let Ok(kind) = entry.file_type() else {
                    unreadable.insert(child);
                    continue;
                };
                if kind.is_symlink() {
                    continue;
                }
                if kind.is_dir() {
                    let whole = (rel.is_empty() && super::modpack::is_save_path(&name))
                        || roots.contains(&child);
                    if !whole {
                        folders.push((entry.path(), child));
                    }
                } else if kind.is_file() && matches!(name.as_str(), "level.dat" | "level.dat_old") {
                    save = true;
                }
            }
            if rel.is_empty() {
                if !complete {
                    bail!("Could not list the server folder {}", dir.display());
                }
                if save {
                    bail!(
                        "The server folder {} has a level.dat directly inside it, so GDLauncher cannot tell the world from the server's own files and will not install a pack into it. Move the world into a folder of its own, set World Name in the server's Properties tab to that folder, then try again.",
                        dir.display()
                    );
                }
            } else if save {
                // Everything inside is protected with it.
                roots.insert(rel);
                continue;
            } else if !complete {
                unreadable.insert(rel.clone());
            }
            pending.extend(folders);
        }
        Ok(Self {
            roots,
            unreadable,
            data: data.to_path_buf(),
        })
    }
    pub fn protects(&self, relative: &str) -> bool {
        let Ok((relative, _)) = effective_destination(&self.data, relative) else {
            return true;
        };
        self.protects_effective(&relative)
    }
    /// [`Self::protects`] for a path already in its on-disk spelling, with no
    /// link on the way (see [`Destinations::effective`]).
    pub(super) fn protects_effective(&self, relative: &str) -> bool {
        super::modpack::is_save_path(relative)
            || self
                .roots
                .iter()
                .chain(&self.unreadable)
                .any(|root| within(relative, root))
    }
    /// Refuses a Repair that deletes `raw` (a folder or file it reinstalls)
    /// while a world is in it or around it, or while a folder in it could
    /// not be checked for one. A link there is no reason to refuse: Repair
    /// removes the link only, never what it points to.
    pub fn refuse_wipe(&self, raw: &str) -> Result<()> {
        let relative = Destinations::new(&self.data, filesystem_resolves).spelled(raw)?;
        let path = |relative: &str| self.data.join(relative).display().to_string();
        if let Some(world) = self.roots.iter().find(|root| within(root, &relative)) {
            bail!(
                "Repair deletes and reinstalls {}, and there is a world in it at {}. Move that world out of {relative}, then run Repair again.",
                path(&relative),
                path(world)
            );
        }
        if let Some(world) = self.roots.iter().find(|root| within(&relative, root)) {
            bail!(
                "Repair deletes and reinstalls {}, which is inside the world at {}. Set World Name in the server's Properties tab to the world's own folder, then run Repair again.",
                path(&relative),
                path(world)
            );
        }
        if super::modpack::is_save_path(&relative) {
            bail!(
                "Repair would delete {}, which holds world data.",
                path(&relative)
            );
        }
        if let Some(folder) = self
            .unreadable
            .iter()
            .find(|folder| within(folder, &relative) || within(&relative, folder))
        {
            bail!(
                "Repair deletes and reinstalls {}, but GDLauncher cannot open {} to check it for worlds. Give GDLauncher access to that folder, or move it out of the server folder, then run Repair again.",
                path(&relative),
                path(folder)
            );
        }
        Ok(())
    }
}

/// The file a server addon row is, for an action that renames or deletes
/// it: a regular file, or a link to one (2.0.40 listed such links), which
/// the action renames or removes itself, never what it points to. Its folder
/// may be a link (see the module docs). Never reinterprets an old-world ID
/// in today's world.
pub fn owned_addon_entry(
    data: &Path,
    kind: &str,
    location: &str,
    filename: &str,
) -> Result<PathBuf> {
    Ok(owned(data, kind, location, filename)?.path)
}

/// [`owned_addon_entry`] for an action that replaces the file: a link is
/// refused, as the file it points to may be shared with other servers.
pub fn owned_addon(data: &Path, kind: &str, location: &str, filename: &str) -> Result<PathBuf> {
    let found = owned(data, kind, location, filename)?;
    if found.link {
        bail!(
            "{} in the server's {} ({}) is a link to a file stored elsewhere, so GDLauncher will not replace it. Delete it in the server's Addons tab (only the link is removed; the file it points to is kept), then install the version you want.",
            found.name,
            found.folder_name,
            found.folder.display()
        );
    }
    Ok(found.path)
}

struct OwnedAddon {
    path: PathBuf,
    /// Whether `path` is a link to the file.
    link: bool,
    name: String,
    folder: PathBuf,
    folder_name: &'static str,
}

fn owned(data: &Path, kind: &str, location: &str, filename: &str) -> Result<OwnedAddon> {
    validate_relative(filename)?;
    if filename.contains('/') {
        bail!("Addon filename must be a basename");
    }
    let (expected, folder_name) = match kind {
        "mods" => (format!("mods/{filename}"), "mods folder"),
        "datapacks" => (
            format!("{}/datapacks/{filename}", selected(data)?),
            "datapacks folder",
        ),
        _ => bail!("Unsupported server addon kind"),
    };
    if location != expected {
        bail!("Addon belongs to an inactive world or stale owner; refresh the server addons");
    }
    let enabled = folder(data, location)?;
    let parent = enabled
        .parent()
        .context("Server addon has no folder")?
        .to_path_buf();
    let disabled = parent.join(format!("{filename}.disabled"));
    let lstat = |path: &Path| match std::fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(anyhow::Error::new(error).context(format!("Could not open {}", path.display())))
        }
    };
    let (path, name, meta) = match (lstat(&enabled)?, lstat(&disabled)?) {
        (Some(_), Some(_)) => bail!(
            "Both {filename} and {filename}.disabled are in the server's {folder_name} ({}), so GDLauncher cannot tell which copy is the addon. Delete the copy you do not want from that folder, then try again. Open Folder in the server's Addons tab opens the server folder; the copies are in its {} folder.",
            parent.display(),
            location
                .rsplit_once('/')
                .map_or(location, |(folder, _)| folder)
        ),
        (None, None) => match unavailable_link(&parent) {
            Some(link) => bail!(
                "{} is a link to a folder that cannot be found. Reconnect the drive or folder it points to, or remove the link, then try again.",
                link.display()
            ),
            None => bail!(
                "{filename} is no longer in the server's {folder_name} ({}). Refresh the server's Addons tab.",
                parent.display()
            ),
        },
        (Some(meta), None) => (enabled, filename.to_owned(), meta),
        (None, Some(meta)) => (disabled, format!("{filename}.disabled"), meta),
    };
    let link = meta.file_type().is_symlink();
    let target = if link {
        match std::fs::metadata(&path) {
            Ok(target) => target,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => bail!(
                "{name} in the server's {folder_name} ({}) is a link to a file that cannot be found. Reconnect the drive or folder it points to, or remove the link, then try again.",
                parent.display()
            ),
            Err(error) => {
                return Err(
                    anyhow::Error::new(error).context(format!("Could not open {}", path.display()))
                );
            }
        }
    } else {
        meta
    };
    if !target.is_file() {
        bail!(
            "{name} in the server's {folder_name} ({}) is a folder or a special file, not an addon file. Move or rename it in your file manager, then try again.",
            parent.display()
        );
    }
    Ok(OwnedAddon {
        path,
        link,
        name,
        folder: parent,
        folder_name,
    })
}

/// The link on the way to `path` (`path` included) whose target cannot be
/// found, such as a world or `mods/` kept on a drive that is not plugged in.
/// `None` when `path` is there, or is missing for another reason.
pub fn unavailable_link(path: &Path) -> Option<PathBuf> {
    let mut path = path;
    loop {
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return std::fs::metadata(path).is_err().then(|| path.to_path_buf());
            }
            Ok(_) => return None,
            Err(_) => path = path.parent()?,
        }
    }
}

/// Links `link` to the folder `target`, as a user does: a symlink, or on
/// Windows a junction (`mklink /J`, which needs no rights).
#[cfg(test)]
pub(crate) fn link_folder(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    {
        // mklink reads a `/` in its arguments as a switch.
        let native = |path: &Path| PathBuf::from(path.to_string_lossy().replace('/', "\\"));
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(native(link))
            .arg(native(target))
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed for {}", link.display());
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LegacyDatapack {
    pub filename: String,
    pub sha512: String,
    pub selected_world: String,
}
/// Bounded positive evidence only: an ordinary ZIP is never a datapack candidate.
pub fn legacy_datapack_evidence(path: &Path) -> Result<bool> {
    use std::io::Read;
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > 128 * 1024 * 1024
    {
        return Ok(false);
    }
    let mut archive = match zip::ZipArchive::new(std::fs::File::open(path)?) {
        Ok(archive) => archive,
        Err(_) => return Ok(false),
    };
    if archive.len() > 2048 {
        return Ok(false);
    }
    let has_data = archive
        .file_names()
        .any(|name| name.starts_with("data/") && !name.ends_with('/'));
    if !has_data {
        return Ok(false);
    }
    let mut metadata = match archive.by_name("pack.mcmeta") {
        Ok(file) => file,
        Err(_) => return Ok(false),
    };
    if metadata.size() > 65536 {
        return Ok(false);
    }
    let mut bytes = Vec::new();
    metadata.by_ref().take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Ok(false);
    }
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return Ok(false),
    };
    Ok(value
        .get("pack")
        .and_then(|pack| pack.get("pack_format"))
        .and_then(|v| v.as_u64())
        .is_some())
}

#[cfg(test)]
mod task07_tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn task07_world_switch_rejects_old_owner_and_preserves_both_files() {
        let root = tempfile::tempdir().unwrap();
        for world in ["A", "B"] {
            std::fs::create_dir_all(root.path().join(format!("{world}/datapacks"))).unwrap();
            std::fs::write(
                root.path().join(format!("{world}/datapacks/pack.zip")),
                world,
            )
            .unwrap();
        }
        std::fs::write(root.path().join("server.properties"), "level-name=A\n").unwrap();
        assert!(owned_addon(root.path(), "datapacks", "A/datapacks/pack.zip", "pack.zip").is_ok());
        std::fs::write(root.path().join("server.properties"), "level-name=B\n").unwrap();
        assert!(owned_addon(root.path(), "datapacks", "A/datapacks/pack.zip", "pack.zip").is_err());
        assert_eq!(
            std::fs::read_to_string(
                owned_addon(root.path(), "datapacks", "B/datapacks/pack.zip", "pack.zip").unwrap()
            )
            .unwrap(),
            "B"
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("A/datapacks/pack.zip")).unwrap(),
            "A"
        );
    }
    #[test]
    fn task07_protection_refuses_wipe_overlap_and_unsafe_selected_world() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("mods/nested-world")).unwrap();
        std::fs::write(root.path().join("mods/nested-world/level.dat"), b"save").unwrap();
        let protection = SaveProtection::read(root.path()).unwrap();
        assert!(protection.refuse_wipe("mods").is_err());
        assert!(protection.refuse_wipe("libraries").is_ok());
        assert!(protection.protects("mods/nested-world/region/r.0.0.mca"));
        std::fs::write(
            root.path().join("server.properties"),
            "level-name=../outside\n",
        )
        .unwrap();
        assert!(selected(root.path()).is_err());
    }
    #[test]
    fn task07_legacy_zip_requires_positive_bounded_datapack_metadata() {
        let root = tempfile::tempdir().unwrap();
        for (name, metadata, expected) in [
            ("pack.zip", r#"{"pack":{"pack_format":48}}"#, true),
            ("unrelated.zip", r#"{"description":"backup"}"#, false),
        ] {
            let path = root.path().join(name);
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            zip.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(metadata.as_bytes()).unwrap();
            zip.start_file(
                "data/example/functions/a.mcfunction",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"say hi").unwrap();
            zip.finish().unwrap();
            assert_eq!(legacy_datapack_evidence(&path).unwrap(), expected);
        }
    }
}

#[cfg(test)]
#[test]
fn task07r1_r1_case_policy_uses_filesystem_acceptance_not_global_folding() {
    assert_eq!(
        spelling_by_name("adventuremap", &["AdventureMap".into()], true).unwrap(),
        "AdventureMap"
    );
    assert_eq!(
        spelling_by_name("adventuremap", &["AdventureMap".into()], false).unwrap(),
        "adventuremap"
    );
    assert_eq!(
        spelling_by_name(
            "adventuremap",
            &["AdventureMap".into(), "adventuremap".into()],
            true
        )
        .unwrap(),
        "adventuremap"
    );
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("AdventureMap")).unwrap();
    std::fs::write(
        root.path().join("server.properties"),
        "level-name=AdventureMap",
    )
    .unwrap();
    let protection = SaveProtection::read(root.path()).unwrap();
    assert!(protection.protects("config/../AdventureMap/level.dat"));
    if !root.path().join("adventuremap").exists() {
        std::fs::create_dir(root.path().join("adventuremap")).unwrap();
        assert!(!protection.protects("adventuremap/config.txt"));
    }
}

#[cfg(test)]
mod link_tests {
    use super::*;

    fn write(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    /// A world name the server saved with Java escapes (`World \#2`), or in
    /// ISO-8859-1, selects the folder the server uses, so Repair and
    /// datapack installs find it.
    #[test]
    fn an_escaped_or_latin1_world_name_selects_the_servers_folder() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        for (written, expected) in [
            (b"level-name=World \\#2\n".to_vec(), "World #2"),
            (b"level-name=Hardcore\\!\n".to_vec(), "Hardcore!"),
            (
                b"level-name=Welt Gr\\u00F6\\u00DFe\n".to_vec(),
                "Welt Größe",
            ),
            (b"level-name=Welt Gr\xF6\xDFe\n".to_vec(), "Welt Größe"),
        ] {
            std::fs::write(data.join("server.properties"), &written).unwrap();
            assert_eq!(selected(data).unwrap(), expected);
            assert_eq!(
                datapacks(data).unwrap(),
                data.join(expected).join("datapacks")
            );
        }
        std::fs::create_dir_all(data.join("Welt Größe/region")).unwrap();
        let protection = SaveProtection::read(data).unwrap();
        assert!(protection.protects("Welt Größe/level.dat"));
        assert!(protection.refuse_wipe("mods").is_ok());
    }

    /// A world name GDLauncher cannot use no longer refuses Repair: the
    /// default world names and every folder with a level.dat stay protected.
    #[test]
    fn an_unusable_world_name_still_protects_every_world_found() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        std::fs::write(data.join("server.properties"), "level-name=../elsewhere\n").unwrap();
        write(&data.join("survival/level.dat"), b"save");
        write(&data.join("mods/nested/level.dat"), b"save");
        let error = selected(data).unwrap_err().to_string();
        assert!(error.contains("World Name"), "{error}");
        // The way out keeps the world: only renaming World Name starts a new one.
        assert!(
            error.contains("move or rename the world's folder"),
            "{error}"
        );
        assert!(error.contains("new, empty world"), "{error}");
        let protection = SaveProtection::read(data).unwrap();
        assert!(protection.protects("world/level.dat"));
        assert!(protection.protects("survival/region/r.0.0.mca"));
        assert!(protection.refuse_wipe("mods").is_err());
        assert!(protection.refuse_wipe("libraries").is_ok());
    }

    /// Outside Windows a world named `SMP: Season 2` (saved as
    /// `level-name=SMP\: Season 2`) is the folder the server uses, and its
    /// datapacks and pack files so named are used as 2.0.40 used them;
    /// Windows reads `:` as a stream, so refuses it there.
    #[test]
    fn a_colon_is_an_ordinary_character_outside_windows() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        std::fs::write(
            data.join("server.properties"),
            "level-name=SMP\\: Season 2\n",
        )
        .unwrap();
        assert!(validate_relative("SMP: Season 2").is_err());
        if cfg!(windows) {
            assert!(validate_local("SMP: Season 2").is_err());
            assert!(selected(data).is_err());
            assert!(normalized_relative(data, "mods/foo:bar.jar").is_err());
            return;
        }
        assert_eq!(selected(data).unwrap(), "SMP: Season 2");
        assert_eq!(
            datapacks(data).unwrap(),
            data.join("SMP: Season 2").join("datapacks")
        );
        write(&data.join("SMP: Season 2/datapacks/p.zip"), b"p");
        assert_eq!(
            owned_addon(data, "datapacks", "SMP: Season 2/datapacks/p.zip", "p.zip").unwrap(),
            data.join("SMP: Season 2/datapacks/p.zip")
        );
        assert_eq!(
            normalized_relative(data, "mods/foo:bar.jar").unwrap(),
            "mods/foo:bar.jar"
        );
        let protection = SaveProtection::read(data).unwrap();
        assert!(protection.protects("SMP: Season 2/level.dat"));
        assert!(!protection.protects("mods/foo:bar.jar"));
        for unsafe_name in ["..", "a/../b", "/abs", "a\\b", "a\0b", ""] {
            assert!(validate_local(unsafe_name).is_err(), "{unsafe_name:?}");
        }
    }

    /// A world on another drive, a `mods/` shared between servers, a linked
    /// jar and a stray link under backups no longer refuse Repair: links are
    /// not followed, a pack never writes through one, and Repair removes a
    /// linked folder it reinstalls (only the link). Windows junctions behave
    /// as links.
    #[test]
    fn links_in_the_server_folder_do_not_refuse_repair() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("server");
        let elsewhere = root.path().join("elsewhere");
        write(&elsewhere.join("world/level.dat"), b"save");
        write(&elsewhere.join("shared-mods/shared.jar"), b"jar");
        write(&elsewhere.join("config/shared.toml"), b"shared");
        write(&elsewhere.join("old-backup/level.dat"), b"backup");
        write(&data.join("server.properties"), b"level-name=world\n");
        write(&data.join("libraries/lib.jar"), b"lib");
        std::fs::create_dir_all(data.join("backups")).unwrap();
        link_folder(&elsewhere.join("world"), &data.join("world"));
        link_folder(&elsewhere.join("shared-mods"), &data.join("mods"));
        link_folder(&elsewhere.join("config"), &data.join("config"));
        link_folder(
            &elsewhere.join("old-backup"),
            &data.join("backups").join("latest"),
        );
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut through_links = vec![
            "world/level.dat",
            "config/shared.toml",
            "backups/latest/level.dat",
        ];
        // A linked file needs rights on Windows.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                elsewhere.join("shared-mods/shared.jar"),
                data.join("libraries/linked.jar"),
            )
            .unwrap();
            through_links.push("libraries/linked.jar");
        }

        let protection = SaveProtection::read(&data).unwrap();
        for name in ["mods", "libraries", "defaultconfigs", "launch_config.json"] {
            protection.refuse_wipe(name).unwrap();
        }
        // Nothing is written through a link.
        for raw in through_links {
            assert!(protection.protects(raw), "{raw}");
        }
        assert!(!protection.protects("config-new/a.toml"));
        // The selected world, a link, is used as it is.
        assert_eq!(selected(&data).unwrap(), "world");
        assert_eq!(
            datapacks(&data).unwrap(),
            data.join("world").join("datapacks")
        );
    }

    /// A folder GDLauncher cannot open is protected as it is; Repair is only
    /// refused when it would delete it, with the way out.
    #[cfg(unix)]
    #[test]
    fn a_folder_that_cannot_be_opened_is_protected_not_fatal() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        write(&data.join("backups/locked/level.dat"), b"save");
        write(&data.join("mods/locked/a.jar"), b"jar");
        let locked = [data.join("backups/locked"), data.join("mods/locked")];
        for path in &locked {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        let enforced = std::fs::read_dir(&locked[0]).is_err();
        let protection = SaveProtection::read(data);
        for path in &locked {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        if !enforced {
            return; // Permissions are not enforced for this user.
        }
        let protection = protection.unwrap();
        assert!(protection.protects("backups/locked/level.dat"));
        assert!(!protection.protects("backups/other/a.txt"));
        let error = protection.refuse_wipe("mods").unwrap_err().to_string();
        assert!(error.contains("cannot open"), "{error}");
        assert!(error.contains("mods/locked"), "{error}");
        assert!(protection.refuse_wipe("libraries").is_ok());
    }

    /// macOS stores a folder made by Java 17 or older decomposed (`é` as `e`
    /// and an accent) while the world name asks for it composed; Windows trims
    /// a trailing dot. The entry the filesystem opens is used.
    #[test]
    fn a_name_only_the_filesystem_matches_is_found_by_identity() {
        let decomposed = "Monde-Cre\u{301}atif".to_owned();
        let entries = vec!["other".to_owned(), decomposed.clone()];
        assert_eq!(spelling_by_name("Monde-Créatif", &entries, true), None);
        let parent = Path::new("/data");
        let same =
            |a: &Path, b: &Path| a == parent.join(&decomposed) && b == parent.join("Monde-Créatif");
        assert_eq!(
            spelling_by_identity(parent, "Monde-Créatif", &entries, same).unwrap(),
            decomposed
        );
        assert!(spelling_by_identity(parent, "Monde-Créatif", &entries, |_, _| false).is_err());
        assert!(spelling_by_identity(parent, "x", &entries, |_, _| true).is_err());
    }

    /// One resolver serves a whole extraction: a folder it listed before the
    /// extraction made a subfolder still finds that subfolder's spelling.
    #[test]
    fn an_extraction_finds_folders_it_made_after_listing() {
        // A case-insensitive volume, simulated on the real folder.
        fn insensitive(parent: &Path, component: &str, _: &[String]) -> Result<bool> {
            Ok(list(parent)?
                .iter()
                .any(|entry| entry.eq_ignore_ascii_case(component)))
        }
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        let mut destinations = Destinations::new(data, filesystem_resolves);
        let (relative, path) = destinations.effective("Config/a.toml").unwrap();
        assert_eq!(relative, "Config/a.toml");
        write(&path, b"a");
        assert_eq!(
            destinations.effective("Config/b.toml").unwrap().0,
            "Config/b.toml"
        );
        // `other2` resolves to the folder made as `Other2` after the data
        // folder was listed.
        let mut destinations = Destinations::new(data, insensitive);
        destinations.effective("Other/x.toml").unwrap();
        std::fs::create_dir(data.join("Other2")).unwrap();
        assert_eq!(
            destinations.effective("other2/y.toml").unwrap().0,
            "Other2/y.toml"
        );
    }

    /// Toggle and Delete act on a regular file or a link to one, through a
    /// linked folder (a junction on Windows); replacing refuses a link; two
    /// copies, a dangling link and a folder are refused with what to do.
    #[test]
    fn owned_addons_follow_linked_folders_and_explain_refusals() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("server");
        let shared = root.path().join("shared");
        write(&shared.join("a.jar"), b"a");
        write(&shared.join("b.jar"), b"b");
        write(&shared.join("b.jar.disabled"), b"b");
        std::fs::create_dir(shared.join("folder.jar")).unwrap();
        std::fs::create_dir_all(&data).unwrap();
        link_folder(&shared, &data.join("mods"));
        write(&root.path().join("worlds/main/datapacks/p.zip"), b"p");
        write(&root.path().join("worlds/main/datapacks/q.zip"), b"q");
        write(
            &root.path().join("worlds/main/datapacks/q.zip.disabled"),
            b"q",
        );
        link_folder(
            &root.path().join("worlds").join("main"),
            &data.join("world"),
        );

        let entry = |name: &str| owned_addon_entry(&data, "mods", &format!("mods/{name}"), name);
        assert_eq!(entry("a.jar").unwrap(), data.join("mods/a.jar"));
        assert_eq!(
            owned_addon(&data, "mods", "mods/a.jar", "a.jar").unwrap(),
            data.join("mods/a.jar")
        );
        assert_eq!(
            owned_addon_entry(&data, "datapacks", "world/datapacks/p.zip", "p.zip").unwrap(),
            data.join("world/datapacks/p.zip")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            write(&root.path().join("kept/linked.jar"), b"kept");
            symlink(
                root.path().join("kept/linked.jar"),
                shared.join("linked.jar"),
            )
            .unwrap();
            symlink(root.path().join("gone.jar"), shared.join("dangling.jar")).unwrap();
            assert_eq!(entry("linked.jar").unwrap(), data.join("mods/linked.jar"));
            let replace = owned_addon(&data, "mods", "mods/linked.jar", "linked.jar")
                .unwrap_err()
                .to_string();
            assert!(
                replace.contains("link to a file stored elsewhere"),
                "{replace}"
            );
            let dangling = entry("dangling.jar").unwrap_err().to_string();
            assert!(dangling.contains("cannot be found"), "{dangling}");
            assert!(dangling.contains("remove the link"), "{dangling}");
        }
        // Two copies: where they are, and that Open Folder opens the server
        // folder above it.
        let twins = entry("b.jar").unwrap_err().to_string();
        assert!(twins.contains("Both b.jar and b.jar.disabled"), "{twins}");
        assert!(
            twins.contains(&data.join("mods").display().to_string()),
            "{twins}"
        );
        assert!(twins.contains("Delete the copy you do not want"), "{twins}");
        assert!(
            twins.contains("opens the server folder; the copies are in its mods folder"),
            "{twins}"
        );
        let datapack_twins =
            owned_addon_entry(&data, "datapacks", "world/datapacks/q.zip", "q.zip")
                .unwrap_err()
                .to_string();
        assert!(
            datapack_twins.contains("the copies are in its world/datapacks folder"),
            "{datapack_twins}"
        );
        let folder = entry("folder.jar").unwrap_err().to_string();
        assert!(folder.contains("not an addon file"), "{folder}");
        // The world is on a drive that is not plugged in.
        std::fs::rename(root.path().join("worlds"), root.path().join("unplugged")).unwrap();
        let unplugged = owned_addon_entry(&data, "datapacks", "world/datapacks/p.zip", "p.zip")
            .unwrap_err()
            .to_string();
        assert!(
            unplugged.contains(&data.join("world").display().to_string()),
            "{unplugged}"
        );
        assert!(unplugged.contains("Reconnect the drive"), "{unplugged}");
        assert_eq!(
            unavailable_link(&data.join("world").join("datapacks")),
            Some(data.join("world"))
        );
        assert_eq!(unavailable_link(&data.join("mods").join("gone.jar")), None);
        assert_eq!(unavailable_link(&data.join("missing").join("x")), None);
    }

    /// One resolver serves a whole extraction. On a case-sensitive volume a
    /// pack that spells one folder two ways keeps each entry in the folder
    /// its archive names, however the entries are ordered and whichever
    /// spelling the server folder already had.
    #[test]
    fn case_distinct_folders_stay_apart_through_an_extraction() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        std::fs::create_dir(data.join("Probe")).unwrap();
        let insensitive = data.join("probe").exists();
        std::fs::remove_dir(data.join("Probe")).unwrap();
        if insensitive {
            return; // Only a case-sensitive volume has two such folders.
        }
        let mut destinations = Destinations::new(data, filesystem_resolves);
        for (raw, expected) in [
            ("Config/x.toml", "Config/x.toml"),
            ("Config/y.toml", "Config/y.toml"),
            ("config/z.toml", "config/z.toml"),
            ("config/w.toml", "config/w.toml"),
            ("Config/v.toml", "Config/v.toml"),
        ] {
            let (relative, path) = destinations.effective(raw).unwrap();
            assert_eq!(relative, expected, "{raw}");
            write(&path, raw.as_bytes());
        }
        // The user's `kubejs/` is there; the pack spells it `KubeJS/`.
        write(&data.join("kubejs/own.js"), b"own");
        let mut destinations = Destinations::new(data, filesystem_resolves);
        for raw in ["KubeJS/a.js", "KubeJS/b.js", "kubejs/c.js"] {
            let (relative, path) = destinations.effective(raw).unwrap();
            assert_eq!(relative, raw);
            write(&path, raw.as_bytes());
        }
        assert_eq!(list(&data.join("KubeJS")).unwrap().len(), 2);
        assert_eq!(list(&data.join("kubejs")).unwrap().len(), 2);
    }
}
