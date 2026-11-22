//! Serves the pieces of a world's map: one region at one pixel per block (scale 0), or four
//! pieces of the scale below joined at half size (scales 1 to 4). A piece kept for the
//! current files is answered from the store; any other is painted, a few at once, and kept
//! when every chunk under it could be read.

use std::collections::HashMap;
use std::fs;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::anyhow;
use futures::future::BoxFuture;
use image::{ImageFormat, RgbaImage};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tokio::task::spawn_blocking;
use tracing::debug;

use super::super::world_refusal::WorldRefusal;
use super::super::{InstanceId, InstanceManager};
#[cfg(test)]
pub(crate) use super::budget::PAINTING_AT_ONCE;
use super::budget::PaintingBudget;
use super::chunk::Roof;
use super::facts::{RegionAt, dimension_folder, has_ceiling, region_at, region_file, short_hex};
use super::legacy::LegacyCache;
use super::paint::{PIECE, Painted, fold, paint_region, png};
use super::region::{MAX_REGION_BYTES, MAX_ROW_BYTES, Region};
use super::store::{PieceKey, Store};
use crate::managers::ManagerRef;

/// The largest scale: a piece of it covers 16 by 16 regions.
pub(crate) const MAX_SCALE: u8 = 4;

/// How long a request waits for the files of its world while an operation moves or removes
/// them; past it, it answers nothing.
const FILES_WAIT: Duration = Duration::from_millis(500);

/// A piece asked for while the files of its world were closed to the map: answered as nothing,
/// and nothing of it is kept.
#[derive(Debug)]
struct FilesClosed;

impl std::fmt::Display for FilesClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the files of the world are being moved or removed")
    }
}

impl std::error::Error for FilesClosed {}

/// How many requests for pieces of scale 1 and up are built at once: each holds a picture per
/// scale it descends while the regions beneath are painted.
pub(crate) const BUILDING_AT_ONCE: usize = 4;

/// One piece of a world's map, as the desktop asks for it.
#[derive(Debug, Clone)]
pub struct PieceRequest {
    pub instance_id: i32,
    /// The world's folder in `saves`.
    pub world: String,
    pub dimension: String,
    pub scale: u8,
    pub x: i32,
    pub z: i32,
    /// Only the address the desktop caches the piece under: the answer is always for the
    /// world's current files.
    pub rev: String,
}

/// A piece's PNG, and whether every chunk under it could be read: one that is not is painted
/// again when asked again, so the desktop must not keep it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub png: Vec<u8>,
    pub complete: bool,
}

impl ManagerRef<'_, InstanceManager> {
    /// The piece; `None` when nothing of the world lies under it, the world included, or
    /// while the world's files are being moved or removed. No instance gate is taken: the map
    /// is drawn while the game runs, and a world renamed or deleted meanwhile only makes the
    /// answer `None` or a piece that is not kept.
    pub async fn world_map_piece(self, request: PieceRequest) -> anyhow::Result<Option<Piece>> {
        match self.world_map_piece_open(request).await {
            Err(error) if error.is::<FilesClosed>() => Ok(None),
            answer => answer,
        }
    }

    async fn world_map_piece_open(self, request: PieceRequest) -> anyhow::Result<Option<Piece>> {
        if request.scale > MAX_SCALE || !is_revision(&request.rev) {
            return Ok(None);
        }
        let instance = InstanceId(request.instance_id);
        let files = self.world_map_files(instance);
        // Held while the world is found and its regions listed; nothing else is waited for.
        let open = tokio::time::timeout(FILES_WAIT, files.clone().read_owned())
            .await
            .map_err(|_| FilesClosed)?;
        let world = match self.map_world_folder(instance, &request.world).await {
            Ok(world) => world,
            // Renamed, deleted or replaced by a link since the desktop listed it.
            Err(error) if matches!(error.downcast_ref(), Some(WorldRefusal::WorldGone)) => {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let (scale, x, z) = (request.scale, request.x, request.z);
        let found = {
            let (world, dimension) = (world.clone(), request.dimension.clone());
            spawn_blocking(move || {
                // Lists the whole world, so asked once per request.
                let folder = dimension_folder(&world, &dimension)?;
                let regions = regions_beneath(&folder, scale, x, z)?;
                Some((folder, regions))
            })
            .await?
        };
        drop(open);
        let Some((folder, regions)) = found else {
            return Ok(None);
        };
        let given_up = Arc::new(AtomicBool::new(false));
        let _give_up = GiveUpOnDrop(given_up.clone());
        let painter = Arc::new(Painter {
            store: self.world_map_store(),
            painting: self.world_map_painting.clone(),
            legacy: self.world_map_legacy.clone(),
            files,
            instance,
            roof: if has_ceiling(&request.dimension) {
                Roof::Ceiling
            } else {
                Roof::Open
            },
            world_name: request.world,
            dimension: request.dimension,
            world,
            folder,
            regions,
            given_up,
            #[cfg(test)]
            pause: self.world_map_pause.get(),
            #[cfg(test)]
            read_pause: self.world_map_read_pause.get(),
        });
        let Some(revision) = painter.revision(scale, x, z) else {
            return Ok(None);
        };
        if let Some(png) = painter.kept(scale, x, z, &revision).await? {
            return Ok(Some(Piece {
                png,
                complete: true,
            }));
        }
        // Taken here, never under a painting permit, so it cannot wait on one it holds.
        let _building = match scale {
            0 => None,
            _ => Some(
                self.world_map_building
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("the map building semaphore is never closed"),
            ),
        };
        #[cfg(test)]
        let _counted = (scale > 0).then(|| self.world_map_builds.enter());
        // Built from the store when another request built it while this one waited.
        Ok(match painter.piece(scale, x, z).await? {
            None => None,
            Some(Drawn::Kept(png)) => Some(Piece {
                png,
                complete: true,
            }),
            Some(Drawn::Painted {
                png: Some(png),
                complete,
                ..
            }) => Some(Piece { png, complete }),
            Some(Drawn::Painted {
                image, complete, ..
            }) => Some(Piece {
                png: spawn_blocking(move || png(&image)).await??,
                complete,
            }),
        })
    }
}

/// Whether `rev` can be an address: the desktop only caches pieces under it and the core
/// computes the revision it answers for, but a value from outside is bounded all the same.
fn is_revision(rev: &str) -> bool {
    (1..=64).contains(&rev.len())
        && rev
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// The regions under the piece `(x, z)` of `scale`, by x and by z; `None` beyond the
/// coordinates a region can have.
fn regions_under(scale: u8, x: i32, z: i32) -> Option<(RangeInclusive<i32>, RangeInclusive<i32>)> {
    let side = 1i64 << scale;
    let range = |at: i32| {
        let first = i32::try_from(i64::from(at) * side).ok()?;
        let last = i32::try_from((i64::from(at) + 1) * side - 1).ok()?;
        Some(first..=last)
    };
    Some((range(x)?, range(z)?))
}

/// Every region under the piece that exists, asked of each file by its name.
fn regions_beneath(
    folder: &Path,
    scale: u8,
    x: i32,
    z: i32,
) -> Option<HashMap<(i32, i32), RegionAt>> {
    let (xs, zs) = regions_under(scale, x, z)?;
    let mut regions = HashMap::new();
    for z in zs {
        for x in xs.clone() {
            if let Some(region) = region_at(folder, x, z) {
                regions.insert((x, z), region);
            }
        }
    }
    Some(regions)
}

/// The bytes of files a painting of `region` reads: the region whole, the last row of the
/// one to its north.
fn bytes_read(region: &RegionAt) -> u64 {
    region.size.min(MAX_REGION_BYTES)
        + region
            .north_size
            .map_or(0, |north| north.min(MAX_ROW_BYTES))
}

/// Set when the request is dropped: the desktop gave the piece up.
struct GiveUpOnDrop(Arc<AtomicBool>);

impl Drop for GiveUpOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

enum Drawn {
    /// As the store kept it.
    Kept(Vec<u8>),
    /// Painted now; `png` when it was kept.
    Painted {
        image: RgbaImage,
        png: Option<Vec<u8>>,
        complete: bool,
    },
}

/// Paints the pieces of one request, all in one dimension of one world.
struct Painter {
    store: Arc<Store>,
    painting: Arc<PaintingBudget>,
    legacy: Arc<LegacyCache>,
    /// Held shared around every read of the world's files (see `world_map_files`).
    files: Arc<RwLock<()>>,
    instance: InstanceId,
    world_name: String,
    dimension: String,
    /// The world's folder as it was when the request came.
    world: PathBuf,
    /// The dimension's region folder.
    folder: PathBuf,
    roof: Roof,
    /// Every region under the asked piece that exists.
    regions: HashMap<(i32, i32), RegionAt>,
    given_up: Arc<AtomicBool>,
    #[cfg(test)]
    pause: Option<Pause>,
    #[cfg(test)]
    read_pause: Option<Pause>,
}

impl Painter {
    /// The piece, from the store when it holds the current one; `None` over no region.
    fn piece(
        self: &Arc<Self>,
        scale: u8,
        x: i32,
        z: i32,
    ) -> BoxFuture<'_, anyhow::Result<Option<Drawn>>> {
        Box::pin(async move {
            let Some(revision) = self.revision(scale, x, z) else {
                return Ok(None);
            };
            if let Some(png) = self.kept(scale, x, z, &revision).await? {
                return Ok(Some(Drawn::Kept(png)));
            }
            let painted = if scale == 0 {
                match self.paint(x, z).await? {
                    Some(painted) => painted,
                    None => return Ok(None),
                }
            } else {
                self.joined(scale, x, z).await?
            };
            let this = self.clone();
            Ok(Some(
                spawn_blocking(move || this.keep(scale, x, z, &revision, painted)).await??,
            ))
        })
    }

    /// The piece as the store keeps it for `revision`.
    async fn kept(
        self: &Arc<Self>,
        scale: u8,
        x: i32,
        z: i32,
        revision: &str,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let (this, revision) = (self.clone(), revision.to_owned());
        Ok(spawn_blocking(move || this.store.get(&this.key(scale, x, z, &revision))).await?)
    }

    /// The revision of the piece: its region's at scale 0, else a digest of the revisions of
    /// the regions under it. `None` when no region is under it.
    fn revision(&self, scale: u8, x: i32, z: i32) -> Option<String> {
        if scale == 0 {
            return self
                .regions
                .get(&(x, z))
                .map(|region| region.revision.clone());
        }
        let (xs, zs) = regions_under(scale, x, z)?;
        let mut digest = Sha256::new();
        let mut any = false;
        for z in zs {
            for x in xs.clone() {
                if let Some(region) = self.regions.get(&(x, z)) {
                    digest.update(format!("{x},{z}:{};", region.revision));
                    any = true;
                }
            }
        }
        any.then(|| short_hex(digest))
    }

    fn key<'a>(&'a self, scale: u8, x: i32, z: i32, revision: &'a str) -> PieceKey<'a> {
        PieceKey {
            instance: self.instance,
            world: &self.world_name,
            dimension: &self.dimension,
            scale,
            x,
            z,
            revision,
        }
    }

    fn given_up(&self) -> anyhow::Result<()> {
        if self.given_up.load(Ordering::Acquire) {
            return Err(anyhow!("the map piece was given up"));
        }
        Ok(())
    }

    /// The region `(x, z)` painted; `None` when it is gone since its revision was taken.
    async fn paint(self: &Arc<Self>, x: i32, z: i32) -> anyhow::Result<Option<Painted>> {
        let bytes = self.regions.get(&(x, z)).map_or(0, bytes_read);
        // A request given up while it waits here is dropped with its wait.
        let admitted = self.painting.admit(bytes).await;
        let this = self.clone();
        spawn_blocking(move || {
            let _admitted = admitted;
            this.paint_blocking(x, z)
        })
        .await?
    }

    fn paint_blocking(&self, x: i32, z: i32) -> anyhow::Result<Option<Painted>> {
        self.given_up()?;
        // Each file is read and closed before its chunks are parsed: the region whole, the one
        // to its north only for its last row. Never waited for: a request that finds the files
        // closed answers nothing.
        let open = self.files.try_read().map_err(|_| FilesClosed)?;
        let Some(region) = Region::read(&region_file(&self.folder, x, z))? else {
            return Ok(None);
        };
        #[cfg(test)]
        if let Some(pause) = &self.read_pause {
            pause();
        }
        let mut north_unread = false;
        let north = match z.checked_sub(1) {
            None => None,
            Some(north) => Region::read_row(&region_file(&self.folder, x, north), 31)
                .unwrap_or_else(|error| {
                    debug!(?error, "could not read the region north of {x},{z}");
                    north_unread = true;
                    None
                }),
        };
        drop(open);
        let cancelled = || {
            #[cfg(test)]
            if let Some(pause) = &self.pause {
                pause();
            }
            self.given_up.load(Ordering::Acquire)
        };
        // Asked of `level.dat` only when a chunk holds numeric ids.
        let loaded = std::cell::OnceCell::new();
        let closed = std::cell::Cell::new(false);
        let legacy = || {
            loaded
                .get_or_init(|| match self.files.try_read() {
                    Ok(_open) => self.legacy.ids(&self.world),
                    Err(_) => {
                        closed.set(true);
                        Arc::new(None)
                    }
                })
                .as_ref()
                .as_ref()
        };
        let Some(mut painted) =
            paint_region(&region, north.as_ref(), self.roof, &legacy, &cancelled)
        else {
            return Err(anyhow!("the map piece was given up"));
        };
        if closed.get() {
            return Err(FilesClosed.into());
        }
        painted.complete &= !north_unread;
        Ok(Some(painted))
    }

    /// The four pieces of the scale below, joined; a child that could not be had leaves a
    /// hole and the piece incomplete. Each child is folded in as it comes, so only one is held.
    async fn joined(self: &Arc<Self>, scale: u8, x: i32, z: i32) -> anyhow::Result<Painted> {
        let below = scale - 1;
        let (x, z) = (x * 2, z * 2);
        let mut parent = RgbaImage::new(PIECE, PIECE);
        let mut complete = true;
        for (quarter, (x, z)) in [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)]
            .into_iter()
            .enumerate()
        {
            self.given_up()?;
            let drawn = match self.piece(below, x, z).await {
                Ok(None) => continue,
                Ok(Some(drawn)) => drawn,
                Err(error) => {
                    self.given_up()?;
                    if error.is::<FilesClosed>() {
                        return Err(error);
                    }
                    debug!(?error, "a map piece at scale {below} could not be painted");
                    complete = false;
                    continue;
                }
            };
            if let Drawn::Painted {
                complete: whole, ..
            } = &drawn
            {
                complete &= *whole;
            }
            let (folded, read) = spawn_blocking(move || {
                let child = match drawn {
                    Drawn::Painted { image, .. } => Some(image),
                    Drawn::Kept(png) => image::load_from_memory_with_format(&png, ImageFormat::Png)
                        .map_err(|error| debug!(?error, "a kept map piece could not be read"))
                        .ok()
                        .map(|image| image.into_rgba8()),
                };
                if let Some(child) = &child {
                    fold(&mut parent, quarter, child);
                }
                (parent, child.is_some())
            })
            .await?;
            parent = folded;
            complete &= read;
        }
        Ok(Painted {
            image: parent,
            complete,
        })
    }

    /// Keeps a complete piece, unless it was given up or its world moved away meanwhile: a
    /// piece kept under the folder the world left would never be asked for again.
    fn keep(
        &self,
        scale: u8,
        x: i32,
        z: i32,
        revision: &str,
        painted: Painted,
    ) -> anyhow::Result<Drawn> {
        let Painted { image, complete } = painted;
        // Taken for gone while its files are closed to the map.
        let world_there = self
            .files
            .try_read()
            .is_ok_and(|_open| fs::symlink_metadata(&self.world).is_ok_and(|meta| meta.is_dir()));
        if !complete || !world_there {
            return Ok(Drawn::Painted {
                image,
                png: None,
                complete,
            });
        }
        self.given_up()?;
        let png = png(&image)?;
        if let Err(error) = self.store.put(&self.key(scale, x, z, revision), &png) {
            debug!(?error, "could not keep a map piece");
        }
        Ok(Drawn::Painted {
            image,
            png: Some(png),
            complete,
        })
    }
}

#[cfg(test)]
type Pause = Arc<dyn Fn() + Send + Sync>;

/// What a test runs at every chunk a painting reads, to hold the painting there.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct PaintPause(std::sync::Mutex<Option<Pause>>);

#[cfg(test)]
impl std::fmt::Debug for PaintPause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PaintPause")
    }
}

#[cfg(test)]
impl PaintPause {
    fn set(&self, pause: Pause) {
        *self.0.lock().unwrap() = Some(pause);
    }

    fn get(&self) -> Option<Pause> {
        self.0.lock().unwrap().clone()
    }
}

/// How many requests are building a piece of scale 1 and up, and the most there were at once.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct BuildCount {
    now: std::sync::atomic::AtomicUsize,
    most: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl BuildCount {
    fn enter(&self) -> Building<'_> {
        let now = self.now.fetch_add(1, Ordering::AcqRel) + 1;
        self.most.fetch_max(now, Ordering::AcqRel);
        Building(self)
    }

    fn now(&self) -> usize {
        self.now.load(Ordering::Acquire)
    }

    fn most(&self) -> usize {
        self.most.load(Ordering::Acquire)
    }
}

#[cfg(test)]
struct Building<'a>(&'a BuildCount);

#[cfg(test)]
impl Drop for Building<'_> {
    fn drop(&mut self) {
        self.0.now.fetch_sub(1, Ordering::AcqRel);
    }
}

/// An instance with the world `Home` holding a region at `(0, 0)` with one chunk, for tests
/// of what serves pieces.
#[cfg(test)]
pub(crate) async fn test_world(app: &crate::managers::App, instance: &str) -> InstanceId {
    let (id, world) = tests::world(app, instance).await;
    tests::put_region(&world, 0, 0, &[tests::ground(0, 0, "minecraft:stone")]);
    id
}

/// An instance with the world `Home` holding a whole region at `(0, 0)`, one with an unreadable
/// chunk at `(1, 0)` and, on unix, one at `(2, 0)` that cannot be opened, for tests of what
/// serves pieces.
#[cfg(test)]
pub(crate) async fn test_world_with_faults(
    app: &crate::managers::App,
    instance: &str,
) -> InstanceId {
    let (id, world) = tests::world(app, instance).await;
    tests::put_region(&world, 0, 0, &[tests::ground(0, 0, "minecraft:stone")]);
    tests::put_region(&world, 1, 0, &[(0, 0, 2, b"not zlib".to_vec())]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let closed = tests::put_region(&world, 2, 0, &[tests::ground(0, 0, "minecraft:stone")]);
        std::fs::set_permissions(closed, std::fs::Permissions::from_mode(0o000)).unwrap();
    }
    id
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Condvar, Mutex};
    use std::time::{Duration, SystemTime};

    use image::{ImageFormat, RgbaImage};

    use super::super::super::mods::tests::addon_fixture;
    use super::super::super::world_level::{gzipped, level_dat_bytes};
    use super::super::chunk::tests::{FULL, chunk_1_18, fill, numeric_chunk, numeric_section};
    use super::super::colors::UNKNOWN;
    use super::super::legacy::tests::{level_dat_1_12, vanilla_level_dat};
    use super::super::paint::PIECE;
    use super::super::region::test::{region_bytes, zlib};
    use super::*;
    use crate::domain::instance::AddonType;
    use crate::managers::App;
    use crate::managers::instance::world_rename::RenameWorld;

    const OVERWORLD: &str = "minecraft:overworld";

    pub(in super::super) async fn world(app: &App, instance: &str) -> (InstanceId, PathBuf) {
        let (id, _, world) = addon_fixture(
            app,
            instance,
            AddonType::Worlds,
            "Home",
            &level_dat_bytes(Some("Home")),
        )
        .await;
        (id, world)
    }

    /// The slot `(x, z)` of a region holding a flat chunk of `block` at height 0.
    pub(in super::super) fn ground(x: u8, z: u8, block: &str) -> (u8, u8, u8, Vec<u8>) {
        let indices = fill(|_, y, _| u16::from(y == 0));
        let nbt = chunk_1_18(vec![(0, vec!["minecraft:air", block], indices)], FULL);
        (x, z, 2, zlib(&nbt))
    }

    /// Writes the overworld region `(x, z)` of `world` holding `chunks`.
    pub(in super::super) fn put_region(
        world: &Path,
        x: i32,
        z: i32,
        chunks: &[(u8, u8, u8, Vec<u8>)],
    ) -> PathBuf {
        let folder = world.join("region");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(format!("r.{x}.{z}.mca"));
        std::fs::write(&path, region_bytes(chunks)).unwrap();
        path
    }

    fn request(id: InstanceId, scale: u8, x: i32, z: i32) -> PieceRequest {
        PieceRequest {
            instance_id: id.0,
            world: "Home".into(),
            dimension: OVERWORLD.into(),
            scale,
            x,
            z,
            rev: "any".into(),
        }
    }

    async fn whole(app: &App, request: PieceRequest) -> Option<Piece> {
        app.instance_manager()
            .world_map_piece(request)
            .await
            .unwrap()
    }

    async fn piece(app: &App, request: PieceRequest) -> Option<Vec<u8>> {
        whole(app, request).await.map(|piece| piece.png)
    }

    fn decoded(png: &[u8]) -> RgbaImage {
        image::load_from_memory_with_format(png, ImageFormat::Png)
            .unwrap()
            .into_rgba8()
    }

    /// Every file the store holds.
    fn kept(app: &App) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut folders = vec![app.settings_manager().runtime_path.get_world_maps()];
        while let Some(folder) = folders.pop() {
            let Ok(entries) = std::fs::read_dir(&folder) else {
                continue;
            };
            for entry in entries.flatten() {
                if entry.file_type().unwrap().is_dir() {
                    folders.push(entry.path());
                } else {
                    found.push(entry.path());
                }
            }
        }
        found
    }

    /// Holds every painting at its next chunk while closed, counting those it holds.
    #[derive(Default)]
    struct Gate {
        state: Mutex<GateState>,
        changed: Condvar,
    }

    #[derive(Default)]
    struct GateState {
        open: bool,
        held: usize,
        most_held: usize,
        passes: usize,
    }

    impl Gate {
        fn install(app: &App) -> Arc<Gate> {
            Gate::install_on(&app.instance_manager().world_map_pause)
        }

        /// Holds every painting inside its file read, after its own region is read.
        fn install_in_read(app: &App) -> Arc<Gate> {
            Gate::install_on(&app.instance_manager().world_map_read_pause)
        }

        fn install_on(pause: &PaintPause) -> Arc<Gate> {
            let gate = Arc::new(Gate::default());
            let held = gate.clone();
            pause.set(Arc::new(move || held.pass()));
            gate
        }

        fn pass(&self) {
            let mut state = self.state.lock().unwrap();
            state.passes += 1;
            if state.open {
                return;
            }
            state.held += 1;
            state.most_held = state.most_held.max(state.held);
            self.changed.notify_all();
            while !state.open {
                state = self.changed.wait(state).unwrap();
            }
            state.held -= 1;
        }

        fn open(&self) {
            self.state.lock().unwrap().open = true;
            self.changed.notify_all();
        }

        fn held(&self) -> usize {
            self.state.lock().unwrap().held
        }

        fn most_held(&self) -> usize {
            self.state.lock().unwrap().most_held
        }

        fn passes(&self) -> usize {
            self.state.lock().unwrap().passes
        }
    }

    async fn until(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        while !done() {
            assert!(tokio::time::Instant::now() < deadline, "waited for {what}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn spawn_piece(
        app: &App,
        request: PieceRequest,
    ) -> tokio::task::JoinHandle<anyhow::Result<Option<Piece>>> {
        let app = app.clone();
        tokio::spawn(async move { app.instance_manager().world_map_piece(request).await })
    }

    #[tokio::test]
    async fn a_piece_is_painted_then_served_from_the_store() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-served").await;
        let region = put_region(&world, 0, 0, &[ground(0, 0, "minecraft:stone")]);

        let painted = whole(&app, request(id, 0, 0, 0)).await.expect("a piece");
        assert!(painted.complete);
        let image = decoded(&painted.png);
        assert_eq!(image.dimensions(), (PIECE, PIECE));
        assert_eq!(image.get_pixel(0, 0)[3], 255);
        assert_eq!(image.get_pixel(16, 0)[3], 0);
        assert_eq!(kept(&app).len(), 1);

        // The second answer cannot come from the region: it can no longer be read.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&region, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        let served = whole(&app, request(id, 0, 0, 0)).await;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&region, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        assert_eq!(served, Some(painted));
    }

    #[tokio::test]
    async fn a_changed_region_gives_a_new_picture() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-changed").await;
        let region = put_region(&world, 0, 0, &[ground(0, 0, "minecraft:stone")]);
        let before = piece(&app, request(id, 0, 0, 0)).await.unwrap();
        let kept_before = kept(&app);
        assert_eq!(kept_before.len(), 1);

        put_region(&world, 0, 0, &[ground(0, 0, "minecraft:sand")]);
        std::fs::File::options()
            .write(true)
            .open(&region)
            .unwrap()
            .set_modified(SystemTime::now() + Duration::from_secs(10))
            .unwrap();
        let after = piece(&app, request(id, 0, 0, 0)).await.unwrap();
        assert_ne!(
            decoded(&before).get_pixel(0, 0),
            decoded(&after).get_pixel(0, 0)
        );
        let kept_after = kept(&app);
        assert_eq!(kept_after.len(), 1, "{kept_after:?}");
        assert_ne!(kept_after, kept_before);
    }

    #[tokio::test]
    async fn a_scale_one_piece_joins_four() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-joined").await;
        let quarters = [
            (0, 0, "minecraft:red_wool"),
            (1, 0, "minecraft:blue_wool"),
            (0, 1, "minecraft:lime_wool"),
            (1, 1, "minecraft:snow_block"),
        ];
        for (x, z, block) in quarters {
            put_region(&world, x, z, &[ground(0, 0, block)]);
        }

        let joined = decoded(&piece(&app, request(id, 1, 0, 0)).await.unwrap());
        let half = PIECE / 2;
        let mut seen = Vec::new();
        for (x, z, _) in quarters {
            let region = decoded(&piece(&app, request(id, 0, x, z)).await.unwrap());
            let (left, top) = (x as u32 * half, z as u32 * half);
            let pixel = *joined.get_pixel(left, top);
            assert_eq!(pixel, *region.get_pixel(0, 0), "quarter {x},{z}");
            assert_eq!(pixel[3], 255);
            // A chunk of 16 blocks is 8 pixels wide at this scale.
            assert_eq!(*joined.get_pixel(left + 7, top + 7), pixel);
            assert_eq!(joined.get_pixel(left + 8, top)[3], 0);
            assert!(!seen.contains(&pixel));
            seen.push(pixel);
        }
        // The joined piece and its four children.
        assert_eq!(kept(&app).len(), 5);
    }

    #[tokio::test]
    async fn a_region_that_does_not_exist_is_nothing() {
        let app = crate::setup_managers_for_test().await;
        let id = test_world(&app, "map-nothing").await;
        assert!(piece(&app, request(id, 0, 0, 0)).await.is_some());
        assert_eq!(piece(&app, request(id, 0, 5, 5)).await, None);
        assert_eq!(piece(&app, request(id, 0, 0, -1)).await, None);
        // Out of bounds: no scale above the largest, no address that is not one.
        assert_eq!(piece(&app, request(id, MAX_SCALE + 1, 0, 0)).await, None);
        for rev in ["", "a/b", "../x", &"a".repeat(65)] {
            let request = PieceRequest {
                rev: rev.to_string(),
                ..request(id, 0, 0, 0)
            };
            assert_eq!(piece(&app, request).await, None, "{rev:?}");
        }
        let longest = PieceRequest {
            rev: "a".repeat(64),
            ..request(id, 0, 0, 0)
        };
        assert!(piece(&app, longest).await.is_some());
    }

    #[tokio::test]
    async fn a_scale_piece_over_nothing_is_nothing() {
        let app = crate::setup_managers_for_test().await;
        let id = test_world(&app, "map-nothing-scaled").await;
        assert!(piece(&app, request(id, 2, 0, 0)).await.is_some());
        assert_eq!(piece(&app, request(id, 2, 10, 10)).await, None);
        assert_eq!(piece(&app, request(id, 1, -1, -1)).await, None);
        assert_eq!(piece(&app, request(id, MAX_SCALE, i32::MAX, 0)).await, None);
        assert_eq!(
            kept(&app).len(),
            3,
            "the piece at scale 2 and those beneath it"
        );
    }

    #[tokio::test]
    async fn negative_pieces_cover_the_regions_west_and_north() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-negative").await;
        put_region(&world, -2, -2, &[ground(0, 0, "minecraft:stone")]);
        assert_eq!(piece(&app, request(id, 1, 0, 0)).await, None);
        let joined = decoded(&piece(&app, request(id, 1, -1, -1)).await.unwrap());
        assert_eq!(joined.get_pixel(0, 0)[3], 255);
    }

    #[tokio::test]
    async fn a_piece_with_an_unreadable_chunk_is_answered_but_not_kept() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-unreadable").await;
        put_region(
            &world,
            0,
            0,
            &[
                ground(0, 0, "minecraft:stone"),
                (1, 0, 2, b"not zlib".to_vec()),
            ],
        );
        let answered = whole(&app, request(id, 0, 0, 0)).await.unwrap();
        assert!(!answered.complete);
        assert_eq!(decoded(&answered.png).get_pixel(0, 0)[3], 255);
        let parent = whole(&app, request(id, 1, 0, 0)).await.unwrap();
        assert!(!parent.complete, "a parent with a child that has a hole");
        assert_eq!(kept(&app), Vec::<PathBuf>::new());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_piece_whose_north_region_cannot_be_read_is_answered_but_not_kept() {
        use std::os::unix::fs::PermissionsExt;
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-north-unreadable").await;
        put_region(&world, 0, 0, &[ground(0, 0, "minecraft:stone")]);
        let north = put_region(&world, 0, -1, &[ground(0, 31, "minecraft:stone")]);
        std::fs::set_permissions(&north, std::fs::Permissions::from_mode(0o000)).unwrap();
        let answered = whole(&app, request(id, 0, 0, 0)).await.unwrap();
        std::fs::set_permissions(&north, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!answered.complete);
        assert_eq!(decoded(&answered.png).get_pixel(0, 0)[3], 255);
        assert_eq!(kept(&app), Vec::<PathBuf>::new());
    }

    #[tokio::test]
    async fn a_dimension_the_world_does_not_list_is_nothing() {
        let app = crate::setup_managers_for_test().await;
        let id = test_world(&app, "map-dimension").await;
        assert!(piece(&app, request(id, 0, 0, 0)).await.is_some());
        for dimension in ["../../etc", "minecraft:the_end", "region", ""] {
            let request = PieceRequest {
                dimension: dimension.into(),
                ..request(id, 0, 0, 0)
            };
            assert_eq!(piece(&app, request).await, None, "{dimension:?}");
        }
    }

    #[tokio::test]
    async fn a_world_that_is_not_there_is_nothing() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-gone").await;
        put_region(&world, 0, 0, &[ground(0, 0, "minecraft:stone")]);
        let missing = PieceRequest {
            world: "Missing".into(),
            ..request(id, 0, 0, 0)
        };
        assert_eq!(piece(&app, missing).await, None);
        #[cfg(unix)]
        {
            // A world replaced by a link is not followed.
            let saves = world.parent().unwrap();
            std::fs::rename(&world, saves.join("Moved")).unwrap();
            std::os::unix::fs::symlink(saves.join("Moved"), &world).unwrap();
            assert_eq!(piece(&app, request(id, 0, 0, 0)).await, None);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_region_that_cannot_be_read_is_an_error() {
        use std::os::unix::fs::PermissionsExt;
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-closed").await;
        let region = put_region(&world, 0, 0, &[ground(0, 0, "minecraft:stone")]);
        std::fs::set_permissions(&region, std::fs::Permissions::from_mode(0o000)).unwrap();
        let answer = app
            .instance_manager()
            .world_map_piece(request(id, 0, 0, 0))
            .await;
        std::fs::set_permissions(&region, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(answer.is_err(), "{answer:?}");
    }

    #[tokio::test]
    async fn a_dropped_request_stops_its_painting() {
        let app = crate::setup_managers_for_test().await;
        let id = test_world(&app, "map-dropped").await;
        let gate = Gate::install(&app);

        let task = spawn_piece(&app, request(id, 0, 0, 0));
        until("the painting to reach a chunk", || gate.held() == 1).await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        gate.open();

        let painting = app.instance_manager().world_map_painting.clone();
        until("the permit to come back", || painting.idle()).await;
        assert_eq!(
            gate.passes(),
            1,
            "the painting stopped at the chunk it was held at"
        );
        assert_eq!(kept(&app), Vec::<PathBuf>::new());
    }

    #[tokio::test]
    async fn at_most_three_pieces_are_painted_at_once() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-at-once").await;
        for x in 0..8 {
            put_region(&world, x, 0, &[ground(0, 0, "minecraft:stone")]);
        }
        let gate = Gate::install(&app);

        let tasks: Vec<_> = (0..8)
            .map(|x| spawn_piece(&app, request(id, 0, x, 0)))
            .collect();
        until("three paintings", || gate.held() == PAINTING_AT_ONCE).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(gate.held(), PAINTING_AT_ONCE);
        gate.open();
        for task in tasks {
            assert!(task.await.unwrap().unwrap().is_some());
        }
        assert_eq!(gate.most_held(), PAINTING_AT_ONCE);
        assert_eq!(kept(&app).len(), 8);
    }

    #[tokio::test]
    async fn the_world_can_be_renamed_while_a_piece_is_painted() {
        let app = crate::setup_managers_for_test().await;
        let id = test_world(&app, "map-renamed").await;
        let listed = app.instance_manager().list_worlds(id).await.unwrap();
        let home = listed.iter().find(|world| world.name == "Home").unwrap();
        let gate = Gate::install(&app);

        let task = spawn_piece(&app, request(id, 0, 0, 0));
        until("the painting to reach a chunk", || gate.held() == 1).await;
        let folder = app
            .instance_manager()
            .rename_world(RenameWorld {
                instance_id: id.0,
                world: "Home".into(),
                identity: home.identity.clone().unwrap(),
                name: "Elsewhere".into(),
            })
            .await
            .unwrap();
        assert_ne!(folder, "Home");
        // What the rename forgets is gone before the painting ends.
        app.instance_manager().forgetting_done().await;
        gate.open();

        task.await.unwrap().unwrap();
        assert_eq!(kept(&app), Vec::<PathBuf>::new());
    }

    /// The overworld region `(x, z)` holding one chunk of stone at height 0 in numeric ids, as
    /// 1.12.2 writes it.
    fn numeric_region(world: &Path, x: i32, z: i32) {
        let section = numeric_section(0, |_, y, _| (u16::from(y == 0), 0));
        let nbt = numeric_chunk(vec![section], Some(1));
        put_region(world, x, z, &[(0, 0, 2, zlib(&nbt))]);
    }

    fn set_level_dat(world: &Path, nbt: &[u8], later: u64) {
        let path = world.join("level.dat");
        std::fs::write(&path, gzipped(nbt)).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(SystemTime::now() + Duration::from_secs(later))
            .unwrap();
    }

    #[tokio::test]
    async fn the_numeric_ids_of_a_world_are_read_once_while_its_level_dat_is_unchanged() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-legacy-once").await;
        // Id 1 is a mod's block in this world, so its chunks are grey.
        set_level_dat(&world, &level_dat_1_12(&[("ic2:rubber_wood", 1)]), 0);
        for x in 0..6 {
            numeric_region(&world, x, 0);
        }
        let legacy = app.instance_manager().world_map_legacy.clone();

        let tasks: Vec<_> = (0..6)
            .map(|x| spawn_piece(&app, request(id, 0, x, 0)))
            .collect();
        let grey = UNKNOWN.map(|c| (u16::from(c) * 220 / 255) as u8);
        for task in tasks {
            let piece = task.await.unwrap().unwrap().unwrap();
            let pixel = *decoded(&piece.png).get_pixel(0, 0);
            assert_eq!(pixel, image::Rgba([grey[0], grey[1], grey[2], 255]));
        }
        assert_eq!(legacy.reads(), 1, "six cold pieces, one read");

        // A level.dat written again is read again, and only once.
        set_level_dat(&world, &vanilla_level_dat(), 10);
        numeric_region(&world, 8, 0);
        numeric_region(&world, 9, 0);
        let stone = decoded(&piece(&app, request(id, 0, 8, 0)).await.unwrap());
        assert_ne!(stone.get_pixel(0, 0).0[..3], grey[..]);
        assert!(piece(&app, request(id, 0, 9, 0)).await.is_some());
        assert_eq!(legacy.reads(), 2);
    }

    #[tokio::test]
    async fn a_world_of_block_names_reads_no_level_dat_to_paint() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-legacy-none").await;
        for x in 0..3 {
            put_region(&world, x, 0, &[ground(0, 0, "minecraft:stone")]);
        }
        for x in 0..3 {
            assert!(piece(&app, request(id, 0, x, 0)).await.is_some());
        }
        assert!(piece(&app, request(id, 1, 0, 0)).await.is_some());
        assert_eq!(app.instance_manager().world_map_legacy.reads(), 0);
    }

    #[tokio::test]
    async fn a_rename_waits_for_a_file_read_in_flight_then_succeeds() {
        let app = crate::setup_managers_for_test().await;
        let id = test_world(&app, "map-rename-read").await;
        let listed = app.instance_manager().list_worlds(id).await.unwrap();
        let home = listed.iter().find(|world| world.name == "Home").unwrap();
        let gate = Gate::install_in_read(&app);

        let task = spawn_piece(&app, request(id, 0, 0, 0));
        until("the painting to read its region", || gate.held() == 1).await;
        let rename = {
            let (app, identity) = (app.clone(), home.identity.clone().unwrap());
            tokio::spawn(async move {
                app.instance_manager()
                    .rename_world(RenameWorld {
                        instance_id: id.0,
                        world: "Home".into(),
                        identity,
                        name: "Elsewhere".into(),
                    })
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(300)).await;
        // Read before opening, asserted after: a closed gate would hold the painting for good.
        let early = rename.is_finished();
        gate.open();
        assert!(!early, "the rename moved a folder with a file open");

        assert_ne!(rename.await.unwrap().unwrap(), "Home");
        task.await.unwrap().unwrap();
        app.instance_manager().forgetting_done().await;
        assert_eq!(kept(&app), Vec::<PathBuf>::new());
    }

    #[tokio::test]
    async fn a_delete_waits_for_a_file_read_in_flight_then_succeeds() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, world) = addon_fixture(
            &app,
            "map-delete-read",
            AddonType::Worlds,
            "Home",
            &level_dat_bytes(Some("Home")),
        )
        .await;
        put_region(&world, 0, 0, &[ground(0, 0, "minecraft:stone")]);
        let gate = Gate::install_in_read(&app);

        let task = spawn_piece(&app, request(id, 0, 0, 0));
        until("the painting to read its region", || gate.held() == 1).await;
        let delete = {
            let app = app.clone();
            tokio::spawn(async move { app.instance_manager().delete_mod(id, row.id).await })
        };
        tokio::time::sleep(Duration::from_millis(300)).await;
        // Read before opening, asserted after: a closed gate would hold the painting for good.
        let early = delete.is_finished();
        let still_there = world.exists();
        gate.open();
        assert!(!early, "the delete removed a folder with a file open");
        assert!(still_there);

        delete.await.unwrap().unwrap();
        assert!(!world.exists());
        task.await.unwrap().unwrap();
        app.instance_manager().forgetting_done().await;
        assert_eq!(kept(&app), Vec::<PathBuf>::new());
    }

    #[tokio::test]
    async fn while_the_files_are_closed_a_request_opens_none() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-files-closed").await;
        let region = put_region(&world, 0, 0, &[ground(0, 0, "minecraft:stone")]);
        // A file read would be an error.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&region, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        let gate = Gate::install_in_read(&app);
        gate.open();
        let manager = app.instance_manager();

        let closed = manager.close_world_map_files(id).await;
        for scale in [0, 1, MAX_SCALE] {
            assert_eq!(whole(&app, request(id, scale, 0, 0)).await, None);
        }
        let facts = {
            let app = app.clone();
            tokio::spawn(async move { app.instance_manager().world_map(id, "Home".into()).await })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !facts.is_finished(),
            "the facts were read while the files were closed"
        );
        drop(closed);
        let facts = facts.await.unwrap().unwrap();
        assert_eq!(facts.dimensions.len(), 1);
        assert_eq!(gate.passes(), 0, "no region was read");
        assert_eq!(kept(&app), Vec::<PathBuf>::new());

        #[cfg(unix)]
        {
            let answer = manager.world_map_piece(request(id, 0, 0, 0)).await;
            assert!(
                answer.is_err(),
                "open again, the region is read: {answer:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_painting_that_finds_the_files_closed_keeps_nothing() {
        let app = crate::setup_managers_for_test().await;
        let id = test_world(&app, "map-files-closed-late").await;
        let gate = Gate::install(&app);
        // The painting is held at its first chunk, its file read done.
        let task = spawn_piece(&app, request(id, 1, 0, 0));
        until("the painting to reach a chunk", || gate.held() == 1).await;
        let closed = app.instance_manager().close_world_map_files(id).await;
        gate.open();
        // It has its bytes, so it may end; what it painted is not kept.
        task.await.unwrap().unwrap();
        drop(closed);
        assert_eq!(kept(&app), Vec::<PathBuf>::new());
    }

    /// Regions at `(4k, 0)`, each alone under the scale 2 piece `(k, 0)`.
    fn one_region_per_scale_two_piece(world: &Path, pieces: i32) {
        for k in 0..pieces {
            put_region(world, 4 * k, 0, &[ground(0, 0, "minecraft:stone")]);
        }
    }

    #[tokio::test]
    async fn at_most_four_joined_pieces_are_built_at_once() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-building").await;
        one_region_per_scale_two_piece(&world, 8);
        let gate = Gate::install(&app);
        let manager = app.instance_manager();
        let builds = &manager.world_map_builds;

        let tasks: Vec<_> = (0..8)
            .map(|k| spawn_piece(&app, request(id, 2, k, 0)))
            .collect();
        until("four builds and three paintings", || {
            builds.most() >= BUILDING_AT_ONCE && gate.held() == PAINTING_AT_ONCE
        })
        .await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        // Read before opening, asserted after: a closed gate would hold the paintings for good.
        let most_while_held = builds.most();
        gate.open();
        assert_eq!(most_while_held, BUILDING_AT_ONCE);
        for task in tasks {
            assert!(task.await.unwrap().unwrap().unwrap().complete);
        }
        assert_eq!(builds.most(), BUILDING_AT_ONCE);
        assert_eq!(builds.now(), 0);
        // Each piece at scale 2, 1 and 0.
        assert_eq!(kept(&app).len(), 24);
    }

    #[tokio::test]
    async fn a_request_dropped_while_waiting_to_build_leaves_the_queue() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-building-dropped").await;
        one_region_per_scale_two_piece(&world, 6);
        let gate = Gate::install(&app);
        let manager = app.instance_manager();
        let builds = &manager.world_map_builds;

        let tasks: Vec<_> = (0..4)
            .map(|k| spawn_piece(&app, request(id, 2, k, 0)))
            .collect();
        until("four builds", || {
            builds.now() == BUILDING_AT_ONCE && gate.held() == PAINTING_AT_ONCE
        })
        .await;
        let waiting = spawn_piece(&app, request(id, 2, 4, 0));
        tokio::time::sleep(Duration::from_millis(200)).await;
        let most_while_held = builds.most();
        waiting.abort();
        let dropped = waiting.await;
        gate.open();
        assert_eq!(most_while_held, BUILDING_AT_ONCE);
        assert!(dropped.unwrap_err().is_cancelled());
        for task in tasks {
            assert!(task.await.unwrap().unwrap().is_some());
        }
        let building = manager.world_map_building.clone();
        until("the permits to come back", || {
            building.available_permits() == BUILDING_AT_ONCE
        })
        .await;
        assert_eq!(kept(&app).len(), 12, "nothing of the dropped request");
        assert!(piece(&app, request(id, 2, 5, 0)).await.is_some());
        assert_eq!(kept(&app).len(), 15);
    }
}
