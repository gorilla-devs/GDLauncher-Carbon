//! The top visible block of each of a chunk's 256 columns, for a map seen from above.
//! Worlds come from anywhere: a chunk that does not hold together gives an empty
//! surface, never a panic or a guess.

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::RangeInclusive;

use super::colors::{Look, Rgb, UNKNOWN, legacy_look_of, look_of};
use super::legacy::{IdKind, LegacyIds};
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

/// The top visible block of each column of a chunk's uncompressed NBT, of 1.2 and later.
/// A chunk of a shape this reader does not know, an unfinished one or one that does not hold
/// together is empty. `legacy` says what each numeric id of a world of 1.2 to 1.12.2 is;
/// without it ids from 256 up are mods'.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn surface(chunk_nbt: &[u8], roof: Roof, legacy: Option<&LegacyIds>) -> Surface {
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
        // A chunk of 1.12.2 and earlier has no status: it is read as finished.
        if !finished(level.get("Status"), FINISHED_LEVEL) {
            return EMPTY;
        }
        let Some(Tag::List { items, .. }) = level.get("Sections") else {
            return EMPTY;
        };
        // All of a chunk's sections are of one shape.
        let shape = Cell::new(None);
        Column::new(items, |section| {
            let Some(this) = LevelShape::of(section)? else {
                return Ok(None);
            };
            if shape.replace(Some(this)).is_some_and(|other| other != this) {
                return Err(IllFormed);
            }
            match this {
                LevelShape::Palette => Section::read_level(section),
                LevelShape::NumericIds => {
                    Section::read_numeric(section, legacy.unwrap_or(&LegacyIds::WITHOUT_LIST))
                }
            }
        })
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
            b"Y" | b"Palette" | b"BlockStates" | b"Blocks" | b"Data" | b"Add" | b"Blocks16"
            | b"Add2",
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

/// The two shapes of section under `Level.Sections`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LevelShape {
    /// `Palette` and `BlockStates`, 1.13 to 1.17.
    Palette,
    /// `Blocks`, `Data` and `Add`, 1.2 to 1.12.2.
    NumericIds,
}

impl LevelShape {
    /// `None` for a section with only light. Arrays of mods that widen the ids, or the arrays
    /// of both shapes in one section, are ill-formed: read as either, the section would lie.
    fn of(section: &Tag) -> Result<Option<LevelShape>, IllFormed> {
        let has = |keys: &[&str]| keys.iter().any(|key| section.get(key).is_some());
        if has(&["Blocks16", "Add2"]) {
            return Err(IllFormed);
        }
        match (
            has(&["Palette", "BlockStates"]),
            has(&["Blocks", "Data", "Add"]),
        ) {
            (true, true) => Err(IllFormed),
            (true, false) => Ok(Some(LevelShape::Palette)),
            (false, true) => Ok(Some(LevelShape::NumericIds)),
            (false, false) => Ok(None),
        }
    }
}

/// The look of a numeric id and data value: grey for a mod's block, an id the world does not
/// vouch for, or one the table lacks.
fn numeric_block(id: u16, data: u8, legacy: &LegacyIds) -> Block {
    let vanilla = legacy.kind_of(id) == IdKind::Vanilla;
    let look = if vanilla {
        legacy_look_of(id, data).unwrap_or(Look::Color(UNKNOWN))
    } else {
        Look::Color(UNKNOWN)
    };
    Block {
        look,
        bedrock: vanilla && id == 7,
    }
}

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

    /// One section of 1.2 to 1.12.2: an id byte per block, and nibbles of data and of the
    /// id's high bits, the low nibble first. These worlds are 256 high, so `Y` is 0 to 15.
    fn read_numeric(section: &Tag, legacy: &LegacyIds) -> SectionRead {
        if !matches!(section.get("Y"), Some(Tag::Byte(0..=15))) {
            return Err(IllFormed);
        }
        let bytes = |key: &str, len: usize| match section.get(key) {
            None => Ok(None),
            Some(Tag::ByteArray(bytes)) if bytes.len() == len => Ok(Some(bytes)),
            Some(_) => Err(IllFormed),
        };
        let blocks = bytes("Blocks", SECTION_BLOCKS)?.ok_or(IllFormed)?;
        let data = bytes("Data", SECTION_BLOCKS / 2)?.ok_or(IllFormed)?;
        let add = bytes("Add", SECTION_BLOCKS / 2)?;
        let nibble = |array: &[u8], i: usize| (array[i / 2] >> ((i % 2) * 4)) & 0xF;
        let mut palette = Vec::new();
        let mut seen = HashMap::new();
        let indices = (0..SECTION_BLOCKS)
            .map(|i| {
                let high = add.map_or(0, |add| u16::from(nibble(add, i)) << 8);
                let (id, data) = (u16::from(blocks[i]) | high, nibble(data, i));
                *seen.entry((id, data)).or_insert_with(|| {
                    palette.push(numeric_block(id, data, legacy));
                    (palette.len() - 1) as u16
                })
            })
            .collect();
        Ok(Some(match palette[..] {
            [block] => Section::Uniform(block),
            _ => Section::Mixed { palette, indices },
        }))
    }

    /// A section from its palette and packed longs, either shape. No palette and no longs is
    /// a section with only light; a palette of one needs no longs (an empty array of them is
    /// none, as world editors write it), but any it has must fit.
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
            Some(Tag::LongArray(data)) if data.is_empty() && palette.len() == 1 => {
                return Ok(Some(Section::Uniform(palette[0])));
            }
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
mod tests;
