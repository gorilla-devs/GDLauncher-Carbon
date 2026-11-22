//! The top visible block of each of a chunk's 256 columns, for a map seen from above.
//! Worlds come from anywhere: a chunk that does not hold together gives an empty
//! surface, never a panic or a guess.

use std::ops::RangeInclusive;

use super::colors::{Look, Rgb, look_of};
use crate::util::nbt::{self, Keep, Limits, Tag};

/// Walking a chunk costs at most this, skipped entities and ticks included.
const LIMITS: Limits = Limits {
    max_tags: 2_000_000,
    max_depth: 64,
};

const SECTION_BLOCKS: usize = 4096;

/// The sections a world can have: the game allows heights from -2032 to 2031.
const SECTION_YS: RangeInclusive<i8> = -127..=126;

/// A chunk whose top block is bedrock in this many columns is taken to have a roof.
const ROOF_COLUMNS: usize = 200;

const EMPTY: Surface = [None; 256];

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Top {
    pub color: Rgb,
    pub y: i16,
    pub water: bool,
}

/// Indexed `z * 16 + x`; `None` where the column shows nothing.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) type Surface = [Option<Top>; 256];

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Roof {
    Open,
    /// The dimension has a roof: show what lies under it.
    Ceiling,
}

/// What a world of 1.2 to 1.12.2 says about its numeric block ids.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct LegacyIds {}

/// The top visible block of each column of a chunk's uncompressed NBT, of 1.13 and later.
/// A chunk of a shape this reader does not know, an unfinished one or one that does not hold
/// together is empty.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn surface(chunk_nbt: &[u8], roof: Roof, _legacy: Option<&LegacyIds>) -> Surface {
    let Ok(root) = nbt::read_keeping(chunk_nbt, &keep, LIMITS) else {
        return EMPTY;
    };
    // Each chunk is read by its own shape: a region upgraded in part holds both.
    let column = if let Some(sections) = root.tag.get("sections") {
        if !finished(root.tag.get("Status"), FINISHED) {
            return EMPTY;
        }
        let Tag::List { items, .. } = sections else {
            return EMPTY;
        };
        Column::new(items, Section::read)
    } else if let Some(level) = root.tag.get("Level") {
        if !finished(level.get("Status"), FINISHED_LEVEL) {
            return EMPTY;
        }
        let Some(Tag::List { items, .. }) = level.get("Sections") else {
            return EMPTY;
        };
        Column::new(items, Section::read_level)
    } else {
        return EMPTY;
    };
    let Some(column) = column else {
        return EMPTY;
    };
    let open = std::array::from_fn(|i| column.top(i, Roof::Open));
    let bedrock = open
        .iter()
        .flatten()
        .filter(|(_, bedrock)| *bedrock)
        .count();
    if roof == Roof::Ceiling || bedrock >= ROOF_COLUMNS {
        std::array::from_fn(|i| column.top(i, Roof::Ceiling).map(|(top, _)| top))
    } else {
        open.map(|top: Option<(Top, bool)>| top.map(|(top, _)| top))
    }
}

/// The statuses of a finished chunk of 1.18 and later.
const FINISHED: &[&[u8]] = &[b"minecraft:full", b"full"];

/// The same for 1.13 to 1.17, where 1.13 has names of its own.
const FINISHED_LEVEL: &[&[u8]] = &[b"minecraft:full", b"full", b"postprocessed", b"fullchunk"];

/// A chunk of either shape without a status is read as finished.
fn finished(status: Option<&Tag>, names: &[&[u8]]) -> bool {
    match status {
        None => true,
        Some(Tag::String(status)) => names.contains(&status.as_slice()),
        Some(_) => false,
    }
}

/// Keeps the block data and status of each shape of chunk and walks past the rest.
fn keep(path: &[&[u8]]) -> Keep {
    match path {
        [b"sections"] | [b"sections", b"block_states"] => Keep::Descend,
        [b"sections", b"Y"] | [b"sections", b"block_states", b"palette" | b"data"] => Keep::All,
        [b"Level"] | [b"Level", b"Sections"] => Keep::Descend,
        [
            b"Level",
            b"Sections",
            b"Y" | b"Palette" | b"BlockStates" | b"Blocks" | b"Data" | b"Add",
        ] => Keep::All,
        [b"Status"] | [b"Level", b"Status"] => Keep::All,
        _ => Keep::Skip,
    }
}

#[derive(Clone, Copy)]
struct Block {
    look: Look,
    bedrock: bool,
}

const AIR: Block = Block {
    look: Look::SeeThrough,
    bedrock: false,
};

enum Section {
    Uniform(Block),
    Mixed {
        palette: Vec<Block>,
        /// One per block, in the game's order `y * 256 + z * 16 + x`, each within `palette`.
        indices: Vec<u16>,
    },
}

/// The block name of a palette entry: `{Name, Properties}` in 1.18 to 1.21; newer games (26.3
/// worlds) write a bare name, `{id, properties}`, or `{"": name}` in a list mixing the two.
fn palette_name(entry: &Tag) -> Option<&[u8]> {
    let name = match entry {
        Tag::String(_) => entry,
        Tag::Compound(_) => entry
            .get("Name")
            .or_else(|| entry.get("id"))
            .or_else(|| entry.get(""))?,
        _ => return None,
    };
    match name {
        Tag::String(raw) => Some(raw),
        _ => None,
    }
}

/// How a section's indices lie in its longs.
#[derive(Clone, Copy)]
enum Packing {
    /// Each long holds `64 / bits` whole indices and leaves its top bits unused (1.16 on).
    Padded,
    /// The indices run on from one long into the next (1.13 to 1.15).
    Spanning,
}

/// A palette of 1 to 4096 entries, so an index has at most 12 bits.
fn palette(items: &[Tag]) -> Option<Vec<Block>> {
    if items.is_empty() || items.len() > SECTION_BLOCKS {
        return None;
    }
    items
        .iter()
        .map(|entry| {
            let name = nbt::decode_string(palette_name(entry)?)?;
            Some(Block {
                look: look_of(&name),
                bedrock: name == "minecraft:bedrock",
            })
        })
        .collect()
}

/// The bits of one index into a palette of `len` entries, at least 4 as the game writes.
fn bits_per_index(len: usize) -> usize {
    ((usize::BITS - len.saturating_sub(1).leading_zeros()) as usize).max(4)
}

/// The 4096 indices packed in `data`, or `None` when the longs run out or an index is past
/// the palette.
fn unpack(palette_len: usize, data: &[i64], packing: Packing) -> Option<Vec<u16>> {
    let bits = bits_per_index(palette_len);
    let mask = (1u64 << bits) - 1;
    let per_long = 64 / bits;
    (0..SECTION_BLOCKS)
        .map(|i| {
            let index = match packing {
                Packing::Padded => {
                    (*data.get(i / per_long)? as u64 >> ((i % per_long) * bits)) & mask
                }
                Packing::Spanning => {
                    let (long, offset) = (i * bits / 64, i * bits % 64);
                    let mut index = *data.get(long)? as u64 >> offset;
                    if offset + bits > 64 {
                        // `offset` is above 64 - bits here, so the shift is 1 to 11.
                        index |= (*data.get(long + 1)? as u64) << (64 - offset);
                    }
                    index & mask
                }
            };
            (index < palette_len as u64).then_some(index as u16)
        })
        .collect()
}

/// A section that does not hold together. Its whole chunk is left empty: read as air, it would
/// show the blocks under it as the top.
struct IllFormed;

/// A section read: `None` when it holds no blocks at all and is air.
type SectionRead = Result<Option<Section>, IllFormed>;

impl Section {
    /// One section of 1.18 and later. Data longer than its palette needs is read as the game
    /// would; shorter is ill-formed.
    fn read(section: &Tag) -> SectionRead {
        match section.get("block_states") {
            None => Ok(None),
            Some(states @ Tag::Compound(_)) => {
                Section::decode(states.get("palette"), states.get("data"), |_, _| {
                    Some(Packing::Padded)
                })
            }
            Some(_) => Err(IllFormed),
        }
    }

    /// One section of 1.13 to 1.17, whose packing the length of its longs tells.
    fn read_level(section: &Tag) -> SectionRead {
        Section::decode(
            section.get("Palette"),
            section.get("BlockStates"),
            |bits, len| {
                // Where 64 is a multiple of `bits` the two lengths are equal and so are the
                // layouts.
                if len == SECTION_BLOCKS.div_ceil(64 / bits) {
                    Some(Packing::Padded)
                } else if len == (SECTION_BLOCKS * bits).div_ceil(64) {
                    Some(Packing::Spanning)
                } else {
                    None
                }
            },
        )
    }

    /// A section from its palette and packed longs, either shape. No palette and no longs is
    /// a section with only light; a palette of one needs no longs, but any it has must fit.
    /// `packing` gives the layout for the bits per index and the number of longs, or `None`
    /// when that number fits none.
    fn decode(
        palette: Option<&Tag>,
        data: Option<&Tag>,
        packing: impl Fn(usize, usize) -> Option<Packing>,
    ) -> SectionRead {
        let palette = match (palette, data) {
            (None, None) => return Ok(None),
            (Some(Tag::List { items, .. }), _) => self::palette(items).ok_or(IllFormed)?,
            _ => return Err(IllFormed),
        };
        let data = match data {
            None if palette.len() == 1 => return Ok(Some(Section::Uniform(palette[0]))),
            Some(Tag::LongArray(data)) => data,
            _ => return Err(IllFormed),
        };
        let packing = packing(bits_per_index(palette.len()), data.len()).ok_or(IllFormed)?;
        let indices = unpack(palette.len(), data, packing).ok_or(IllFormed)?;
        Ok(Some(match palette[..] {
            [block] => Section::Uniform(block),
            _ => Section::Mixed { palette, indices },
        }))
    }

    fn block(&self, i: usize) -> Block {
        match self {
            Section::Uniform(block) => *block,
            Section::Mixed { palette, indices } => palette[indices[i] as usize],
        }
    }
}

/// A chunk's sections from the lowest to the highest present, a missing one counting as air.
struct Column {
    lowest: i8,
    sections: Vec<Option<Section>>,
}

impl Column {
    /// `None` when there are no sections at all, a section's `Y` is out of the game's range or
    /// a section is ill-formed.
    fn new(sections: &[Tag], read_section: impl Fn(&Tag) -> SectionRead) -> Option<Column> {
        let mut read: Vec<(i8, &Tag)> = Vec::new();
        let mut seen = [false; 256];
        for section in sections {
            let Some(Tag::Byte(y)) = section.get("Y") else {
                return None;
            };
            if !SECTION_YS.contains(y) {
                return None;
            }
            // The first of two sections at one height is the one kept.
            let slot = &mut seen[(*y as u8) as usize];
            if !*slot {
                *slot = true;
                read.push((*y, section));
            }
        }
        let lowest = read.iter().map(|(y, _)| *y).min()?;
        let highest = read.iter().map(|(y, _)| *y).max()?;
        let mut column: Vec<Option<Section>> = (lowest..=highest).map(|_| None).collect();
        for (y, section) in read {
            column[(i16::from(y) - i16::from(lowest)) as usize] = read_section(section).ok()?;
        }
        Some(Column {
            lowest,
            sections: column,
        })
    }

    /// The top visible block of column `i` (`z * 16 + x`) and whether it is bedrock. Under a
    /// roof the scan passes the roof and the air under it, and takes the first block below.
    fn top(&self, i: usize, roof: Roof) -> Option<(Top, bool)> {
        let mut under_roof = roof == Roof::Open;
        let mut roof_seen = false;
        for (n, section) in self.sections.iter().enumerate().rev() {
            let base = (i16::from(self.lowest) + n as i16) * 16;
            for local in (0..16).rev() {
                let block = section.as_ref().map_or(AIR, |s| s.block(local * 256 + i));
                let (color, water) = match block.look {
                    Look::SeeThrough => {
                        under_roof |= roof_seen;
                        continue;
                    }
                    Look::Color(color) => (color, false),
                    Look::Water(color) => (color, true),
                };
                if under_roof {
                    let y = base + local as i16;
                    return Some((Top { color, y, water }, block.bedrock));
                }
                roof_seen = true;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::colors::{UNKNOWN, look_of};
    use super::super::region::Region;
    use super::*;
    use crate::util::nbt::{self, Keep, Limits, Root, Tag};

    fn compound(entries: Vec<(&str, Tag)>) -> Tag {
        Tag::Compound(
            entries
                .into_iter()
                .map(|(key, tag)| (key.as_bytes().to_vec(), tag))
                .collect(),
        )
    }

    fn string(text: &str) -> Tag {
        Tag::String(text.as_bytes().to_vec())
    }

    fn compounds(items: Vec<Tag>) -> Tag {
        Tag::List {
            element: if items.is_empty() { 0 } else { 10 },
            items,
        }
    }

    fn bits_for(palette_len: usize) -> usize {
        let needed = (usize::BITS - palette_len.saturating_sub(1).leading_zeros()) as usize;
        needed.max(4)
    }

    /// Packs 4096 indices the way the game does from 1.16: no index spans two longs.
    fn pack(indices: &[u16], bits: usize) -> Vec<i64> {
        let per_long = 64 / bits;
        let mut data = vec![0u64; indices.len().div_ceil(per_long)];
        for (i, &index) in indices.iter().enumerate() {
            data[i / per_long] |= (index as u64) << ((i % per_long) * bits);
        }
        data.into_iter().map(|long| long as i64).collect()
    }

    fn section(y: i8, palette: &[&str], data: Option<Vec<i64>>) -> Tag {
        let mut states = vec![(
            "palette",
            compounds(
                palette
                    .iter()
                    .map(|name| compound(vec![("Name", string(name))]))
                    .collect(),
            ),
        )];
        if let Some(data) = data {
            states.push(("data", Tag::LongArray(data)));
        }
        compound(vec![
            ("Y", Tag::Byte(y)),
            ("block_states", compound(states)),
            ("SkyLight", Tag::ByteArray(vec![0; 2048])),
        ])
    }

    fn chunk_with(sections: Vec<Tag>, status: &str) -> Vec<u8> {
        let mut entries = vec![
            ("DataVersion", Tag::Int(3465)),
            ("yPos", Tag::Int(-4)),
            ("sections", compounds(sections)),
            ("block_entities", compounds(vec![])),
        ];
        if !status.is_empty() {
            entries.push(("Status", string(status)));
        }
        nbt::write(&Root {
            name: Vec::new(),
            tag: compound(entries),
        })
    }

    /// A chunk of 1.18 and later; an empty index list means a palette of one with no `data`.
    fn chunk_1_18(sections: Vec<(i8, Vec<&str>, Vec<u16>)>, status: &str) -> Vec<u8> {
        let sections = sections
            .into_iter()
            .map(|(y, palette, indices)| {
                let data = (!indices.is_empty()).then(|| pack(&indices, bits_for(palette.len())));
                section(y, &palette, data)
            })
            .collect();
        chunk_with(sections, status)
    }

    /// 4096 indices in the game's order, `y * 256 + z * 16 + x`.
    fn fill(index_at: impl Fn(usize, usize, usize) -> u16) -> Vec<u16> {
        (0..4096)
            .map(|i| index_at(i % 16, i / 256, (i / 16) % 16))
            .collect()
    }

    fn color(name: &str) -> Rgb {
        match look_of(name) {
            Look::Color(rgb) | Look::Water(rgb) => rgb,
            Look::SeeThrough => panic!("{name} is see-through"),
        }
    }

    fn top(name: &str, y: i16) -> Option<Top> {
        Some(Top {
            color: color(name),
            y,
            water: matches!(look_of(name), Look::Water(_)),
        })
    }

    const FULL: &str = "minecraft:full";

    /// Stone up to 63 and grass at 64 where `x < 8`; air above.
    fn terrain() -> Vec<(i8, Vec<&'static str>, Vec<u16>)> {
        vec![
            (2, vec!["minecraft:stone"], vec![]),
            (
                4,
                vec!["minecraft:air", "minecraft:grass_block"],
                fill(|x, y, _| u16::from(y == 0 && x < 8)),
            ),
            (3, vec!["minecraft:stone"], vec![]),
            (5, vec!["minecraft:air"], vec![]),
        ]
    }

    fn assert_terrain(surface: &Surface) {
        for z in 0..16 {
            for x in 0..16 {
                let expected = if x < 8 {
                    top("minecraft:grass_block", 64)
                } else {
                    top("minecraft:stone", 63)
                };
                assert_eq!(surface[z * 16 + x], expected, "column {x},{z}");
            }
        }
    }

    #[test]
    fn the_top_block_of_each_column_is_found() {
        let bytes = chunk_1_18(terrain(), FULL);
        let surface = surface(&bytes, Roof::Open, None);
        assert_terrain(&surface);
        // The numeric id table is not needed for this shape.
        assert_eq!(
            super::surface(&bytes, Roof::Open, Some(&LegacyIds {})),
            surface
        );
    }

    #[test]
    fn plants_and_air_are_looked_through() {
        let bytes = chunk_1_18(
            vec![(
                4,
                vec![
                    "minecraft:air",
                    "minecraft:grass_block",
                    "minecraft:poppy",
                    "minecraft:stone",
                ],
                fill(|_, y, _| match y {
                    0..=4 => 3,
                    5 => 1,
                    6 => 2,
                    _ => 0,
                }),
            )],
            "full",
        );
        let surface = surface(&bytes, Roof::Open, None);
        assert!(
            surface
                .iter()
                .all(|column| *column == top("minecraft:grass_block", 69))
        );
    }

    #[test]
    fn water_is_marked() {
        let bytes = chunk_1_18(
            vec![(
                3,
                vec!["minecraft:air", "minecraft:sand", "minecraft:water"],
                fill(|_, y, _| match y {
                    0..=5 => 1,
                    6..=12 => 2,
                    _ => 0,
                }),
            )],
            FULL,
        );
        let surface = surface(&bytes, Roof::Open, None);
        let water = surface[0].expect("a top");
        assert!(water.water);
        assert_eq!(water.y, 3 * 16 + 12);
        assert_eq!(water.color, color("minecraft:water"));
        assert!(surface.iter().all(|column| *column == Some(water)));
    }

    #[test]
    fn a_palette_of_one_needs_no_data() {
        let bytes = chunk_1_18(vec![(0, vec!["minecraft:dirt"], vec![])], FULL);
        let surface = surface(&bytes, Roof::Open, None);
        assert!(
            surface
                .iter()
                .all(|column| *column == top("minecraft:dirt", 15))
        );
    }

    #[test]
    fn palette_entries_of_newer_games_are_read() {
        let indices = fill(|x, y, _| if y == 15 { (x % 3) as u16 + 1 } else { 0 });
        let palettes = [
            Tag::List {
                element: 8,
                items: [
                    "minecraft:air",
                    "minecraft:stone",
                    "minecraft:dirt",
                    "minecraft:sand",
                ]
                .map(string)
                .to_vec(),
            },
            compounds(vec![
                compound(vec![("", string("minecraft:air"))]),
                compound(vec![("", string("minecraft:stone"))]),
                compound(vec![
                    ("id", string("minecraft:dirt")),
                    ("properties", compound(vec![("snowy", string("false"))])),
                ]),
                compound(vec![("", string("minecraft:sand"))]),
            ]),
        ];
        for palette in palettes {
            let section = compound(vec![
                ("Y", Tag::Byte(0)),
                (
                    "block_states",
                    compound(vec![
                        ("palette", palette),
                        ("data", Tag::LongArray(pack(&indices, 4))),
                    ]),
                ),
            ]);
            let surface = surface(&chunk_with(vec![section], FULL), Roof::Open, None);
            assert_eq!(surface[0], top("minecraft:stone", 15));
            assert_eq!(surface[1], top("minecraft:dirt", 15));
            assert_eq!(surface[2], top("minecraft:sand", 15));
        }
    }

    #[test]
    fn indices_do_not_span_longs() {
        let palette = [
            "minecraft:air",
            "minecraft:stone",
            "minecraft:dirt",
            "minecraft:sand",
            "minecraft:gravel",
            "minecraft:oak_planks",
            "minecraft:cobblestone",
            "minecraft:bricks",
            "minecraft:white_wool",
            "minecraft:red_wool",
            "minecraft:blue_wool",
            "minecraft:green_wool",
            "minecraft:yellow_wool",
            "minecraft:black_wool",
            "minecraft:orange_wool",
            "minecraft:lime_wool",
            "minecraft:gold_block",
            "minecraft:diamond_block",
            "minecraft:emerald_block",
            "minecraft:lapis_block",
        ];
        assert_eq!(bits_for(palette.len()), 5);
        // 5 bits: 12 indices to a long and 4 bits left over. The top layer of the
        // section starts at index 3840, the first index of long 320, so x = 11 is the
        // last index of that long and x = 12 and 13 open the next one.
        let mut data = vec![0i64; 4096_usize.div_ceil(12)];
        data[320] = 17 << 55;
        data[321] = 18 | (19 << 5);
        let bytes = chunk_with(vec![section(0, &palette, Some(data))], FULL);
        let surface = surface(&bytes, Roof::Open, None);
        assert_eq!(surface[11], top("minecraft:diamond_block", 15));
        assert_eq!(surface[12], top("minecraft:emerald_block", 15));
        assert_eq!(surface[13], top("minecraft:lapis_block", 15));
        let shown = surface.iter().filter(|column| column.is_some()).count();
        assert_eq!(shown, 3);
    }

    #[test]
    fn sections_below_zero_and_above_255_are_read() {
        let one_block =
            |local_y: usize| fill(move |x, y, z| u16::from(x == 3 && z == 5 && y == local_y));
        let below = chunk_1_18(
            vec![(
                -3,
                vec!["minecraft:air", "minecraft:obsidian"],
                one_block(8),
            )],
            FULL,
        );
        let surface_below = surface(&below, Roof::Open, None);
        assert_eq!(surface_below[5 * 16 + 3], top("minecraft:obsidian", -40));
        assert_eq!(surface_below.iter().flatten().count(), 1);

        let above = chunk_1_18(
            vec![(
                18,
                vec!["minecraft:air", "minecraft:obsidian"],
                one_block(12),
            )],
            FULL,
        );
        let surface_above = surface(&above, Roof::Open, None);
        assert_eq!(surface_above[5 * 16 + 3], top("minecraft:obsidian", 300));
        assert_eq!(surface_above.iter().flatten().count(), 1);

        // The lowest and highest sections the game allows, in one chunk.
        let tallest = chunk_1_18(
            vec![
                (-127, vec!["minecraft:stone"], vec![]),
                (
                    126,
                    vec!["minecraft:air", "minecraft:obsidian"],
                    one_block(15),
                ),
            ],
            FULL,
        );
        let surface_tallest = surface(&tallest, Roof::Open, None);
        assert_eq!(surface_tallest[5 * 16 + 3], top("minecraft:obsidian", 2031));
        assert_eq!(surface_tallest[0], top("minecraft:stone", -2017));
    }

    #[test]
    fn sections_out_of_order_or_duplicated() {
        let bytes = chunk_1_18(
            vec![
                (1, vec!["minecraft:stone"], vec![]),
                (6, vec!["minecraft:dirt"], vec![]),
                (-2, vec!["minecraft:deepslate"], vec![]),
                (6, vec!["minecraft:gold_block"], vec![]),
                (7, vec!["minecraft:air"], vec![]),
            ],
            FULL,
        );
        let surface = surface(&bytes, Roof::Open, None);
        assert!(
            surface
                .iter()
                .all(|column| *column == top("minecraft:dirt", 6 * 16 + 15))
        );
    }

    fn names(palette: &[&str]) -> Tag {
        compounds(
            palette
                .iter()
                .map(|name| compound(vec![("Name", string(name))]))
                .collect(),
        )
    }

    /// Palettes and packed data that do not hold together in either shape, as
    /// `(what, palette, packed data)`.
    fn ill_formed_sections() -> Vec<(&'static str, Option<Tag>, Option<Tag>)> {
        let two = ["minecraft:air", "minecraft:stone"];
        let zeros = || Some(Tag::LongArray(vec![0; 256]));
        let mut past_one = vec![0; 256];
        past_one[100] = 1 << 8;
        vec![
            (
                "an index past the palette",
                Some(names(&two)),
                Some(Tag::LongArray(pack(
                    &fill(|x, _, _| u16::from(x == 0) * 2),
                    4,
                ))),
            ),
            (
                "packed data too short",
                Some(names(&two)),
                Some(Tag::LongArray(vec![0; 255])),
            ),
            ("packed data missing", Some(names(&two)), None),
            (
                "packed data not longs",
                Some(names(&two)),
                Some(Tag::IntArray(vec![0; 512])),
            ),
            ("palette missing", None, zeros()),
            ("palette empty", Some(compounds(vec![])), zeros()),
            ("palette not a list", Some(Tag::Int(3)), zeros()),
            (
                "palette of numbers",
                Some(Tag::List {
                    element: 3,
                    items: vec![Tag::Int(1), Tag::Int(2)],
                }),
                zeros(),
            ),
            (
                "palette entry without a name",
                Some(compounds(vec![
                    compound(vec![("Name", string("minecraft:air"))]),
                    compound(vec![("Colour", string("red"))]),
                ])),
                zeros(),
            ),
            (
                "palette of 4097",
                Some(names(&vec!["minecraft:stone"; 4097])),
                Some(Tag::LongArray(vec![0; 64 * 13])),
            ),
            (
                "one block with data of the wrong length",
                Some(names(&["minecraft:stone"])),
                Some(Tag::LongArray(vec![0; 255])),
            ),
            (
                "one block with an index past it",
                Some(names(&["minecraft:stone"])),
                Some(Tag::LongArray(past_one)),
            ),
        ]
    }

    fn assert_empty(bytes: &[u8], what: &str) {
        let surface = surface(bytes, Roof::Open, None);
        assert!(surface.iter().all(Option::is_none), "{what}");
    }

    #[test]
    fn a_chunk_that_lies_is_empty_not_a_panic() {
        let empty = assert_empty;
        // Each bad section lies over good stone, which must not show through it.
        let ground = section(0, &["minecraft:stone"], None);
        for (what, palette, data) in ill_formed_sections() {
            let mut states = Vec::new();
            if let Some(palette) = palette {
                states.push(("palette", palette));
            }
            if let Some(data) = data {
                states.push(("data", data));
            }
            let bad = compound(vec![
                ("Y", Tag::Byte(1)),
                ("block_states", compound(states)),
            ]);
            empty(&chunk_with(vec![ground.clone(), bad], FULL), what);
        }
        let not_compound = compound(vec![("Y", Tag::Byte(1)), ("block_states", Tag::Int(3))]);
        empty(
            &chunk_with(vec![ground.clone(), not_compound], FULL),
            "block_states not a compound",
        );

        for y in [127, -128] {
            let bytes = chunk_1_18(vec![(y, vec!["minecraft:stone"], vec![])], FULL);
            empty(&bytes, &format!("section Y {y}"));
        }

        empty(b"", "nothing");
        empty(b"not nbt at all", "not NBT");
        empty(&[10, 0, 0, 9, 0, 1], "cut short");

        // A valid chunk with 3,000,000 bytes in a list the map never reads.
        let mut many = chunk_1_18(vec![(0, vec!["minecraft:stone"], vec![])], FULL);
        let end = many.pop();
        assert_eq!(end, Some(0));
        many.extend_from_slice(&[9, 0, 4]);
        many.extend_from_slice(b"junk");
        many.push(1);
        many.extend_from_slice(&3_000_000_i32.to_be_bytes());
        many.resize(many.len() + 3_000_000, 0);
        many.push(0);
        empty(&many, "3,000,000 tags");
    }

    #[test]
    fn an_unfinished_chunk_is_empty() {
        let bytes = chunk_1_18(
            vec![(0, vec!["minecraft:stone"], vec![])],
            "minecraft:features",
        );
        let surface = surface(&bytes, Roof::Open, None);
        assert!(surface.iter().all(Option::is_none));
    }

    /// Bedrock roof at 127, netherrack 100 to 126, air 40 to 99. Where `x < 8` netherrack
    /// fills 0 to 39; elsewhere air goes on down to 32, over lava at 31 and netherrack below.
    fn nether() -> Vec<u8> {
        let palette = vec![
            "minecraft:air",
            "minecraft:netherrack",
            "minecraft:bedrock",
            "minecraft:lava",
        ];
        let sections = (0..8)
            .map(|section_y: i8| {
                let indices = fill(|x, local, _| {
                    let y = i32::from(section_y) * 16 + local as i32;
                    match y {
                        127 => 2,
                        100..=126 => 1,
                        40..=99 => 0,
                        _ if x < 8 => 1,
                        32..=39 => 0,
                        31 => 3,
                        _ => 1,
                    }
                });
                (section_y, palette.clone(), indices)
            })
            .collect();
        chunk_1_18(sections, FULL)
    }

    fn assert_floor(surface: &Surface) {
        for z in 0..16 {
            for x in 0..16 {
                let expected = if x < 8 {
                    top("minecraft:netherrack", 39)
                } else {
                    top("minecraft:lava", 31)
                };
                assert_eq!(surface[z * 16 + x], expected, "column {x},{z}");
            }
        }
    }

    #[test]
    fn under_a_ceiling_the_floor_is_shown() {
        let bytes = nether();
        assert_floor(&surface(&bytes, Roof::Ceiling, None));
        // A flat sheet of bedrock on top is taken for a roof.
        assert_floor(&surface(&bytes, Roof::Open, None));

        let terrain = chunk_1_18(terrain(), FULL);
        let open = surface(&terrain, Roof::Open, None);
        assert_eq!(open[0], top("minecraft:grass_block", 64));
        assert_eq!(open[15], top("minecraft:stone", 63));
    }

    /// Packs 4096 indices the way 1.13 to 1.15 do: an index may run on into the next long.
    fn pack_spanning(indices: &[u16], bits: usize) -> Vec<i64> {
        let mut data = vec![0u64; (indices.len() * bits).div_ceil(64)];
        for (i, &index) in indices.iter().enumerate() {
            let (long, offset) = (i * bits / 64, i * bits % 64);
            data[long] |= (index as u64) << offset;
            if offset + bits > 64 {
                data[long + 1] |= (index as u64) >> (64 - offset);
            }
        }
        data.into_iter().map(|long| long as i64).collect()
    }

    /// A section of 1.13 to 1.17; no `data` means one with only light, as 1.13 and 1.14 write.
    fn level_section(y: i8, palette: &[&str], data: Option<Vec<i64>>) -> Tag {
        let mut entries = vec![("Y", Tag::Byte(y))];
        if let Some(data) = data {
            let palette = palette
                .iter()
                .map(|name| {
                    let mut entry = vec![("Name", string(name))];
                    if *name == "minecraft:grass_block" {
                        entry.push(("Properties", compound(vec![("snowy", string("false"))])));
                    }
                    compound(entry)
                })
                .collect();
            entries.push(("Palette", compounds(palette)));
            entries.push(("BlockStates", Tag::LongArray(data)));
        }
        entries.push(("BlockLight", Tag::ByteArray(vec![0; 2048])));
        entries.push(("SkyLight", Tag::ByteArray(vec![0; 2048])));
        compound(entries)
    }

    fn level_chunk_with(sections: Vec<Tag>, status: &str) -> Vec<u8> {
        let mut level = vec![
            ("xPos", Tag::Int(0)),
            ("zPos", Tag::Int(0)),
            ("LastUpdate", Tag::Long(1234)),
            ("Sections", compounds(sections)),
            ("Entities", compounds(vec![])),
        ];
        if !status.is_empty() {
            level.push(("Status", string(status)));
        }
        nbt::write(&Root {
            name: Vec::new(),
            tag: compound(vec![
                ("DataVersion", Tag::Int(2586)),
                ("Level", compound(level)),
            ]),
        })
    }

    /// A chunk of 1.13 to 1.17, packed as 1.13 to 1.15 do (`spanning`) or as 1.16 and 1.17
    /// do; an empty index list is a section of one block, written with its zeros as the game
    /// writes them.
    fn chunk_1_13(
        sections: Vec<(i8, Vec<&str>, Vec<u16>)>,
        status: &str,
        spanning: bool,
    ) -> Vec<u8> {
        let sections = sections
            .into_iter()
            .map(|(y, palette, mut indices)| {
                if indices.is_empty() {
                    indices = vec![0; 4096];
                }
                let bits = bits_for(palette.len());
                let data = if spanning {
                    pack_spanning(&indices, bits)
                } else {
                    pack(&indices, bits)
                };
                level_section(y, &palette, Some(data))
            })
            .collect();
        level_chunk_with(sections, status)
    }

    /// Twenty blocks: 5 bits an index, the smallest width where the two packings differ.
    const TWENTY: [&str; 20] = [
        "minecraft:air",
        "minecraft:stone",
        "minecraft:dirt",
        "minecraft:sand",
        "minecraft:gravel",
        "minecraft:oak_planks",
        "minecraft:cobblestone",
        "minecraft:bricks",
        "minecraft:white_wool",
        "minecraft:red_wool",
        "minecraft:blue_wool",
        "minecraft:green_wool",
        "minecraft:yellow_wool",
        "minecraft:black_wool",
        "minecraft:orange_wool",
        "minecraft:lime_wool",
        "minecraft:gold_block",
        "minecraft:diamond_block",
        "minecraft:emerald_block",
        "minecraft:lapis_block",
    ];

    /// Blocks 17, 18 and 19 at x = 11, 12 and 13 of the top layer of a section at `Y` 0.
    fn assert_three_blocks(surface: &Surface) {
        assert_eq!(surface[11], top("minecraft:diamond_block", 15));
        assert_eq!(surface[12], top("minecraft:emerald_block", 15));
        assert_eq!(surface[13], top("minecraft:lapis_block", 15));
        let shown = surface.iter().filter(|column| column.is_some()).count();
        assert_eq!(shown, 3);
    }

    /// Every column a different block of the twenty, at a different height.
    fn mixed_indices() -> Vec<u16> {
        fill(|x, y, z| {
            let i = z * 16 + x;
            if y <= i % 16 { (i % 19 + 1) as u16 } else { 0 }
        })
    }

    /// The surface of a section at `Y` 0 holding `mixed_indices`.
    fn assert_mixed(surface: &Surface) {
        for i in 0..256 {
            assert_eq!(
                surface[i],
                top(TWENTY[i % 19 + 1], (i % 16) as i16),
                "column {i}"
            );
        }
    }

    #[test]
    fn the_spanning_packing_is_read() {
        assert_eq!(bits_for(TWENTY.len()), 5);
        // 5 bits: the top layer starts at bit 3840 * 5 = 19200, the first bit of long 300.
        // x = 11 takes bits 55 to 59 of it, x = 12 its bits 60 to 63 and bit 0 of long 301,
        // x = 13 bits 1 to 5 of long 301.
        let mut data = vec![0i64; 320];
        data[300] = (17 << 55) | ((18 & 0b1111) << 60);
        data[301] = (18 >> 4) | (19 << 1);
        let bytes = level_chunk_with(vec![level_section(0, &TWENTY, Some(data))], "full");
        assert_three_blocks(&surface(&bytes, Roof::Open, None));

        let bytes = chunk_1_13(vec![(0, TWENTY.to_vec(), mixed_indices())], "full", true);
        assert_mixed(&surface(&bytes, Roof::Open, None));
    }

    #[test]
    fn the_spanning_packing_is_read_at_7_and_12_bits() {
        // 7 bits, 448 longs. Index 0 is 70 in bits 0 to 6 of long 0. Index 9 is 99 = 0b1100011
        // and straddles: its bit 0 is bit 63 of long 0, its bits 1 to 6 (49) bits 0 to 5 of
        // long 1. Index 4095 is 85 in bits 57 to 63 of long 447, the last.
        let mut seven = vec![0i64; 448];
        seven[0] = 0x8000_0000_0000_0046_u64 as i64;
        seven[1] = 0x31;
        seven[447] = 0xAA00_0000_0000_0000_u64 as i64;
        // 12 bits, 768 longs. Index 0 is 0xFFF in bits 0 to 11 of long 0. Index 5 is 0xABC and
        // straddles: 0xC in bits 60 to 63 of long 0, 0xAB in bits 0 to 7 of long 1. Index 4095
        // is 0x123 in bits 52 to 63 of long 767, the last.
        let mut twelve = vec![0i64; 768];
        twelve[0] = 0xC000_0000_0000_0FFF_u64 as i64;
        twelve[1] = 0xAB;
        twelve[767] = 0x1230_0000_0000_0000;
        for (len, bits, data, [first, straddling, last], straddling_x) in [
            (128, 7, seven, [70, 99, 85], 9),
            (4096, 12, twelve, [0xFFF, 0xABC, 0x123], 5),
        ] {
            assert_eq!(bits_for(len), bits);
            let named = [
                (first, "minecraft:diamond_block"),
                (straddling, "minecraft:emerald_block"),
                (last, "minecraft:lapis_block"),
            ];
            let palette: Vec<String> = (0..len)
                .map(|i| match named.iter().find(|(at, _)| *at == i) {
                    Some((_, name)) => name.to_string(),
                    None if i == 0 => "minecraft:air".to_string(),
                    None => format!("minecraft:x{i}"),
                })
                .collect();
            let palette: Vec<&str> = palette.iter().map(String::as_str).collect();
            let bytes = level_chunk_with(vec![level_section(0, &palette, Some(data))], "full");
            let surface = surface(&bytes, Roof::Open, None);
            assert_eq!(surface[0], top("minecraft:diamond_block", 0), "{bits} bits");
            assert_eq!(
                surface[straddling_x],
                top("minecraft:emerald_block", 0),
                "{bits} bits"
            );
            assert_eq!(
                surface[255],
                top("minecraft:lapis_block", 15),
                "{bits} bits"
            );
            assert_eq!(surface.iter().flatten().count(), 3, "{bits} bits");
        }
    }

    #[test]
    fn the_padded_packing_is_read() {
        // 5 bits: 12 indices to a long, the top layer from long 320. x = 11 is the last index
        // of that long, x = 12 and 13 open long 321.
        let mut data = vec![0i64; 342];
        data[320] = 17 << 55;
        data[321] = 18 | (19 << 5);
        let bytes = level_chunk_with(vec![level_section(0, &TWENTY, Some(data))], "full");
        assert_three_blocks(&surface(&bytes, Roof::Open, None));

        let bytes = chunk_1_13(vec![(0, TWENTY.to_vec(), mixed_indices())], "full", false);
        assert_mixed(&surface(&bytes, Roof::Open, None));
    }

    #[test]
    fn where_the_packings_agree_either_is_read() {
        // 4 and 8 bits fill a long exactly: both packings are the same longs.
        let sixteen = &TWENTY[..16];
        let unknown: Vec<String> = (0..200).map(|i| format!("minecraft:x{i}")).collect();
        let mut wide: Vec<&str> = vec!["minecraft:air"];
        wide.extend(unknown.iter().map(String::as_str));
        wide.push("minecraft:gold_block");
        assert_eq!(bits_for(sixteen.len()), 4);
        assert_eq!(bits_for(wide.len()), 8);
        let gold = (wide.len() - 1) as u16;
        for spanning in [true, false] {
            let bytes = chunk_1_13(
                vec![
                    (
                        0,
                        sixteen.to_vec(),
                        fill(|x, y, _| if y < 3 { x as u16 } else { 0 }),
                    ),
                    (
                        1,
                        wide.clone(),
                        fill(|x, y, _| if y == 0 && x == 15 { gold } else { 0 }),
                    ),
                ],
                "full",
                spanning,
            );
            let surface = surface(&bytes, Roof::Open, None);
            assert_eq!(surface[0], None);
            assert_eq!(surface[3], top("minecraft:sand", 2));
            assert_eq!(surface[15], top("minecraft:gold_block", 16));
        }
    }

    #[test]
    fn a_section_with_no_blocks_is_air() {
        // Gold at 32 over one column, nothing at all in the section under it, stone below.
        let gold = ["minecraft:air", "minecraft:gold_block"];
        let gold_indices = fill(|x, y, z| u16::from(x == 0 && z == 0 && y == 0));
        let assert_stone_shows = |bytes: &[u8], what: &str| {
            let surface = surface(bytes, Roof::Open, None);
            assert_eq!(surface[0], top("minecraft:gold_block", 32), "{what}");
            assert!(
                surface[1..]
                    .iter()
                    .all(|column| *column == top("minecraft:stone", 15)),
                "{what}"
            );
        };

        let stone = section(0, &["minecraft:stone"], None);
        let top_section = section(2, &gold, Some(pack(&gold_indices, 4)));
        let light_only = compound(vec![
            ("Y", Tag::Byte(1)),
            ("SkyLight", Tag::ByteArray(vec![0; 2048])),
        ]);
        let no_states = compound(vec![
            ("Y", Tag::Byte(1)),
            ("block_states", compound(vec![])),
        ]);
        for (what, middle) in [
            ("light only", light_only),
            ("empty block_states", no_states),
        ] {
            let bytes = chunk_with(vec![stone.clone(), middle, top_section.clone()], FULL);
            assert_stone_shows(&bytes, what);
        }

        let bytes = level_chunk_with(
            vec![
                level_section(0, &["minecraft:stone"], Some(vec![0; 256])),
                level_section(1, &[], None),
                level_section(2, &gold, Some(pack(&gold_indices, 4))),
            ],
            "full",
        );
        assert_stone_shows(&bytes, "light only, older shape");
    }

    #[test]
    fn a_length_that_fits_neither_empties_the_chunk() {
        let indices = fill(|_, y, _| u16::from(y == 0));
        let ground = level_section(0, &["minecraft:stone"], Some(vec![0; 256]));
        let spanning = pack_spanning(&indices, 5);
        let padded = pack(&indices, 5);
        assert_eq!((spanning.len(), padded.len()), (320, 342));
        let resized = |data: &[i64], length: usize| {
            let mut data = data.to_vec();
            data.resize(length, 0);
            data
        };
        for (data, empty) in [
            (spanning.clone(), false),
            (padded.clone(), false),
            (resized(&spanning, 330), true),
            (resized(&spanning, 319), true),
            (resized(&padded, 343), true),
            (resized(&padded, 341), true),
            (Vec::new(), true),
        ] {
            let length = data.len();
            let bytes = level_chunk_with(
                vec![ground.clone(), level_section(1, &TWENTY, Some(data))],
                "full",
            );
            let surface = surface(&bytes, Roof::Open, None);
            // The stone under a section that does not fit must not show as the top.
            let expected = (!empty).then(|| top("minecraft:stone", 16)).flatten();
            assert!(
                surface.iter().all(|column| *column == expected),
                "{length} longs"
            );
        }
    }

    #[test]
    fn negative_and_high_sections_are_read() {
        let one_block =
            |local_y: usize| fill(move |x, y, z| u16::from(x == 3 && z == 5 && y == local_y));
        for spanning in [true, false] {
            let below = chunk_1_13(
                vec![(
                    -1,
                    vec!["minecraft:air", "minecraft:obsidian"],
                    one_block(6),
                )],
                "full",
                spanning,
            );
            let surface_below = surface(&below, Roof::Open, None);
            assert_eq!(surface_below[5 * 16 + 3], top("minecraft:obsidian", -10));
            assert_eq!(surface_below.iter().flatten().count(), 1);

            let above = chunk_1_13(
                vec![(
                    18,
                    vec!["minecraft:air", "minecraft:obsidian"],
                    one_block(12),
                )],
                "full",
                spanning,
            );
            let surface_above = surface(&above, Roof::Open, None);
            assert_eq!(surface_above[5 * 16 + 3], top("minecraft:obsidian", 300));
            assert_eq!(surface_above.iter().flatten().count(), 1);
        }

        // Sections with only light, below the world and above it as 1.14 writes them, are air.
        let mut sections = vec![level_section(-1, &[], None)];
        sections.push(level_section(
            3,
            &["minecraft:dirt"],
            Some(pack(&[0; 4096], 4)),
        ));
        sections.push(level_section(16, &[], None));
        let bytes = level_chunk_with(sections, "full");
        let surface = surface(&bytes, Roof::Open, None);
        assert!(
            surface
                .iter()
                .all(|column| *column == top("minecraft:dirt", 63))
        );
    }

    #[test]
    fn old_and_new_chunks_side_by_side() {
        let old = chunk_1_13(terrain(), "postprocessed", true);
        let new = chunk_1_18(terrain(), FULL);
        for bytes in [old, new] {
            assert_terrain(&surface(&bytes, Roof::Open, None));
        }
    }

    #[test]
    fn an_unfinished_old_chunk_is_empty() {
        for status in [
            "carved",
            "minecraft:features",
            "liquid_carved",
            "base",
            "postprocessed!",
        ] {
            let bytes = chunk_1_13(vec![(0, vec!["minecraft:stone"], vec![])], status, false);
            let surface = surface(&bytes, Roof::Open, None);
            assert!(surface.iter().all(Option::is_none), "{status}");
        }
        // The statuses 1.13 and 1.14 to 1.17 give a finished chunk, and none at all.
        for status in ["full", "minecraft:full", "postprocessed", "fullchunk", ""] {
            let bytes = chunk_1_13(vec![(0, vec!["minecraft:stone"], vec![])], status, false);
            let surface = surface(&bytes, Roof::Open, None);
            assert_eq!(surface[0], top("minecraft:stone", 15), "{status:?}");
        }
        // The statuses of 1.13 alone do not finish a chunk of the newer shape.
        let bytes = chunk_1_18(vec![(0, vec!["minecraft:stone"], vec![])], "postprocessed");
        assert!(
            surface(&bytes, Roof::Open, None)
                .iter()
                .all(Option::is_none)
        );
    }

    #[test]
    fn an_old_chunk_that_lies_is_empty_not_a_panic() {
        let empty = assert_empty;
        let ground = level_section(0, &["minecraft:stone"], Some(vec![0; 256]));
        for (what, palette, data) in ill_formed_sections() {
            let mut bad = vec![("Y", Tag::Byte(1))];
            if let Some(palette) = palette {
                bad.push(("Palette", palette));
            }
            if let Some(data) = data {
                bad.push(("BlockStates", data));
            }
            let bytes = level_chunk_with(vec![ground.clone(), compound(bad)], "full");
            empty(&bytes, what);
        }

        for spanning in [true, false] {
            let past_palette = chunk_1_13(
                vec![
                    (0, vec!["minecraft:stone"], vec![]),
                    (1, TWENTY.to_vec(), fill(|x, _, _| u16::from(x == 0) * 20)),
                ],
                "full",
                spanning,
            );
            empty(&past_palette, "an index past the palette");
            for y in [127, -128] {
                let bytes =
                    chunk_1_13(vec![(y, vec!["minecraft:stone"], vec![])], "full", spanning);
                empty(&bytes, &format!("section Y {y}"));
            }
        }

        // Every index of 5 bits set: 31 is past a palette of twenty.
        let all_ones = level_section(1, &TWENTY, Some(vec![-1; 320]));
        empty(
            &level_chunk_with(vec![ground.clone(), all_ones], "full"),
            "indices of all ones",
        );

        let no_y = level_chunk_with(
            vec![compound(vec![("SkyLight", Tag::ByteArray(vec![0; 2048]))])],
            "full",
        );
        empty(&no_y, "a section without Y");

        let shapes = [
            (
                "Level not a compound",
                compound(vec![("Level", Tag::Int(3))]),
            ),
            (
                "Sections not a list",
                compound(vec![("Level", compound(vec![("Sections", Tag::Int(3))]))]),
            ),
            ("no Sections", compound(vec![("Level", compound(vec![]))])),
            (
                "Status not a string",
                compound(vec![(
                    "Level",
                    compound(vec![
                        ("Status", Tag::Int(1)),
                        (
                            "Sections",
                            compounds(vec![level_section(
                                0,
                                &["minecraft:stone"],
                                Some(vec![0; 256]),
                            )]),
                        ),
                    ]),
                )]),
            ),
        ];
        for (what, tag) in shapes {
            let bytes = nbt::write(&Root {
                name: Vec::new(),
                tag,
            });
            empty(&bytes, what);
        }
    }

    fn real_status_is_full(bytes: &[u8]) -> bool {
        let keep = |path: &[&[u8]]| match path {
            [b"Status"] | [b"Level", b"Status"] => Keep::All,
            [b"Level"] => Keep::Descend,
            _ => Keep::Skip,
        };
        let limits = Limits {
            max_tags: 2_000_000,
            max_depth: 64,
        };
        let Ok(root) = nbt::read_keeping(bytes, &keep, limits) else {
            return false;
        };
        let status = root
            .tag
            .get("Status")
            .or_else(|| root.tag.get("Level")?.get("Status"));
        matches!(status, Some(Tag::String(s)) if s == b"minecraft:full" || s == b"full")
    }

    /// Runs only with `GDL_REAL_WORLDS` set to the folder of real worlds.
    #[test]
    fn real_chunks_show_terrain() {
        let Some(root) = std::env::var_os("GDL_REAL_WORLDS") else {
            return;
        };
        let root = std::path::PathBuf::from(root);
        let mut colors = std::collections::HashSet::new();
        let mut water = false;
        let mut grids = Vec::new();
        for world in [
            "1.16.5/region",
            "1.20.1/region",
            "1.21.1/region",
            "26.3/dimensions/minecraft/overworld/region",
        ] {
            let mut files: Vec<_> = std::fs::read_dir(root.join(world))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.extension().is_some_and(|e| e == "mca"))
                .collect();
            files.sort();
            assert!(!files.is_empty(), "no regions in {world}");
            let (mut chunks, mut full, mut columns, mut shown, mut grey) = (0, 0, 0, 0, 0);
            let mut world_colors = std::collections::HashSet::new();
            let mut world_water = false;
            for path in files {
                let Some(region) = Region::read(&path).unwrap() else {
                    continue;
                };
                for z in 0..32 {
                    for x in 0..32 {
                        let Some(bytes) = region.chunk(x, z) else {
                            continue;
                        };
                        chunks += 1;
                        let surface = surface(&bytes, Roof::Open, None);
                        if !real_status_is_full(&bytes) {
                            continue;
                        }
                        full += 1;
                        columns += 256;
                        for top in surface.iter().flatten() {
                            shown += 1;
                            grey += usize::from(top.color == UNKNOWN && !top.water);
                            world_water |= top.water;
                            world_colors.insert(top.color);
                        }
                        // The first finished chunk of each world, rows z and columns x.
                        if !grids.contains(&world) {
                            grids.push(world);
                            println!("heights of {path:?} chunk {x},{z}:");
                            for row in surface.chunks(16) {
                                let line: Vec<String> = row
                                    .iter()
                                    .map(|top| top.map_or("   -".into(), |t| format!("{:4}", t.y)))
                                    .collect();
                                println!("{}", line.join(""));
                            }
                        }
                    }
                }
            }
            println!(
                "{world}: {chunks} read, {full} full, {shown} of {columns} columns shown, \
                 {grey} grey, {} colours, water seen: {world_water}",
                world_colors.len()
            );
            assert!(full > 0, "no finished chunk in {world}");
            assert!(
                shown * 100 >= columns * 90,
                "too few columns shown in {world}"
            );
            assert!(grey * 50 < shown, "too many grey columns in {world}");
            water |= world_water;
            colors.extend(world_colors);
        }
        println!("all worlds: {} colours, water seen: {water}", colors.len());
        assert!(water, "no water");
        assert!(colors.len() >= 20, "too few colours");
    }
}
