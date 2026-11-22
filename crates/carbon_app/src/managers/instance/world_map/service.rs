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

use anyhow::anyhow;
use futures::future::BoxFuture;
use image::{ImageFormat, RgbaImage};
use sha2::{Digest, Sha256};
use tokio::sync::{OnceCell, Semaphore};
use tokio::task::spawn_blocking;
use tracing::debug;

use super::super::{InstanceId, InstanceManager, world_level::inflate_level_dat};
use super::chunk::Roof;
use super::facts::{dimension_folder, has_ceiling, region_file, region_revision_at, short_hex};
use super::legacy::LegacyIds;
use super::paint::{Painted, join, paint_region, png};
use super::region::Region;
use super::store::{PieceKey, Store};
use crate::managers::ManagerRef;

/// The largest scale: a piece of it covers 16 by 16 regions.
pub(crate) const MAX_SCALE: u8 = 4;

/// How many regions are painted at once across the launcher.
pub(crate) const PAINTING_AT_ONCE: usize = 3;

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

impl ManagerRef<'_, InstanceManager> {
    /// The PNG of a piece; `None` when nothing of the world lies under it. No gate is taken:
    /// the map is drawn while the game runs, and a world renamed or deleted meanwhile only
    /// makes the answer `None` or a piece that is not kept.
    pub async fn world_map_piece(self, request: PieceRequest) -> anyhow::Result<Option<Vec<u8>>> {
        if request.scale > MAX_SCALE || !is_revision(&request.rev) {
            return Ok(None);
        }
        let instance = InstanceId(request.instance_id);
        let world = self.map_world_folder(instance, &request.world).await?;
        let (scale, x, z) = (request.scale, request.x, request.z);
        let found = {
            let (world, dimension) = (world.clone(), request.dimension.clone());
            spawn_blocking(move || {
                // Lists the whole world, so asked once per request.
                let folder = dimension_folder(&world, &dimension)?;
                let revisions = revisions_beneath(&folder, scale, x, z)?;
                Some((folder, revisions))
            })
            .await?
        };
        let Some((folder, revisions)) = found else {
            return Ok(None);
        };
        let given_up = Arc::new(AtomicBool::new(false));
        let _give_up = GiveUpOnDrop(given_up.clone());
        let painter = Arc::new(Painter {
            store: self.world_map_store(),
            painting: self.world_map_painting.clone(),
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
            revisions,
            legacy: OnceCell::new(),
            given_up,
            #[cfg(test)]
            pause: self.world_map_pause.get(),
        });
        Ok(match painter.piece(scale, x, z).await? {
            None => None,
            Some(Piece::Kept(png) | Piece::Painted { png: Some(png), .. }) => Some(png),
            Some(Piece::Painted { image, .. }) => Some(spawn_blocking(move || png(&image)).await??),
        })
    }
}

/// Whether `rev` can be a revision: the store keeps pieces under no other.
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

/// The revision of every region under the piece that exists, asked of each file by its name.
fn revisions_beneath(
    folder: &Path,
    scale: u8,
    x: i32,
    z: i32,
) -> Option<HashMap<(i32, i32), String>> {
    let (xs, zs) = regions_under(scale, x, z)?;
    let mut revisions = HashMap::new();
    for z in zs {
        for x in xs.clone() {
            if let Some(revision) = region_revision_at(folder, x, z) {
                revisions.insert((x, z), revision);
            }
        }
    }
    Some(revisions)
}

/// Set when the request is dropped: the desktop gave the piece up.
struct GiveUpOnDrop(Arc<AtomicBool>);

impl Drop for GiveUpOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

enum Piece {
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
    painting: Arc<Semaphore>,
    instance: InstanceId,
    world_name: String,
    dimension: String,
    /// The world's folder as it was when the request came.
    world: PathBuf,
    /// The dimension's region folder.
    folder: PathBuf,
    roof: Roof,
    /// The revision of every region under the asked piece that exists.
    revisions: HashMap<(i32, i32), String>,
    /// Read from `level.dat` once, when a region is first painted; `None` without one.
    legacy: OnceCell<Arc<Option<LegacyIds>>>,
    given_up: Arc<AtomicBool>,
    #[cfg(test)]
    pause: Option<Pause>,
}

impl Painter {
    /// The piece, from the store when it holds the current one; `None` over no region.
    fn piece(
        self: &Arc<Self>,
        scale: u8,
        x: i32,
        z: i32,
    ) -> BoxFuture<'_, anyhow::Result<Option<Piece>>> {
        Box::pin(async move {
            let Some(revision) = self.revision(scale, x, z) else {
                return Ok(None);
            };
            let kept = {
                let (this, revision) = (self.clone(), revision.clone());
                spawn_blocking(move || this.store.get(&this.key(scale, x, z, &revision))).await?
            };
            if let Some(png) = kept {
                return Ok(Some(Piece::Kept(png)));
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

    /// The revision of the piece: its region's at scale 0, else a digest of the revisions of
    /// the regions under it. `None` when no region is under it.
    fn revision(&self, scale: u8, x: i32, z: i32) -> Option<String> {
        if scale == 0 {
            return self.revisions.get(&(x, z)).cloned();
        }
        let (xs, zs) = regions_under(scale, x, z)?;
        let mut digest = Sha256::new();
        let mut any = false;
        for z in zs {
            for x in xs.clone() {
                if let Some(revision) = self.revisions.get(&(x, z)) {
                    digest.update(format!("{x},{z}:{revision};"));
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
        let legacy =
            self.legacy
                .get_or_try_init(|| {
                    let world = self.world.clone();
                    async move {
                        anyhow::Ok(Arc::new(spawn_blocking(move || legacy_ids(&world)).await?))
                    }
                })
                .await?
                .clone();
        // A request given up while it waits here is dropped with its wait.
        let permit = self
            .painting
            .clone()
            .acquire_owned()
            .await
            .expect("the map painting semaphore is never closed");
        let this = self.clone();
        spawn_blocking(move || {
            let _permit = permit;
            this.paint_blocking(x, z, legacy.as_ref().as_ref())
        })
        .await?
    }

    fn paint_blocking(
        &self,
        x: i32,
        z: i32,
        legacy: Option<&LegacyIds>,
    ) -> anyhow::Result<Option<Painted>> {
        self.given_up()?;
        // Each file is read whole and closed before its chunks are parsed.
        let Some(region) = Region::read(&region_file(&self.folder, x, z))? else {
            return Ok(None);
        };
        let mut north_unread = false;
        let north = match z.checked_sub(1) {
            None => None,
            Some(north) => {
                Region::read(&region_file(&self.folder, x, north)).unwrap_or_else(|error| {
                    debug!(?error, "could not read the region north of {x},{z}");
                    north_unread = true;
                    None
                })
            }
        };
        let cancelled = || {
            #[cfg(test)]
            if let Some(pause) = &self.pause {
                pause();
            }
            self.given_up.load(Ordering::Acquire)
        };
        let Some(mut painted) =
            paint_region(&region, north.as_ref(), self.roof, legacy, &cancelled)
        else {
            return Err(anyhow!("the map piece was given up"));
        };
        painted.complete &= !north_unread;
        Ok(Some(painted))
    }

    /// The four pieces of the scale below, joined; a child that could not be had leaves a
    /// hole and the piece incomplete.
    async fn joined(self: &Arc<Self>, scale: u8, x: i32, z: i32) -> anyhow::Result<Painted> {
        let below = scale - 1;
        let (x, z) = (x * 2, z * 2);
        let mut children: [Option<RgbaImage>; 4] = Default::default();
        let mut complete = true;
        for (child, (x, z)) in
            children
                .iter_mut()
                .zip([(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)])
        {
            self.given_up()?;
            match self.piece(below, x, z).await {
                Ok(None) => {}
                Ok(Some(Piece::Painted {
                    image,
                    complete: whole,
                    ..
                })) => {
                    *child = Some(image);
                    complete &= whole;
                }
                Ok(Some(Piece::Kept(png))) => {
                    let decoded = spawn_blocking(move || {
                        image::load_from_memory_with_format(&png, ImageFormat::Png)
                    })
                    .await?;
                    match decoded {
                        Ok(image) => *child = Some(image.into_rgba8()),
                        Err(error) => {
                            debug!(?error, "a kept map piece could not be read");
                            complete = false;
                        }
                    }
                }
                Err(error) => {
                    self.given_up()?;
                    debug!(?error, "a map piece at scale {below} could not be painted");
                    complete = false;
                }
            }
        }
        let image = spawn_blocking(move || join(children.each_ref().map(Option::as_ref))).await?;
        Ok(Painted { image, complete })
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
    ) -> anyhow::Result<Piece> {
        let Painted { image, complete } = painted;
        let world_there = fs::symlink_metadata(&self.world).is_ok_and(|meta| meta.is_dir());
        if !complete || !world_there {
            return Ok(Piece::Painted {
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
        Ok(Piece::Painted {
            image,
            png: Some(png),
            complete,
        })
    }
}

/// What the world's `level.dat` says of its numeric ids; `None` when there is none to read.
fn legacy_ids(world: &Path) -> Option<LegacyIds> {
    let path = world.join("level.dat");
    if !fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_file()) {
        return None;
    }
    let nbt = inflate_level_dat(&path).ok()?;
    Some(LegacyIds::from_level_dat(&nbt))
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

/// An instance with the world `Home` holding a region at `(0, 0)` with one chunk, for tests
/// of what serves pieces.
#[cfg(test)]
pub(crate) async fn test_world(app: &crate::managers::App, instance: &str) -> InstanceId {
    let (id, world) = tests::world(app, instance).await;
    tests::put_region(&world, 0, 0, &[tests::ground(0, 0, "minecraft:stone")]);
    id
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Condvar, Mutex};
    use std::time::{Duration, SystemTime};

    use image::{ImageFormat, RgbaImage};

    use super::super::super::mods::tests::addon_fixture;
    use super::super::super::world_level::level_dat_bytes;
    use super::super::chunk::tests::{FULL, chunk_1_18, fill};
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

    async fn piece(app: &App, request: PieceRequest) -> Option<Vec<u8>> {
        app.instance_manager()
            .world_map_piece(request)
            .await
            .unwrap()
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
            let gate = Arc::new(Gate::default());
            let held = gate.clone();
            app.instance_manager()
                .world_map_pause
                .set(Arc::new(move || held.pass()));
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
    ) -> tokio::task::JoinHandle<anyhow::Result<Option<Vec<u8>>>> {
        let app = app.clone();
        tokio::spawn(async move { app.instance_manager().world_map_piece(request).await })
    }

    #[tokio::test]
    async fn a_piece_is_painted_then_served_from_the_store() {
        let app = crate::setup_managers_for_test().await;
        let (id, world) = world(&app, "map-served").await;
        let region = put_region(&world, 0, 0, &[ground(0, 0, "minecraft:stone")]);

        let painted = piece(&app, request(id, 0, 0, 0)).await.expect("a piece");
        let image = decoded(&painted);
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
        let served = piece(&app, request(id, 0, 0, 0)).await;
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
        let answered = decoded(&piece(&app, request(id, 0, 0, 0)).await.unwrap());
        assert_eq!(answered.get_pixel(0, 0)[3], 255);
        assert!(piece(&app, request(id, 1, 0, 0)).await.is_some());
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
    async fn a_world_that_is_not_there_is_refused() {
        let app = crate::setup_managers_for_test().await;
        let id = test_world(&app, "map-gone").await;
        let request = PieceRequest {
            world: "Missing".into(),
            ..request(id, 0, 0, 0)
        };
        let error = app
            .instance_manager()
            .world_map_piece(request)
            .await
            .unwrap_err();
        assert_eq!(
            crate::error::coded_cause(&error).map(|(code, _)| code),
            Some("WORLD_GONE".to_owned())
        );
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
        until("the permit to come back", || {
            painting.available_permits() == PAINTING_AT_ONCE
        })
        .await;
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
}
