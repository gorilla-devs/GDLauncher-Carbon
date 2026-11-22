//! Paints the pieces of a map seen from above: one region at one pixel per block, shaded by
//! height as the game's own map item is, and four pieces joined at half scale.

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage};

use super::chunk::{Roof, Top, surface_checked};
use super::legacy::LegacyIds;
use super::region::Region;

/// The side of a piece in pixels, at every scale.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) const PIECE: u32 = 512;

const SIDE: usize = PIECE as usize;

/// The game's three shades of a map colour, out of 255: a column higher than the one to its
/// north, as high, and lower.
const HIGHER: u16 = 255;
const EQUAL: u16 = 220;
const LOWER: u16 = 180;

#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct Painted {
    pub image: RgbaImage,
    /// Every chunk the region lists was read; a piece that is not is painted again later.
    pub complete: bool,
}

/// One region at one pixel per block, pixel `(x, z)` being the region's block `(x, z)`;
/// `north` is the region whose last row lies north of its first. `None` when `cancelled`
/// says so, which is asked before each chunk.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn paint_region(
    region: &Region,
    north: Option<&Region>,
    roof: Roof,
    legacy: Option<&LegacyIds>,
    cancelled: &dyn Fn() -> bool,
) -> Option<Painted> {
    let mut tops: Vec<Option<Top>> = vec![None; SIDE * SIDE];
    let mut complete = true;
    for cz in 0..32 {
        for cx in 0..32 {
            if cancelled() {
                return None;
            }
            if !region.lists_chunk(cx, cz) {
                continue;
            }
            // A listed chunk that cannot be found is damaged or not written yet.
            let read = region.chunk(cx, cz);
            let Some(Ok(surface)) = read.map(|nbt| surface_checked(&nbt, roof, legacy)) else {
                complete = false;
                continue;
            };
            for (i, top) in surface.into_iter().enumerate() {
                let (x, z) = (usize::from(cx) * 16 + i % 16, usize::from(cz) * 16 + i / 16);
                tops[z * SIDE + x] = top;
            }
        }
    }
    // The last row of the region to the north: a chunk of it that cannot be read is that
    // piece's to report, here it only leaves the row unshaded.
    let mut north_row: Vec<Option<Top>> = vec![None; SIDE];
    if let Some(north) = north {
        for cx in 0..32 {
            if cancelled() {
                return None;
            }
            let Some(Ok(surface)) = north
                .chunk(cx, 31)
                .map(|nbt| surface_checked(&nbt, roof, legacy))
            else {
                continue;
            };
            north_row[usize::from(cx) * 16..][..16].copy_from_slice(&surface[15 * 16..]);
        }
    }
    let mut image = RgbaImage::new(PIECE, PIECE);
    for (i, top) in tops.iter().enumerate() {
        let Some(top) = top else {
            continue;
        };
        let (x, z) = (i % SIDE, i / SIDE);
        let beyond = if z == 0 { north_row[x] } else { tops[i - SIDE] };
        image.put_pixel(x as u32, z as u32, shade(top, beyond));
    }
    Some(Painted { image, complete })
}

/// A column's colour against the column to its north. Water is flat, and so is a column with
/// nothing to its north, so the edge of what was explored is not drawn as a ridge.
fn shade(top: &Top, north: Option<Top>) -> Rgba<u8> {
    let factor = match north {
        _ if top.water => EQUAL,
        Some(north) if top.y > north.y => HIGHER,
        Some(north) if top.y < north.y => LOWER,
        _ => EQUAL,
    };
    let [r, g, b] = top.color.map(|c| (u16::from(c) * factor / 255) as u8);
    Rgba([r, g, b, 255])
}

/// Four pieces, north-west, north-east, south-west and south-east, as one at half scale.
/// Each pixel is the mean of the opaque ones of four, so the edge of what was explored is not
/// darkened; a missing piece is transparent.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn join(children: [Option<&RgbaImage>; 4]) -> RgbaImage {
    let half = PIECE / 2;
    let mut image = RgbaImage::new(PIECE, PIECE);
    for (n, child) in children.into_iter().enumerate() {
        let Some(child) = child else {
            continue;
        };
        let (left, top) = ((n as u32 % 2) * half, (n as u32 / 2) * half);
        for y in 0..half {
            for x in 0..half {
                let (mut sum, mut count) = ([0u32; 3], 0u32);
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let Some(pixel) = child.get_pixel_checked(x * 2 + dx, y * 2 + dy) else {
                        continue;
                    };
                    if pixel[3] != 0 {
                        for (total, channel) in sum.iter_mut().zip(pixel.0) {
                            *total += u32::from(channel);
                        }
                        count += 1;
                    }
                }
                if count > 0 {
                    let [r, g, b] = sum.map(|total| ((total + count / 2) / count) as u8);
                    image.put_pixel(left + x, top + y, Rgba([r, g, b, 255]));
                }
            }
        }
    }
    image
}

/// A piece as PNG, compressed little: pieces are kept, so painting them fast matters more.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn png(image: &RgbaImage) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Fast, FilterType::Adaptive)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use super::super::chunk::tests::{FULL, chunk_1_18, fill};
    use super::super::colors::{Look, Rgb, look_of};
    use super::super::region::test::{region_bytes, region_of, zlib};
    use super::*;

    const CLEAR: Rgba<u8> = Rgba([0, 0, 0, 0]);

    fn color(name: &str) -> Rgb {
        match look_of(name) {
            Look::Color(rgb) | Look::Water(rgb) => rgb,
            Look::SeeThrough => panic!("{name} is see-through"),
        }
    }

    /// `rgb` times `factor / 255`, as the game shades a map.
    fn shaded(rgb: Rgb, factor: u16) -> Rgba<u8> {
        let [r, g, b] = rgb.map(|c| (u16::from(c) * factor / 255) as u8);
        Rgba([r, g, b, 255])
    }

    /// A finished chunk of 1.18 whose column `(x, z)` is `block` from 0 up to `height(x, z)`
    /// (0 to 15), with air above.
    fn hill(block: &str, height: impl Fn(usize, usize) -> usize) -> Vec<u8> {
        let indices = fill(|x, y, z| u16::from(y <= height(x, z)));
        chunk_1_18(vec![(0, vec!["minecraft:air", block], indices)], FULL)
    }

    /// A region holding each chunk's NBT at its slot.
    fn region(chunks: Vec<(u8, u8, Vec<u8>)>) -> Region {
        let stored: Vec<_> = chunks
            .into_iter()
            .map(|(x, z, nbt)| (x, z, 2, zlib(&nbt)))
            .collect();
        region_of(region_bytes(&stored))
    }

    fn never() -> bool {
        false
    }

    fn paint(region: &Region, north: Option<&Region>) -> Painted {
        paint_region(region, north, Roof::Open, None, &never).expect("not cancelled")
    }

    /// Checks every pixel of a piece: `expected` gives the pixels of the chunk at `(cx, cz)`
    /// by their place `(x, z)` in it, or `None` for a chunk that must be transparent.
    fn assert_piece(image: &RgbaImage, expected: impl Fn(u32, u32, u32, u32) -> Option<Rgba<u8>>) {
        assert_eq!(image.dimensions(), (PIECE, PIECE));
        for (px, pz, pixel) in image.enumerate_pixels() {
            let want = expected(px / 16, pz / 16, px % 16, pz % 16).unwrap_or(CLEAR);
            assert_eq!(*pixel, want, "pixel {px},{pz}");
        }
    }

    #[test]
    fn a_region_is_painted_one_pixel_per_block() {
        let stone = color("minecraft:stone");
        let grass = color("minecraft:grass_block");
        let region = region(vec![
            (0, 0, hill("minecraft:stone", |_, _| 5)),
            (3, 2, hill("minecraft:grass_block", |_, _| 7)),
        ]);
        let painted = paint(&region, None);
        assert!(painted.complete);
        assert_piece(&painted.image, |cx, cz, _, _| match (cx, cz) {
            (0, 0) => Some(shaded(stone, EQUAL)),
            (3, 2) => Some(shaded(grass, EQUAL)),
            _ => None,
        });
    }

    #[test]
    fn slopes_are_lighter_and_darker() {
        let stone = color("minecraft:stone");
        let region = region(vec![
            // Rising to the south: every row but the first is above the one to its north.
            (0, 0, hill("minecraft:stone", |_, z| z)),
            // Falling to the south.
            (1, 0, hill("minecraft:stone", |_, z| 15 - z)),
            (2, 0, hill("minecraft:stone", |_, _| 9)),
            // Its first row lies under the last row of the rising chunk, at 15.
            (0, 1, hill("minecraft:stone", |_, _| 10)),
            // East to west slopes do not shade.
            (3, 0, hill("minecraft:stone", |x, _| x)),
        ]);
        let painted = paint(&region, None);
        assert_piece(&painted.image, |cx, cz, _, z| {
            let factor = match (cx, cz, z) {
                (0..=3, 0, 0) => EQUAL,
                (0, 0, _) => HIGHER,
                (1, 0, _) => LOWER,
                (2 | 3, 0, _) => EQUAL,
                (0, 1, 0) => LOWER,
                (0, 1, _) => EQUAL,
                _ => return None,
            };
            Some(shaded(stone, factor))
        });
    }

    #[test]
    fn the_first_row_uses_the_region_to_the_north() {
        let stone = color("minecraft:stone");
        let region = region(vec![(4, 0, hill("minecraft:stone", |_, _| 8))]);
        // Its last row is at 3 for x below 8, at 8 for x 8 to 11 and at 12 above.
        let north = self::region(vec![
            (
                4,
                31,
                hill("minecraft:stone", |x, z| match (x, z) {
                    (_, 0..=14) => 15,
                    (0..=7, _) => 3,
                    (8..=11, _) => 8,
                    _ => 12,
                }),
            ),
            (5, 31, hill("minecraft:stone", |_, _| 0)),
        ]);
        let with_north = paint(&region, Some(&north));
        assert!(with_north.complete);
        assert_piece(&with_north.image, |cx, cz, x, z| {
            let factor = match (cx, cz, x, z) {
                (4, 0, 0..=7, 0) => HIGHER,
                (4, 0, 8..=11, 0) => EQUAL,
                (4, 0, _, 0) => LOWER,
                (4, 0, _, _) => EQUAL,
                _ => return None,
            };
            Some(shaded(stone, factor))
        });

        let alone = paint(&region, None);
        assert_piece(&alone.image, |cx, cz, _, _| {
            ((cx, cz) == (4, 0)).then(|| shaded(stone, EQUAL))
        });
    }

    #[test]
    fn water_is_flat() {
        let water = color("minecraft:water");
        let stone = color("minecraft:stone");
        let region = region(vec![
            (0, 0, hill("minecraft:water", |_, z| z)),
            // Below water rising to 15: land is still shaded against the water's height.
            (0, 1, hill("minecraft:stone", |_, _| 15)),
        ]);
        let painted = paint(&region, None);
        assert_piece(&painted.image, |cx, cz, _, _| match (cx, cz) {
            (0, 0) => Some(shaded(water, EQUAL)),
            (0, 1) => Some(shaded(stone, EQUAL)),
            _ => None,
        });
        let falling = self::region(vec![(0, 0, hill("minecraft:water", |_, z| 15 - z))]);
        assert_piece(&paint(&falling, None).image, |cx, cz, _, _| {
            ((cx, cz) == (0, 0)).then(|| shaded(water, EQUAL))
        });
    }

    #[test]
    fn a_chunk_in_the_last_slot_fills_the_last_pixels() {
        let stone = color("minecraft:stone");
        let region = region(vec![(31, 31, hill("minecraft:stone", |_, _| 5))]);
        let painted = paint(&region, None);
        assert!(painted.complete);
        assert_eq!(painted.image.get_pixel(511, 511), &shaded(stone, EQUAL));
        assert_piece(&painted.image, |cx, cz, _, _| {
            ((cx, cz) == (31, 31)).then(|| shaded(stone, EQUAL))
        });
    }

    #[test]
    fn the_first_row_takes_the_last_row_of_the_chunk_above_it() {
        let stone = color("minecraft:stone");
        let region = region(vec![
            (4, 0, hill("minecraft:stone", |_, _| 8)),
            (5, 0, hill("minecraft:stone", |_, _| 8)),
            (6, 0, hill("minecraft:stone", |_, _| 8)),
        ]);
        let north = self::region(vec![
            // Every row is higher than 8: a wrong chunk of the row would show as lower.
            (4, 31, hill("minecraft:stone", |_, _| 15)),
            // Row 14 is higher, row 15 is lower.
            (
                5,
                31,
                hill("minecraft:stone", |_, z| if z == 15 { 3 } else { 15 }),
            ),
            (
                6,
                31,
                hill("minecraft:stone", |_, z| if z == 15 { 8 } else { 0 }),
            ),
        ]);
        let painted = paint(&region, Some(&north));
        assert_piece(&painted.image, |cx, cz, _, z| {
            let factor = match (cx, z) {
                (4, 0) => LOWER,
                (5, 0) => HIGHER,
                (6, 0) | (4..=6, _) => EQUAL,
                _ => return None,
            };
            (cz == 0).then(|| shaded(stone, factor))
        });
    }

    /// A finished chunk of 1.18 whose column `(x, z)` has its top block at the world height
    /// `height(x, z)`, between -48 and -17, with stone below it.
    fn deep_hill(height: impl Fn(usize, usize) -> i32) -> Vec<u8> {
        let stone_to = |base: i32| fill(|x, y, z| u16::from(base + y as i32 <= height(x, z)));
        chunk_1_18(
            vec![
                (-3, vec!["minecraft:air", "minecraft:stone"], stone_to(-48)),
                (-2, vec!["minecraft:air", "minecraft:stone"], stone_to(-32)),
            ],
            FULL,
        )
    }

    #[test]
    fn heights_below_zero_compare_as_signed() {
        let stone = color("minecraft:stone");
        for (rising, first, later) in [(true, -40, -30), (false, -30, -40)] {
            let region = region(vec![(
                0,
                0,
                deep_hill(|_, z| if z == 0 { first } else { later }),
            )]);
            let painted = paint(&region, None);
            assert_piece(&painted.image, |cx, cz, _, z| {
                if (cx, cz) != (0, 0) {
                    return None;
                }
                let factor = match (z, rising) {
                    (1, true) => HIGHER,
                    (1, false) => LOWER,
                    _ => EQUAL,
                };
                Some(shaded(stone, factor))
            });
        }
    }

    #[test]
    fn an_unfinished_chunk_to_the_north_leaves_the_row_unshaded() {
        let stone = color("minecraft:stone");
        let region = region(vec![(4, 0, hill("minecraft:stone", |_, _| 8))]);
        let unfinished = chunk_1_18(
            vec![(0, vec!["minecraft:stone"], vec![])],
            "minecraft:features",
        );
        let north = self::region(vec![(4, 31, unfinished)]);
        let painted = paint(&region, Some(&north));
        assert!(painted.complete);
        assert_piece(&painted.image, |cx, cz, _, _| {
            ((cx, cz) == (4, 0)).then(|| shaded(stone, EQUAL))
        });
    }

    #[test]
    fn land_is_shaded_against_water_to_its_north() {
        let stone = color("minecraft:stone");
        let north = region(vec![(0, 31, hill("minecraft:water", |_, _| 5))]);
        for (height, factor) in [(8, HIGHER), (5, EQUAL), (3, LOWER)] {
            let land = self::region(vec![(0, 0, hill("minecraft:stone", |_, _| height))]);
            let painted = paint(&land, Some(&north));
            assert_piece(&painted.image, |cx, cz, _, z| {
                ((cx, cz) == (0, 0)).then(|| shaded(stone, if z == 0 { factor } else { EQUAL }))
            });
        }
    }

    #[test]
    fn a_chunk_listed_beyond_the_end_of_the_file_makes_the_piece_incomplete() {
        let stone = color("minecraft:stone");
        let mut bytes = region_bytes(&[(0, 0, 2, zlib(&hill("minecraft:stone", |_, _| 4)))]);
        // Slot (1, 0) lists sector 500 of a file of a few sectors, as a region still being
        // extended or a truncated copy does.
        let at = 4;
        bytes[at..at + 3].copy_from_slice(&500u32.to_be_bytes()[1..]);
        bytes[at + 3] = 1;
        let region = region_of(bytes);
        assert!(!region.has_chunk(1, 0));
        assert!(region.lists_chunk(1, 0));
        let painted = paint(&region, None);
        assert!(!painted.complete);
        assert_piece(&painted.image, |cx, cz, _, _| {
            ((cx, cz) == (0, 0)).then(|| shaded(stone, EQUAL))
        });
    }

    #[test]
    fn an_unreadable_chunk_makes_the_piece_incomplete_but_painted() {
        let stone = color("minecraft:stone");
        let good = || (0, 0, 2, zlib(&hill("minecraft:stone", |_, _| 4)));
        let painted_stone = |cx, cz, _, _| ((cx, cz) == (0, 0)).then(|| shaded(stone, EQUAL));

        // A stream that does not inflate, and NBT that is no chunk.
        for bad in [
            (1, 0, 2, b"not zlib".to_vec()),
            (1, 0, 2, zlib(b"not nbt at all")),
        ] {
            let region = region_of(region_bytes(&[good(), bad]));
            let painted = paint(&region, None);
            assert!(!painted.complete);
            assert_piece(&painted.image, painted_stone);
        }

        // The game has not finished this chunk yet: nothing is wrong.
        let unfinished = chunk_1_18(
            vec![(0, vec!["minecraft:stone"], vec![])],
            "minecraft:features",
        );
        let region = region_of(region_bytes(&[good(), (1, 0, 2, zlib(&unfinished))]));
        let painted = paint(&region, None);
        assert!(painted.complete);
        assert_piece(&painted.image, painted_stone);

        // A broken chunk of the region to the north is that piece's own.
        let north = region_of(region_bytes(&[(0, 31, 2, b"not zlib".to_vec())]));
        let region = region_of(region_bytes(&[good()]));
        assert!(paint(&region, Some(&north)).complete);
    }

    #[test]
    fn cancel_stops_the_painting() {
        let region = region(vec![
            (0, 0, hill("minecraft:stone", |_, _| 4)),
            (1, 0, hill("minecraft:stone", |_, _| 4)),
        ]);
        let polls = Cell::new(0);
        let after_first_chunk = || {
            polls.set(polls.get() + 1);
            polls.get() > 1
        };
        let painted = paint_region(&region, None, Roof::Open, None, &after_first_chunk);
        assert!(painted.is_none());
        assert_eq!(polls.get(), 2, "it stops at once");
    }

    /// A piece of one colour, with its `(x, y)` pixels given by `at` where it returns some.
    fn piece(at: impl Fn(u32, u32) -> Option<Rgba<u8>>) -> RgbaImage {
        RgbaImage::from_fn(PIECE, PIECE, |x, y| at(x, y).unwrap_or(CLEAR))
    }

    fn rgb(r: u8, g: u8, b: u8) -> Rgba<u8> {
        Rgba([r, g, b, 255])
    }

    #[test]
    fn four_pieces_join_at_half_size() {
        let nw = piece(|x, y| match (x, y) {
            (0, 0) => Some(rgb(10, 20, 30)),
            (1, 0) => Some(rgb(20, 30, 40)),
            (0, 1) => Some(rgb(30, 40, 50)),
            (1, 1) => Some(rgb(40, 50, 61)),
            // Two opaque of four: the transparent two do not darken them.
            (2, 0) => Some(rgb(100, 100, 100)),
            (3, 1) => Some(rgb(201, 0, 50)),
            _ => None,
        });
        let ne = piece(|x, y| ((x, y) == (511, 511)).then(|| rgb(7, 8, 9)));
        let se = piece(|_, _| Some(rgb(1, 2, 3)));
        let joined = join([Some(&nw), Some(&ne), None, Some(&se)]);
        assert_eq!(joined.dimensions(), (PIECE, PIECE));
        for (x, y, pixel) in joined.enumerate_pixels() {
            let want = match (x, y) {
                (0, 0) => rgb(25, 35, 45),
                (1, 0) => rgb(151, 50, 75),
                (511, 255) => rgb(7, 8, 9),
                (256.., 256..) => rgb(1, 2, 3),
                _ => CLEAR,
            };
            assert_eq!(*pixel, want, "pixel {x},{y}");
        }
    }

    #[test]
    fn a_png_round_trips() {
        let image = piece(|x, y| (x % 3 != 0).then(|| rgb(x as u8, y as u8, (x ^ y) as u8)));
        let bytes = png(&image).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        assert_eq!(decoded, image);
    }

    /// The overworld region folder of a real world, of either layout.
    fn overworld_regions(world: &Path) -> PathBuf {
        let newer = world.join("dimensions/minecraft/overworld/region");
        if newer.is_dir() {
            newer
        } else {
            world.join("region")
        }
    }

    /// The `(x, z)` of a file named `r.<x>.<z>.mca`.
    fn region_position(path: &Path) -> Option<(i32, i32)> {
        let name = path.file_name()?.to_str()?;
        let mut parts = name.strip_prefix("r.")?.strip_suffix(".mca")?.split('.');
        let position = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
        parts.next().is_none().then_some(position)
    }

    fn level_dat(world: &Path) -> Option<LegacyIds> {
        use std::io::Read;
        let bytes = std::fs::read(world.join("level.dat")).ok()?;
        let mut nbt = Vec::new();
        flate2::read::GzDecoder::new(&bytes[..])
            .read_to_end(&mut nbt)
            .ok()?;
        Some(LegacyIds::from_level_dat(&nbt))
    }

    /// Runs only with `GDL_REAL_WORLDS` set to the folder of real worlds and `GDL_MAP_OUT` to
    /// a folder for the pictures.
    #[test]
    fn real_worlds_are_painted() {
        let (Some(root), Some(out)) = (
            std::env::var_os("GDL_REAL_WORLDS"),
            std::env::var_os("GDL_MAP_OUT"),
        ) else {
            return;
        };
        let (root, out) = (PathBuf::from(root), PathBuf::from(out));
        let mut worlds: Vec<PathBuf> = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_dir())
            .collect();
        worlds.sort();
        assert!(!worlds.is_empty(), "no worlds under GDL_REAL_WORLDS");
        for world in worlds {
            let name = world.file_name().unwrap().to_string_lossy().into_owned();
            let folder = overworld_regions(&world);
            let legacy = level_dat(&world);
            let file = |x: i32, z: i32| folder.join(format!("r.{x}.{z}.mca"));
            let mut positions: Vec<(i32, i32)> = std::fs::read_dir(&folder)
                .unwrap()
                .filter_map(|entry| region_position(&entry.unwrap().path()))
                .collect();
            positions.sort();
            let world_out = out.join(&name);
            std::fs::create_dir_all(&world_out).unwrap();
            let mut pieces = std::collections::HashMap::new();
            let (mut opaque, mut flags, mut times) = (0, Vec::new(), Vec::new());
            for &(x, z) in &positions {
                // The game leaves empty files for regions it only looked at.
                let Some(region) = Region::read(&file(x, z)).unwrap() else {
                    flags.push(format!("r.{x}.{z}=no region"));
                    continue;
                };
                let north = Region::read(&file(x, z - 1)).unwrap();
                let start = Instant::now();
                let painted =
                    paint_region(&region, north.as_ref(), Roof::Open, legacy.as_ref(), &never)
                        .unwrap();
                times.push(start.elapsed());
                opaque += painted.image.pixels().filter(|p| p[3] != 0).count();
                flags.push(format!("r.{x}.{z}={}", painted.complete));
                let bytes = png(&painted.image).unwrap();
                std::fs::write(world_out.join(format!("r.{x}.{z}.png")), bytes).unwrap();
                pieces.insert((x, z), painted.image);
            }
            // The first block of four neighbouring regions, joined as one piece of scale 1.
            let joined = positions.iter().find_map(|&(x, z)| {
                let children = [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)];
                let children = children.map(|at| pieces.get(&at));
                children
                    .iter()
                    .all(Option::is_some)
                    .then(|| (x, z, join(children)))
            });
            if let Some((x, z, image)) = &joined {
                let bytes = png(image).unwrap();
                std::fs::write(world_out.join(format!("joined.{x}.{z}.png")), bytes).unwrap();
            }
            let millis: Vec<String> = times
                .iter()
                .map(|t| format!("{:.1}", t.as_secs_f64() * 1000.0))
                .collect();
            println!(
                "{name}: {} regions painted, {opaque} opaque pixels, complete [{}], ms per region [{}], joined {}",
                times.len(),
                flags.join(" "),
                millis.join(" "),
                joined.map_or("none".to_string(), |(x, z, _)| format!("from r.{x}.{z}")),
            );
        }
    }
}
