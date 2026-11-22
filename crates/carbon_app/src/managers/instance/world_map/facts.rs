//! What the map of a world is drawn from: its dimensions and the regions each holds, the
//! spawn, the player's last position, and why a world cannot be mapped. Read from
//! `level.dat` and folder listings; no region file is opened.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::bail;
use sha2::{Digest, Sha256};

use super::super::{
    InstanceId, InstanceManager, installer::publication, mods::validate_addon_basename,
    world_level::inflate_level_dat, world_refusal::WorldRefusal,
};
use crate::domain::instance::AddonType;
use crate::managers::ManagerRef;
use crate::util::nbt::{self, Keep, Limits, Tag};

const OVERWORLD: &str = "minecraft:overworld";
const NETHER: &str = "minecraft:the_nether";
const END: &str = "minecraft:the_end";

/// The vanilla dimensions: their id, their folder under `dimensions/minecraft` (the newest
/// games keep all three there) and their region folder in the older layout.
const VANILLA: [(&str, &str, &[&str]); 3] = [
    (OVERWORLD, "overworld", &["region"]),
    (NETHER, "the_nether", &["DIM-1", "region"]),
    (END, "the_end", &["DIM1", "region"]),
];

/// A region file smaller than its header holds no chunk.
const MIN_REGION_SIZE: u64 = 8192;

/// How deep a dimension's path below its namespace is looked for.
const MAX_PATH_SEGMENTS: usize = 8;

/// Folders the game keeps inside a dimension's folder, never dimensions of their own.
/// `entities` and `poi` hold as many files as `region`, so they are not walked.
const DIMENSION_PARTS: [&str; 3] = ["region", "entities", "poi"];

/// How many folders under `dimensions/` are walked at most; what was found by then is listed.
const MAX_WALKED_FOLDERS: usize = 4096;

/// `Data.version` of the first worlds kept in Anvil regions (1.2); McRegion is 19132.
const ANVIL_VERSION: i32 = 19133;

/// Walking a `level.dat` or a player file costs at most this, inventories included.
const LIMITS: Limits = Limits {
    max_tags: 2_000_000,
    max_depth: 64,
};

#[derive(Debug, Clone, PartialEq)]
pub struct WorldMap {
    pub dimensions: Vec<MapDimension>,
    pub spawn: Option<MapPoint>,
    pub player: Option<MapPoint>,
    pub unmappable: Option<Unmappable>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapDimension {
    pub id: String,
    pub ceiling: bool,
    pub revision: String,
    pub regions: Vec<MapRegion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapRegion {
    pub x: i32,
    pub z: i32,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapPoint {
    pub dimension: String,
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unmappable {
    /// McRegion files, or a `level.dat` from before Anvil and no region folder (the folders
    /// per chunk of the first versions).
    TooOld,
    /// Cubic Chunks' files, which are not regions of 32 by 32 chunks.
    NotRegions,
    /// `level.dat` cannot be read and no region is listed.
    Unreadable,
}

impl ManagerRef<'_, InstanceManager> {
    /// The folder of the save `world`, which must be a plain entry of `saves`: a link out of
    /// it is refused as a world that is gone. No gate: the map may be looked at while the
    /// game runs.
    pub(super) async fn map_world_folder(
        self,
        id: InstanceId,
        world: &str,
    ) -> anyhow::Result<PathBuf> {
        validate_addon_basename(world)?;
        let path = publication::current_path(self.app, id).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let folder = saves.join(world);
        let is_folder = match tokio::fs::symlink_metadata(&folder).await {
            Ok(metadata) => metadata.is_dir(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if !is_folder {
            return Err(WorldRefusal::WorldGone.into());
        }
        Ok(folder)
    }

    pub async fn world_map(self, id: InstanceId, world: String) -> anyhow::Result<WorldMap> {
        let folder = self.map_world_folder(id, &world).await?;
        // A world explored far holds thousands of regions to list.
        Ok(tokio::task::spawn_blocking(move || facts_of(&folder)).await?)
    }
}

fn facts_of(world: &Path) -> WorldMap {
    let dimensions: Vec<MapDimension> = listed_folders(world)
        .into_iter()
        .filter_map(|(id, folder)| dimension(id, &folder))
        .collect();
    let level = read_level(world);
    let (spawn, player) = match &level {
        Ok(level) => (
            level.spawn.clone(),
            match &level.player {
                Some(player) => player.clone(),
                None => lone_player(world),
            },
        ),
        Err(_) => (None, None),
    };
    let listed = |point: Option<MapPoint>| {
        point.filter(|point| dimensions.iter().any(|d| d.id == point.dimension))
    };
    // With regions listed the map is shown, even without a readable `level.dat`.
    let unmappable = if dimensions.is_empty() {
        why_no_regions(world, level.as_ref().ok())
    } else {
        None
    };
    WorldMap {
        spawn: listed(spawn),
        player: listed(player),
        unmappable,
        dimensions,
    }
}

/// The region folder of `dimension`, when the world lists it: the folder its regions were
/// listed from. Looked up in the listing itself, so an id is never joined onto a path and
/// the two cannot disagree (a case-insensitive file system answering `dim7` for `DIM7`).
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn dimension_folder(world: &Path, dimension: &str) -> Option<PathBuf> {
    listed_folders(world)
        .into_iter()
        .find(|(id, _)| id == dimension)
        .map(|(_, folder)| folder)
        .filter(|folder| has_region(folder))
}

/// Each dimension the world may hold, with the region folder it is read from: vanilla ones
/// first, then the rest by id.
fn listed_folders(world: &Path) -> Vec<(String, PathBuf)> {
    let mut others = Vec::new();
    for name in subfolders(world) {
        if numbered(&name).is_some() {
            if let Some(folder) = folder_within(world, &[&name, "region"]) {
                others.push((name, folder));
            }
        }
    }
    if let Some(root) = folder_within(world, &["dimensions"]) {
        let mut budget = MAX_WALKED_FOLDERS;
        for namespace in subfolders(&root) {
            if !segment(&namespace) {
                continue;
            }
            if budget == 0 {
                break;
            }
            budget -= 1;
            walk_dimensions(
                &root.join(&namespace),
                &namespace,
                &mut Vec::new(),
                &mut budget,
                &mut others,
            );
        }
    }
    // Numbered folders by their number (`DIM7` before `DIM10`), then the ids.
    others.sort_by_cached_key(|(id, _)| (numbered(id).is_none(), numbered(id), id.clone()));
    VANILLA
        .iter()
        .filter_map(|(id, name, old)| Some((id.to_string(), vanilla_folder(world, name, old)?)))
        .chain(others)
        .collect()
}

/// Adds every dimension of `namespace` below `folder`, `path` being the segments walked,
/// while `budget` folders may still be walked.
fn walk_dimensions(
    folder: &Path,
    namespace: &str,
    path: &mut Vec<String>,
    budget: &mut usize,
    out: &mut Vec<(String, PathBuf)>,
) {
    if path.len() == MAX_PATH_SEGMENTS {
        return;
    }
    for name in subfolders(folder) {
        if !segment(&name) || DIMENSION_PARTS.contains(&name.as_str()) {
            continue;
        }
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let here = folder.join(&name);
        path.push(name);
        let id = format!("{namespace}:{}", path.join("/"));
        if !VANILLA.iter().any(|(vanilla, _, _)| *vanilla == id) {
            if let Some(region) = folder_within(&here, &["region"]) {
                out.push((id, region));
            }
        }
        walk_dimensions(&here, namespace, path, budget, out);
        path.pop();
    }
}

/// The newer place of a vanilla dimension when it holds regions, else the older one.
fn vanilla_folder(world: &Path, name: &str, old: &[&str]) -> Option<PathBuf> {
    [
        folder_within(world, &["dimensions", "minecraft", name, "region"]),
        folder_within(world, old),
    ]
    .into_iter()
    .flatten()
    .find(|folder| has_region(folder))
}

/// The number of an old mod dimension's folder, `DIM<n>`; the nether's and the end's are
/// vanilla.
fn numbered(name: &str) -> Option<i32> {
    let number: i32 = name.strip_prefix("DIM")?.parse().ok()?;
    (format!("DIM{number}") == name && !matches!(number, -1 | 1)).then_some(number)
}

/// A part of a dimension id the game could have written, which cannot leave its folder.
fn segment(part: &str) -> bool {
    !part.is_empty()
        && part != "."
        && part != ".."
        && part
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.'))
}

/// `world` joined with `parts` when every step is a folder and none is a link.
fn folder_within(world: &Path, parts: &[&str]) -> Option<PathBuf> {
    let mut folder = world.to_path_buf();
    for part in parts {
        folder.push(part);
        if !fs::symlink_metadata(&folder).is_ok_and(|m| m.is_dir()) {
            return None;
        }
    }
    Some(folder)
}

/// The names of the folders in `folder`, links left out.
fn subfolders(folder: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// What a region's revision is computed from: a file's size and modified time.
#[derive(Clone, Copy)]
pub(super) struct Stamp {
    size: u64,
    modified: u128,
}

/// The stamp of the file at `path`, `None` when it is not a regular file (a link is not).
/// Asked of the file system each time: on Windows the metadata of a directory listing
/// lags while the game holds the file open.
pub(super) fn stamp(path: &Path) -> Option<Stamp> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    Some(Stamp {
        size: metadata.len(),
        modified,
    })
}

/// The files named `r.<x>.<z>.mca` in `folder` that are files, not links, of any size.
fn region_files(folder: &Path) -> HashMap<(i32, i32), Stamp> {
    region_entries(folder).collect()
}

fn region_entries(folder: &Path) -> impl Iterator<Item = ((i32, i32), Stamp)> {
    fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let at = region_name(entry.file_name().to_str()?)?;
            // The entry's own type, not a link's target.
            if !entry.file_type().ok()?.is_file() {
                return None;
            }
            Some((at, stamp(&entry.path())?))
        })
}

/// The place of a region file by its name, only as the game writes it (`r.5.0.mca`, never
/// `r.+5.0.mca` or `r.05.0.mca`).
fn region_name(name: &str) -> Option<(i32, i32)> {
    let (x, z) = name
        .strip_prefix("r.")?
        .strip_suffix(".mca")?
        .split_once('.')?;
    let (x, z) = (x.parse().ok()?, z.parse().ok()?);
    (format!("r.{x}.{z}.mca") == name).then_some((x, z))
}

/// Whether `folder` holds a region with chunks in it, as the listing counts them.
fn has_region(folder: &Path) -> bool {
    region_entries(folder).any(|(_, stamp)| stamp.size >= MIN_REGION_SIZE)
}

fn dimension(id: String, folder: &Path) -> Option<MapDimension> {
    let files = region_files(folder);
    let mut regions: Vec<MapRegion> = files
        .iter()
        .filter(|(_, stamp)| stamp.size >= MIN_REGION_SIZE)
        .map(|(&(x, z), stamp)| {
            // A region's first row is shaded against the last row of the one to its north.
            let north = z.checked_sub(1).and_then(|north| files.get(&(x, north)));
            MapRegion {
                x,
                z,
                revision: region_revision(stamp, north),
            }
        })
        .collect();
    if regions.is_empty() {
        return None;
    }
    regions.sort_by_key(|region| (region.z, region.x));
    let mut digest = Sha256::new();
    for region in &regions {
        digest.update(format!("{},{}:{};", region.x, region.z, region.revision));
    }
    Some(MapDimension {
        ceiling: id == NETHER,
        id,
        revision: short_hex(digest),
        regions,
    })
}

/// The revision of a region from its own stamp and that of `r.<x>.<z-1>.mca`, the region to
/// its north; a north file too small to hold a chunk counts as absent.
pub(super) fn region_revision(own: &Stamp, north: Option<&Stamp>) -> String {
    let mut digest = Sha256::new();
    digest.update(format!("{}:{}|", own.size, own.modified));
    match north.filter(|north| north.size >= MIN_REGION_SIZE) {
        Some(north) => digest.update(format!("{}:{}", north.size, north.modified)),
        None => digest.update("-"),
    }
    short_hex(digest)
}

fn short_hex(digest: Sha256) -> String {
    hex::encode(&digest.finalize()[..8])
}

/// When the world lists no dimension, why it cannot be mapped; `None` for a world that has
/// no region yet. `level` is `None` when `level.dat` cannot be read.
fn why_no_regions(world: &Path, level: Option<&Level>) -> Option<Unmappable> {
    let holds = |folder: Option<PathBuf>, extensions: &[&str]| {
        folder.is_some_and(|folder| {
            fs::read_dir(folder).is_ok_and(|entries| {
                entries.flatten().any(|entry| {
                    Path::new(&entry.file_name())
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| extensions.contains(&e))
                })
            })
        })
    };
    let cubic = ["3dr", "2dr"];
    let region = folder_within(world, &["region"]);
    if holds(region.clone(), &["mcr"]) {
        return Some(Unmappable::TooOld);
    }
    if holds(region.clone(), &cubic)
        || holds(folder_within(world, &["region3d"]), &cubic)
        || holds(folder_within(world, &["region2d"]), &cubic)
    {
        return Some(Unmappable::NotRegions);
    }
    let Some(level) = level else {
        return Some(Unmappable::Unreadable);
    };
    // The first versions kept a folder per chunk column and no region folder at all; a world
    // of a later version without one crashed before its first save.
    let before_anvil = level.version.is_none_or(|version| version < ANVIL_VERSION);
    let any_region_folder = region.is_some()
        || folder_within(world, &["DIM-1", "region"]).is_some()
        || folder_within(world, &["DIM1", "region"]).is_some()
        || folder_within(world, &["dimensions"]).is_some();
    (before_anvil && !any_region_folder).then_some(Unmappable::TooOld)
}

struct Level {
    /// `Data.version`, the format of the world's files; absent before McRegion.
    version: Option<i32>,
    spawn: Option<MapPoint>,
    /// `None` when `level.dat` holds no player (a server's world).
    player: Option<Option<MapPoint>>,
}

fn read_level(world: &Path) -> anyhow::Result<Level> {
    let path = world.join("level.dat");
    if !fs::symlink_metadata(&path)?.is_file() {
        bail!("{} is not a file", path.display());
    }
    let root = nbt::read_keeping(&inflate_level_dat(&path)?, &keep_level, LIMITS)?;
    let Some(data @ Tag::Compound(_)) = root.tag.get("Data") else {
        bail!("{} has no Data compound", path.display());
    };
    Ok(Level {
        version: match data.get("version") {
            Some(Tag::Int(version)) => Some(*version),
            _ => None,
        },
        spawn: spawn_of(data),
        player: data.get("Player").map(player_point),
    })
}

fn keep_level(path: &[&[u8]]) -> Keep {
    match path {
        [b"Data"] | [b"Data", b"Player"] => Keep::Descend,
        [b"Data", b"version" | b"SpawnX" | b"SpawnZ" | b"spawn"]
        | [b"Data", b"Player", b"Pos" | b"Dimension"] => Keep::All,
        _ => Keep::Skip,
    }
}

/// The newer record, which names its dimension, wins over the three numbers in the
/// overworld.
fn spawn_of(data: &Tag) -> Option<MapPoint> {
    let record = data.get("spawn").and_then(|spawn| {
        let Some(Tag::IntArray(pos)) = spawn.get("pos") else {
            return None;
        };
        let [x, _, z] = pos.as_slice() else {
            return None;
        };
        let dimension = match spawn.get("dimension") {
            None => OVERWORLD.to_string(),
            Some(Tag::String(raw)) => nbt::decode_string(raw)?,
            Some(_) => return None,
        };
        Some(MapPoint {
            dimension,
            x: *x,
            z: *z,
        })
    });
    record.or_else(|| match (data.get("SpawnX"), data.get("SpawnZ")) {
        (Some(Tag::Int(x)), Some(Tag::Int(z))) => Some(MapPoint {
            dimension: OVERWORLD.to_string(),
            x: *x,
            z: *z,
        }),
        _ => None,
    })
}

/// Where a player record says the player is: `Pos` and `Dimension`, an id since 1.16 and a
/// number before.
fn player_point(player: &Tag) -> Option<MapPoint> {
    let Some(Tag::List { items, .. }) = player.get("Pos") else {
        return None;
    };
    let [Tag::Double(x), _, Tag::Double(z)] = items.as_slice() else {
        return None;
    };
    let dimension = match player.get("Dimension") {
        None => OVERWORLD.to_string(),
        Some(Tag::String(raw)) => nbt::decode_string(raw)?,
        Some(Tag::Int(0)) => OVERWORLD.to_string(),
        Some(Tag::Int(-1)) => NETHER.to_string(),
        Some(Tag::Int(1)) => END.to_string(),
        Some(Tag::Int(number)) => format!("DIM{number}"),
        Some(_) => return None,
    };
    Some(MapPoint {
        dimension,
        x: block(*x)?,
        z: block(*z)?,
    })
}

fn block(coordinate: f64) -> Option<i32> {
    let block = coordinate.floor();
    (block >= i32::MIN as f64 && block <= i32::MAX as f64).then_some(block as i32)
}

/// The player of a world whose `level.dat` holds none, when the world holds exactly one
/// player file; a file that cannot be read gives no player.
fn lone_player(world: &Path) -> Option<MapPoint> {
    let files = [&["players", "data"][..], &["playerdata"][..]]
        .into_iter()
        .filter_map(|parts| folder_within(world, parts))
        .map(|folder| player_files(&folder))
        .find(|files| !files.is_empty())?;
    let [file] = files.as_slice() else {
        return None;
    };
    let root = nbt::read_keeping(&inflate_level_dat(file).ok()?, &keep_player, LIMITS).ok()?;
    player_point(&root.tag)
}

/// Up to two `*.dat` files of `folder`, links left out: enough to tell one from several.
fn player_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "dat"))
        .take(2)
        .collect()
}

fn keep_player(path: &[&[u8]]) -> Keep {
    match path {
        [b"Pos" | b"Dimension"] => Keep::All,
        _ => Keep::Skip,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime};

    use super::super::super::{mods::tests::addon_fixture, world_level};
    use super::super::test_nbt::{compound, compounds, string};
    use super::*;
    use crate::domain::instance::AddonType;
    use crate::util::nbt::{Root, Tag};

    fn world_with(data: Vec<(&str, Tag)>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("level.dat"),
            world_level::level_dat_holding(data),
        )
        .unwrap();
        dir
    }

    fn plain_world() -> tempfile::TempDir {
        world_with(vec![("LevelName", string("Plain"))])
    }

    fn put_file(path: &Path, size: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![1u8; size]).unwrap();
    }

    /// A region file of `folder` big enough to hold its header.
    fn put_region(world: &Path, folder: &str, x: i32, z: i32) -> PathBuf {
        let path = world.join(folder).join(format!("r.{x}.{z}.mca"));
        put_file(&path, 8192);
        path
    }

    fn ids(map: &WorldMap) -> Vec<&str> {
        map.dimensions.iter().map(|d| d.id.as_str()).collect()
    }

    fn dimension<'a>(map: &'a WorldMap, id: &str) -> &'a MapDimension {
        map.dimensions.iter().find(|d| d.id == id).unwrap()
    }

    fn regions(map: &WorldMap, id: &str) -> Vec<(i32, i32)> {
        dimension(map, id)
            .regions
            .iter()
            .map(|r| (r.x, r.z))
            .collect()
    }

    fn revision(map: &WorldMap, id: &str, x: i32, z: i32) -> String {
        dimension(map, id)
            .regions
            .iter()
            .find(|r| (r.x, r.z) == (x, z))
            .unwrap()
            .revision
            .clone()
    }

    fn point(dimension: &str, x: i32, z: i32) -> Option<MapPoint> {
        Some(MapPoint {
            dimension: dimension.into(),
            x,
            z,
        })
    }

    #[test]
    fn the_three_vanilla_dimensions_and_their_regions_are_listed() {
        let old = plain_world();
        for (x, z) in [(1, 0), (0, 1), (0, 0), (-1, -1)] {
            put_region(old.path(), "region", x, z);
        }
        put_region(old.path(), "DIM-1/region", 0, 0);
        put_region(old.path(), "DIM1/region", 2, 3);
        put_file(&old.path().join("region/r.0.0.mca.tmp"), 8192);
        let map = facts_of(old.path());
        assert_eq!(ids(&map), [OVERWORLD, NETHER, END]);
        let ceilings: Vec<bool> = map.dimensions.iter().map(|d| d.ceiling).collect();
        assert_eq!(ceilings, [false, true, false]);
        assert_eq!(regions(&map, OVERWORLD), [(-1, -1), (0, 0), (1, 0), (0, 1)]);
        assert_eq!(regions(&map, END), [(2, 3)]);
        assert_eq!(map.unmappable, None);

        // The newest games keep the vanilla dimensions under `dimensions/` too.
        let new = plain_world();
        put_region(new.path(), "dimensions/minecraft/overworld/region", 4, 4);
        put_region(new.path(), "dimensions/minecraft/the_nether/region", 0, 0);
        put_region(new.path(), "dimensions/minecraft/the_end/region", 0, 0);
        let map = facts_of(new.path());
        assert_eq!(ids(&map), [OVERWORLD, NETHER, END]);
        assert!(dimension(&map, NETHER).ceiling);
        assert_eq!(regions(&map, OVERWORLD), [(4, 4)]);

        // Found at both places, a dimension is listed once, from the new place when it
        // holds regions; the folder answered for it is the one listed.
        let both = plain_world();
        put_region(both.path(), "region", 0, 0);
        put_region(both.path(), "dimensions/minecraft/overworld/region", 7, 7);
        put_region(both.path(), "DIM-1/region", 1, 1);
        fs::create_dir_all(both.path().join("dimensions/minecraft/the_nether/region")).unwrap();
        let map = facts_of(both.path());
        assert_eq!(ids(&map), [OVERWORLD, NETHER]);
        assert_eq!(regions(&map, OVERWORLD), [(7, 7)]);
        assert_eq!(regions(&map, NETHER), [(1, 1)]);
        assert_eq!(
            dimension_folder(both.path(), OVERWORLD),
            Some(both.path().join("dimensions/minecraft/overworld/region"))
        );
        assert_eq!(
            dimension_folder(both.path(), NETHER),
            Some(both.path().join("DIM-1/region"))
        );
        assert_eq!(dimension_folder(both.path(), END), None);
    }

    #[test]
    fn datapack_and_old_mod_dimensions_are_listed_by_their_id_or_folder() {
        let world = plain_world();
        put_region(world.path(), "region", 0, 0);
        put_region(world.path(), "dimensions/aether/the_aether/region", 1, 2);
        put_region(world.path(), "dimensions/a/b/c/region", 0, 0);
        put_region(world.path(), "DIM7/region", -3, 0);
        let map = facts_of(world.path());
        assert_eq!(ids(&map), [OVERWORLD, "DIM7", "a:b/c", "aether:the_aether"]);
        assert!(map.dimensions.iter().all(|d| !d.ceiling));
        assert_eq!(regions(&map, "aether:the_aether"), [(1, 2)]);
        assert_eq!(regions(&map, "DIM7"), [(-3, 0)]);
        for (id, folder) in [
            ("aether:the_aether", "dimensions/aether/the_aether/region"),
            ("a:b/c", "dimensions/a/b/c/region"),
            ("DIM7", "DIM7/region"),
            (OVERWORLD, "region"),
        ] {
            assert_eq!(
                dimension_folder(world.path(), id),
                Some(world.path().join(folder)),
                "{id}"
            );
        }
    }

    #[test]
    fn a_dimension_without_real_regions_is_not_listed() {
        let world = plain_world();
        put_region(world.path(), "region", 0, 0);
        fs::create_dir_all(world.path().join("DIM-1/region")).unwrap();
        put_file(&world.path().join("DIM1/region/r.0.0.mca"), 100);
        put_file(&world.path().join("DIM6/region/r.x.y.mca"), 8192);
        put_file(&world.path().join("DIM6/region/r.0.0.mcr"), 8192);
        fs::create_dir_all(world.path().join("DIM8/region/r.0.0.mca")).unwrap();
        fs::create_dir_all(world.path().join("dimensions/empty/one/region")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let elsewhere = tempfile::tempdir().unwrap();
            let real = put_region(elsewhere.path(), "region", 0, 0);
            fs::create_dir_all(world.path().join("DIM5/region")).unwrap();
            symlink(&real, world.path().join("DIM5/region/r.0.0.mca")).unwrap();
            fs::create_dir_all(world.path().join("DIM9")).unwrap();
            symlink(
                elsewhere.path().join("region"),
                world.path().join("DIM9/region"),
            )
            .unwrap();
            fs::create_dir_all(world.path().join("dimensions/linked")).unwrap();
            symlink(elsewhere.path(), world.path().join("dimensions/linked/far")).unwrap();
            let map = facts_of(world.path());
            assert_eq!(ids(&map), [OVERWORLD]);
            for id in ["DIM5", "DIM9", "linked:far"] {
                assert_eq!(dimension_folder(world.path(), id), None, "{id}");
            }
        }
        let map = facts_of(world.path());
        assert_eq!(ids(&map), [OVERWORLD]);
        assert_eq!(map.unmappable, None);
        for id in [NETHER, END, "DIM6", "DIM8", "empty:one"] {
            assert_eq!(dimension_folder(world.path(), id), None, "{id}");
        }
    }

    #[test]
    fn a_regions_revision_changes_with_it_and_with_its_north_neighbour() {
        let world = plain_world();
        let own = put_region(world.path(), "region", 0, 0);
        put_region(world.path(), "region", 5, 5);
        let mut seen = vec![facts_of(world.path())];

        // The north neighbour appears.
        let north = put_region(world.path(), "region", 0, -1);
        seen.push(facts_of(world.path()));
        // The region itself is rewritten: same size, a later time.
        let later = SystemTime::now() + Duration::from_secs(60);
        fs::File::options()
            .write(true)
            .open(&own)
            .unwrap()
            .set_modified(later)
            .unwrap();
        seen.push(facts_of(world.path()));
        // The north neighbour grows.
        put_file(&north, 3 * 4096);
        seen.push(facts_of(world.path()));

        for pair in seen.windows(2) {
            assert_ne!(
                revision(&pair[0], OVERWORLD, 0, 0),
                revision(&pair[1], OVERWORLD, 0, 0)
            );
            assert_eq!(
                revision(&pair[0], OVERWORLD, 5, 5),
                revision(&pair[1], OVERWORLD, 5, 5)
            );
            assert_ne!(
                dimension(&pair[0], OVERWORLD).revision,
                dimension(&pair[1], OVERWORLD).revision
            );
        }
        // Unchanged, the same facts come back.
        assert_eq!(facts_of(world.path()), seen[3]);
        let rev = revision(&seen[3], OVERWORLD, 0, 0);
        assert!(
            rev.len() == 16 && rev.chars().all(|c| c.is_ascii_hexdigit()),
            "{rev}"
        );
    }

    fn player(pos: [f64; 3], dimension: Option<Tag>) -> Tag {
        let mut entries = vec![(
            "Pos",
            Tag::List {
                element: 6,
                items: pos.into_iter().map(Tag::Double).collect(),
            },
        )];
        if let Some(dimension) = dimension {
            entries.push(("Dimension", dimension));
        }
        entries.push(("Inventory", compounds(vec![])));
        compound(entries)
    }

    fn new_spawn(dimension: &str, pos: [i32; 3]) -> Tag {
        compound(vec![
            ("pos", Tag::IntArray(pos.to_vec())),
            ("dimension", string(dimension)),
            ("yaw", Tag::Float(0.0)),
            ("pitch", Tag::Float(0.0)),
        ])
    }

    /// A world of `data` holding a region in the overworld and in the nether.
    fn two_dimensions(data: Vec<(&str, Tag)>) -> tempfile::TempDir {
        let world = world_with(data);
        put_region(world.path(), "region", 0, 0);
        put_region(world.path(), "DIM-1/region", 0, 0);
        world
    }

    #[test]
    fn spawn_and_player_in_both_forms() {
        let old = two_dimensions(vec![
            ("SpawnX", Tag::Int(10)),
            ("SpawnY", Tag::Int(64)),
            ("SpawnZ", Tag::Int(-20)),
            ("Player", player([1.5, 70.0, -3.7], Some(Tag::Int(-1)))),
        ]);
        let map = facts_of(old.path());
        assert_eq!(map.spawn, point(OVERWORLD, 10, -20));
        assert_eq!(map.player, point(NETHER, 1, -4));

        let new = two_dimensions(vec![
            ("SpawnX", Tag::Int(5)),
            ("SpawnZ", Tag::Int(5)),
            ("spawn", new_spawn(NETHER, [100, 64, -200])),
            ("Player", player([-0.5, 70.0, 8.0], Some(string(NETHER)))),
        ]);
        let map = facts_of(new.path());
        assert_eq!(map.spawn, point(NETHER, 100, -200));
        assert_eq!(map.player, point(NETHER, -1, 8));

        // Without a dimension the player is in the overworld.
        let no_dimension = two_dimensions(vec![("Player", player([3.0, 0.0, 4.0], None))]);
        let map = facts_of(no_dimension.path());
        assert_eq!(map.spawn, None);
        assert_eq!(map.player, point(OVERWORLD, 3, 4));

        // A point in a dimension the world does not list is dropped.
        for (spawn, dimension) in [(END, Tag::Int(1)), ("aether:the_aether", string(END))] {
            let unlisted = two_dimensions(vec![
                ("spawn", new_spawn(spawn, [0, 0, 0])),
                ("Player", player([0.0, 0.0, 0.0], Some(dimension))),
            ]);
            let map = facts_of(unlisted.path());
            assert_eq!((map.spawn, map.player), (None, None), "{spawn}");
        }
    }

    fn player_file(dimension: &str, pos: [f64; 3]) -> Vec<u8> {
        let Tag::Compound(entries) = player(pos, Some(string(dimension))) else {
            unreachable!()
        };
        world_level::gzipped(&nbt::write(&Root {
            name: vec![],
            tag: Tag::Compound(entries),
        }))
    }

    #[test]
    fn the_player_of_a_world_without_one_in_level_dat_is_its_only_player_file() {
        for folder in ["players/data", "playerdata"] {
            let world = two_dimensions(vec![]);
            let players = world.path().join(folder);
            fs::create_dir_all(&players).unwrap();
            fs::write(
                players.join("069a79f4-44e9-4726-a5be-fca90e38aaf5.dat"),
                player_file(NETHER, [16.0, 64.0, 32.0]),
            )
            .unwrap();
            fs::write(players.join("old.dat_old"), b"previous").unwrap();
            assert_eq!(
                facts_of(world.path()).player,
                point(NETHER, 16, 32),
                "{folder}"
            );

            fs::write(
                players.join("2b5a9e1c-0000-4000-8000-000000000000.dat"),
                player_file(OVERWORLD, [0.0, 0.0, 0.0]),
            )
            .unwrap();
            assert_eq!(facts_of(world.path()).player, None, "{folder}: two players");
        }

        let unreadable = two_dimensions(vec![]);
        put_file(&unreadable.path().join("playerdata/one.dat"), 300);
        let map = facts_of(unreadable.path());
        assert_eq!(map.player, None);
        assert_eq!(map.unmappable, None);
    }

    #[test]
    fn worlds_that_cannot_be_mapped_say_why() {
        let garbled = tempfile::tempdir().unwrap();
        fs::write(garbled.path().join("level.dat"), b"not gzip at all").unwrap();
        put_region(garbled.path(), "region", 0, 0);
        let without_level = tempfile::tempdir().unwrap();
        put_region(without_level.path(), "region", 0, 0);
        // With regions listed the map is shown, without spawn and player.
        for (what, world) in [("garbled", &garbled), ("no level.dat", &without_level)] {
            let map = facts_of(world.path());
            assert_eq!(ids(&map), [OVERWORLD], "{what}");
            assert_eq!(map.unmappable, None, "{what}");
            assert_eq!((map.spawn, map.player), (None, None), "{what}");
        }
        let garbled_empty = tempfile::tempdir().unwrap();
        fs::write(garbled_empty.path().join("level.dat"), b"not gzip at all").unwrap();
        let garbled_new = tempfile::tempdir().unwrap();
        fs::write(garbled_new.path().join("level.dat"), b"not gzip at all").unwrap();
        fs::create_dir_all(garbled_new.path().join("region")).unwrap();
        for (what, world) in [
            ("no folders", &garbled_empty),
            ("empty region", &garbled_new),
        ] {
            let map = facts_of(world.path());
            assert_eq!(map.unmappable, Some(Unmappable::Unreadable), "{what}");
        }

        // Beta 1.3 to 1.1: McRegion files.
        let mcregion = plain_world();
        put_file(&mcregion.path().join("region/r.0.0.mcr"), 8192);
        put_file(&mcregion.path().join("DIM-1/region/r.0.0.mcr"), 8192);
        // Before Beta 1.3: a folder per chunk column and no region folder.
        let alpha = plain_world();
        put_file(&alpha.path().join("0/0/c.0.0.dat"), 300);
        for (what, world) in [("mcregion", &mcregion), ("alpha", &alpha)] {
            let map = facts_of(world.path());
            assert_eq!(ids(&map), Vec::<&str>::new(), "{what}");
            assert_eq!(map.unmappable, Some(Unmappable::TooOld), "{what}");
        }

        // Cubic Chunks.
        let cubic = plain_world();
        put_file(&cubic.path().join("region/0.0.0.3dr"), 8192);
        put_file(&cubic.path().join("region/0.0.2dr"), 8192);
        let cubic_folders = plain_world();
        put_file(&cubic_folders.path().join("region3d/0.0.0.3dr"), 8192);
        put_file(&cubic_folders.path().join("region2d/0.0.2dr"), 8192);
        fs::create_dir_all(cubic_folders.path().join("region")).unwrap();
        for (what, world) in [("cubic", &cubic), ("cubic folders", &cubic_folders)] {
            let map = facts_of(world.path());
            assert_eq!(map.unmappable, Some(Unmappable::NotRegions), "{what}");
        }

        // Brand new worlds hold no region yet: nothing to show, and nothing wrong.
        let new_old_layout = plain_world();
        fs::create_dir_all(new_old_layout.path().join("region")).unwrap();
        let new_new_layout = plain_world();
        fs::create_dir_all(
            new_new_layout
                .path()
                .join("dimensions/minecraft/overworld/region"),
        )
        .unwrap();
        fs::create_dir_all(new_new_layout.path().join("dimensions/minecraft/the_end")).unwrap();
        for (what, world) in [("older", &new_old_layout), ("newest", &new_new_layout)] {
            let map = facts_of(world.path());
            assert_eq!(ids(&map), Vec::<&str>::new(), "{what}");
            assert_eq!(map.unmappable, None, "{what}");
        }
    }

    #[test]
    fn a_world_with_only_a_level_dat_is_too_old_only_when_its_version_says_so() {
        for (version, expected) in [
            (None, Some(Unmappable::TooOld)),
            (Some(19132), Some(Unmappable::TooOld)),
            // Anvil: a new world that has not saved a region yet.
            (Some(19133), None),
        ] {
            let mut data = vec![("LevelName", string("Only level"))];
            if let Some(version) = version {
                data.push(("version", Tag::Int(version)));
            }
            let world = world_with(data);
            let map = facts_of(world.path());
            assert_eq!(ids(&map), Vec::<&str>::new(), "{version:?}");
            assert_eq!(map.unmappable, expected, "{version:?}");
        }
        // Any region folder says the world is not from before regions.
        for folder in ["DIM-1/region", "DIM1/region", "dimensions/minecraft"] {
            let world = plain_world();
            fs::create_dir_all(world.path().join(folder)).unwrap();
            assert_eq!(facts_of(world.path()).unmappable, None, "{folder}");
        }
    }

    #[test]
    fn region_names_the_game_never_writes_are_ignored() {
        let world = plain_world();
        put_region(world.path(), "region", 0, 0);
        for name in ["r.+5.0.mca", "r.05.0.mca", "r.0.-0.mca", "r.0.+1.mca"] {
            put_file(&world.path().join("region").join(name), 8192);
            put_file(&world.path().join("DIM4/region").join(name), 8192);
        }
        let map = facts_of(world.path());
        assert_eq!(ids(&map), [OVERWORLD]);
        assert_eq!(regions(&map, OVERWORLD), [(0, 0)]);
        assert_eq!(dimension_folder(world.path(), "DIM4"), None);
    }

    #[test]
    fn numbered_dimensions_are_ordered_by_their_number() {
        let world = plain_world();
        for folder in ["DIM10", "DIM7", "DIM-3", "DIM2", "dimensions/a/b"] {
            put_region(world.path(), &format!("{folder}/region"), 0, 0);
        }
        let map = facts_of(world.path());
        assert_eq!(ids(&map), ["DIM-3", "DIM2", "DIM7", "DIM10", "a:b"]);
    }

    #[test]
    fn the_walk_under_dimensions_stops_after_so_many_folders() {
        let world = plain_world();
        let total = MAX_WALKED_FOLDERS + 100;
        for i in 0..total {
            let path = world
                .path()
                .join(format!("dimensions/many/d{i:05}/region/r.0.0.mca"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            // Sparse: the size is what counts.
            fs::File::create(&path).unwrap().set_len(8192).unwrap();
        }
        let map = facts_of(world.path());
        // The namespace's folder is one of those walked.
        assert_eq!(map.dimensions.len(), MAX_WALKED_FOLDERS - 1);
        // The folder of a dimension is only answered for one the listing gives.
        let listed: Vec<&str> = ids(&map);
        let first = listed[0];
        assert_eq!(
            dimension_folder(world.path(), first),
            Some(world.path().join(format!(
                "dimensions/many/{}/region",
                first.strip_prefix("many:").unwrap()
            )))
        );
        let unlisted: Vec<String> = (0..total)
            .map(|i| format!("many:d{i:05}"))
            .filter(|id| !listed.contains(&id.as_str()))
            .take(5)
            .collect();
        assert_eq!(unlisted.len(), 5);
        for id in unlisted {
            assert_eq!(dimension_folder(world.path(), &id), None, "{id}");
        }
    }

    #[test]
    fn adding_or_removing_a_region_changes_the_dimensions_revision() {
        let world = plain_world();
        put_region(world.path(), "region", 0, 0);
        let before = facts_of(world.path());
        let added = put_region(world.path(), "region", 9, 9);
        let with = facts_of(world.path());
        fs::remove_file(added).unwrap();
        let after = facts_of(world.path());
        assert_ne!(
            dimension(&before, OVERWORLD).revision,
            dimension(&with, OVERWORLD).revision
        );
        assert_ne!(
            dimension(&with, OVERWORLD).revision,
            dimension(&after, OVERWORLD).revision
        );
    }

    #[test]
    fn a_north_neighbour_without_chunks_counts_as_absent() {
        let world = plain_world();
        put_region(world.path(), "region", 0, 0);
        let alone = revision(&facts_of(world.path()), OVERWORLD, 0, 0);
        put_file(&world.path().join("region/r.0.-1.mca"), 100);
        let small = revision(&facts_of(world.path()), OVERWORLD, 0, 0);
        assert_eq!(alone, small);
        put_file(&world.path().join("region/r.0.-1.mca"), 8192);
        let real = revision(&facts_of(world.path()), OVERWORLD, 0, 0);
        assert_ne!(alone, real);
    }

    async fn refusal_of(app: &crate::managers::App, id: InstanceId, world: &str) -> Option<String> {
        let error = app
            .instance_manager()
            .world_map(id, world.into())
            .await
            .unwrap_err();
        crate::error::coded_cause(&error).map(|(code, _)| code)
    }

    #[tokio::test]
    async fn a_world_that_is_a_link_or_is_gone_is_refused() {
        let app = crate::setup_managers_for_test().await;
        let level =
            world_level::level_dat_holding(vec![("SpawnX", Tag::Int(3)), ("SpawnZ", Tag::Int(-9))]);
        let (id, _, world) =
            addon_fixture(&app, "map-facts", AddonType::Worlds, "Home", &level).await;
        put_region(&world, "region", 0, 0);
        let saves = world.parent().unwrap();

        let map = app
            .instance_manager()
            .world_map(id, "Home".into())
            .await
            .unwrap();
        assert_eq!(ids(&map), [OVERWORLD]);
        assert_eq!(map.spawn, point(OVERWORLD, 3, -9));
        assert_eq!(
            app.instance_manager()
                .map_world_folder(id, "Home")
                .await
                .unwrap(),
            world
        );

        fs::write(saves.join("File"), b"not a world").unwrap();
        let mut gone = vec!["Missing", "File"];
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&world, saves.join("Link")).unwrap();
            gone.push("Link");
        }
        for name in gone {
            assert_eq!(
                refusal_of(&app, id, name).await,
                Some("WORLD_GONE".to_owned()),
                "{name}"
            );
        }
        for name in ["../Home", "Home/region", "", ".."] {
            assert!(
                app.instance_manager()
                    .world_map(id, name.into())
                    .await
                    .is_err(),
                "{name:?}"
            );
        }
    }

    #[test]
    fn dimension_folder_only_knows_listed_ids() {
        let world = plain_world();
        put_region(world.path(), "region", 0, 0);
        put_region(world.path(), "DIM7/region", 0, 0);
        put_region(world.path(), "DIM-1/region", 0, 0);
        put_region(world.path(), "DIM1/region", 0, 0);
        put_region(world.path(), "dimensions/a/c/region", 0, 0);
        put_region(world.path(), "x/region", 0, 0);
        assert_eq!(
            dimension_folder(world.path(), "DIM7"),
            Some(world.path().join("DIM7/region"))
        );
        for id in [
            "../x",
            "x",
            "DIM7/../..",
            "DIM07",
            "DIM+7",
            "DIM-1",
            "DIM1",
            "minecraft:overworld/..",
            "minecraft:../../x",
            "a:b",
            "a:c/..",
            "a:",
            ":c",
            "a:/c",
            "a:c/",
            "A:c",
            "",
        ] {
            assert_eq!(dimension_folder(world.path(), id), None, "{id:?}");
        }
    }
}
