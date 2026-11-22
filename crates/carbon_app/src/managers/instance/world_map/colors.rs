//! The colour each block shows on a map seen from above.

use std::collections::HashMap;
use std::sync::OnceLock;

pub(super) type Rgb = [u8; 3];

#[cfg_attr(not(test), allow(dead_code))]
pub(super) const UNKNOWN: Rgb = [0x80, 0x80, 0x80];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Look {
    Color(Rgb),
    Water(Rgb),
    SeeThrough,
}

/// Made once by a local tool from the game's own textures. A bad line is skipped at run time;
/// `every_line_of_both_tables_parses` is what keeps the shipped files free of them.
const BLOCK_COLORS: &str = include_str!("block_colors.tsv");
const LEGACY_BLOCK_COLORS: &str = include_str!("legacy_block_colors.tsv");

/// Looks of 1.12.2 numeric ids: by id and data value, and by id for any other data value.
struct LegacyTable {
    by_data: HashMap<(u16, u8), Look>,
    by_id: HashMap<u16, Look>,
}

/// The look of a block of 1.13 or later by its name, such as `minecraft:stone`. A block the
/// table does not know (a mod's, or one of a newer game) is grey.
pub(super) fn look_of(name: &str) -> Look {
    static TABLE: OnceLock<HashMap<&'static str, Look>> = OnceLock::new();
    let table = TABLE.get_or_init(|| BLOCK_COLORS.lines().filter_map(parse_block_line).collect());
    table.get(name).copied().unwrap_or(Look::Color(UNKNOWN))
}

/// The look of a vanilla block of 1.2 to 1.12.2 by its numeric id and data value; `None` for an
/// id the game never had.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn legacy_look_of(id: u16, data: u8) -> Option<Look> {
    static TABLE: OnceLock<LegacyTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = LegacyTable {
            by_data: HashMap::new(),
            by_id: HashMap::new(),
        };
        for (id, data, look) in LEGACY_BLOCK_COLORS.lines().filter_map(parse_legacy_line) {
            match data {
                Some(data) => table.by_data.insert((id, data), look),
                None => table.by_id.insert(id, look),
            };
        }
        table
    });
    table
        .by_data
        .get(&(id, data))
        .or_else(|| table.by_id.get(&id))
        .copied()
}

/// `name<TAB>look`
fn parse_block_line(line: &str) -> Option<(&str, Look)> {
    let (name, look) = line.split_once('\t')?;
    if name.is_empty() {
        return None;
    }
    Some((name, parse_look(look)?))
}

/// `id<TAB>data|*<TAB>look`; a data value of `*` is `None`, for any data value.
fn parse_legacy_line(line: &str) -> Option<(u16, Option<u8>, Look)> {
    let mut fields = line.split('\t');
    let id = fields.next()?.parse().ok()?;
    let data = match fields.next()? {
        "*" => None,
        data => Some(data.parse::<u8>().ok().filter(|&data| data < 16)?),
    };
    let look = parse_look(fields.next()?)?;
    if fields.next().is_some() {
        return None;
    }
    Some((id, data, look))
}

/// `rrggbb`, `water:rrggbb` or `-`.
fn parse_look(text: &str) -> Option<Look> {
    if text == "-" {
        return Some(Look::SeeThrough);
    }
    if let Some(rgb) = text.strip_prefix("water:") {
        return Some(Look::Water(parse_rgb(rgb)?));
    }
    Some(Look::Color(parse_rgb(text)?))
}

fn parse_rgb(text: &str) -> Option<Rgb> {
    if text.len() != 6 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&text[at..at + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn color(look: Look) -> Rgb {
        match look {
            Look::Color(rgb) => rgb,
            other => panic!("expected a colour, got {other:?}"),
        }
    }

    fn largest_channel(rgb: Rgb) -> usize {
        (0..3).max_by_key(|&i| rgb[i]).unwrap()
    }

    #[test]
    fn known_blocks_have_plausible_colours() {
        let stone = color(look_of("minecraft:stone"));
        let (low, high) = (*stone.iter().min().unwrap(), *stone.iter().max().unwrap());
        assert!(high - low <= 30, "stone is not grey: {stone:?}");
        assert!(
            (90..=150).contains(&low) && (90..=150).contains(&high),
            "stone: {stone:?}"
        );

        let grass = color(look_of("minecraft:grass_block"));
        assert_eq!(largest_channel(grass), 1, "grass_block: {grass:?}");

        let sand = color(look_of("minecraft:sand"));
        // Sand is light and yellowish: its texture's blue is about 163, as on the game's own map.
        let mean = sand.iter().map(|&c| u32::from(c)).sum::<u32>() / 3;
        assert!(
            mean > 180 && sand.iter().all(|&c| c > 150),
            "sand: {sand:?}"
        );

        match look_of("minecraft:water") {
            Look::Water(rgb) => assert_eq!(largest_channel(rgb), 2, "water: {rgb:?}"),
            other => panic!("water is {other:?}"),
        }

        let leaves = color(look_of("minecraft:oak_leaves"));
        assert_eq!(largest_channel(leaves), 1, "oak_leaves: {leaves:?}");
    }

    #[test]
    fn air_and_small_things_are_see_through() {
        for name in [
            "minecraft:air",
            "minecraft:cave_air",
            "minecraft:void_air",
            "minecraft:torch",
            "minecraft:poppy",
            "minecraft:glass_pane",
            "minecraft:rail",
            "minecraft:barrier",
        ] {
            assert_eq!(look_of(name), Look::SeeThrough, "{name}");
        }
    }

    #[test]
    fn a_mod_block_and_an_unknown_vanilla_block_are_grey() {
        assert_eq!(look_of("create:andesite_casing"), Look::Color(UNKNOWN));
        assert_eq!(look_of("minecraft:not_a_block"), Look::Color(UNKNOWN));
    }

    #[test]
    fn every_line_of_both_tables_parses() {
        let mut names = HashSet::new();
        for line in BLOCK_COLORS.lines() {
            let (name, _) = parse_block_line(line).unwrap_or_else(|| panic!("bad line {line:?}"));
            assert!(names.insert(name), "{name} is in the table twice");
        }
        assert!(names.len() >= 900, "only {} names", names.len());

        let mut keys = HashSet::new();
        for line in LEGACY_BLOCK_COLORS.lines() {
            let (id, data, _) =
                parse_legacy_line(line).unwrap_or_else(|| panic!("bad line {line:?}"));
            assert!(
                keys.insert((id, data)),
                "{id}:{data:?} is in the table twice"
            );
        }
        let ids: HashSet<u16> = keys.iter().map(|&(id, _)| id).collect();
        assert!(ids.len() >= 150, "only {} ids", ids.len());
    }

    #[test]
    fn legacy_ids() {
        let stone = color(legacy_look_of(1, 0).unwrap());
        let (low, high) = (*stone.iter().min().unwrap(), *stone.iter().max().unwrap());
        assert!(high - low <= 30, "stone is not grey: {stone:?}");

        let grass = color(legacy_look_of(2, 0).unwrap());
        assert_eq!(largest_channel(grass), 1, "grass: {grass:?}");

        assert!(matches!(legacy_look_of(9, 0), Some(Look::Water(_))));

        let red_wool = color(legacy_look_of(35, 14).unwrap());
        assert_eq!(largest_channel(red_wool), 0, "red wool: {red_wool:?}");
        assert_ne!(legacy_look_of(35, 14), legacy_look_of(35, 11));

        assert_eq!(legacy_look_of(0, 0), Some(Look::SeeThrough));
        assert_eq!(legacy_look_of(4000, 0), None);
    }
}
