//! Which numeric block ids of a world of 1.2 to 1.12.2 belong to mods, by the world's own list.
//! A list that does not hold together is not used at all.

use crate::util::nbt::{self, Keep, Limits, Tag};

/// Walking a `level.dat` costs at most this, the player's inventory and the rest included.
const LIMITS: Limits = Limits {
    max_tags: 2_000_000,
    max_depth: 64,
};

/// Ids run to 4095: a byte and the nibble of `Add`.
const IDS: usize = 4096;

/// The game itself never gave a block an id above this.
pub(super) const HIGHEST_VANILLA_ID: u16 = 255;

/// What a world of 1.2 to 1.12.2 says about its numeric block ids.
pub(super) struct LegacyIds {
    /// By id, whether the world's list names a mod's block there; `None` where it names
    /// nothing. `None` altogether for a world without a list that holds together.
    listed: Option<Vec<Option<bool>>>,
}

impl LegacyIds {
    /// From the uncompressed NBT of the world's `level.dat`: Forge's list of 1.12
    /// (`FML.Registries`) or of 1.7 to 1.11 (`FML.ItemData`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn from_level_dat(level_nbt: &[u8]) -> LegacyIds {
        LegacyIds {
            listed: read_list(level_nbt),
        }
    }

    /// An id the list does not name, or any id without a list, is a mod's from 256 up.
    pub(super) fn is_mod_block(&self, id: u16) -> bool {
        let listed = self.listed.as_ref();
        match listed
            .and_then(|ids| ids.get(usize::from(id)))
            .copied()
            .flatten()
        {
            Some(is_mod) => is_mod,
            None => id > HIGHEST_VANILLA_ID,
        }
    }
}

fn keep(path: &[&[u8]]) -> Keep {
    match path {
        [b"FML"]
        | [b"FML", b"Registries" | b"ItemData"]
        | [b"FML", b"Registries", b"minecraft:blocks"]
        | [b"FML", b"Registries", b"minecraft:blocks", b"ids"] => Keep::Descend,
        [b"FML", b"ItemData", b"K" | b"V"]
        | [
            b"FML",
            b"Registries",
            b"minecraft:blocks",
            b"ids",
            b"K" | b"V",
        ] => Keep::All,
        _ => Keep::Skip,
    }
}

/// The 1.12 list when the world has one, else the 1.7 to 1.11 list.
fn read_list(level_nbt: &[u8]) -> Option<Vec<Option<bool>>> {
    let root = nbt::read_keeping(level_nbt, &keep, LIMITS).ok()?;
    let fml = root.tag.get("FML")?;
    let registry = fml
        .get("Registries")
        .and_then(|r| r.get("minecraft:blocks"));
    if let Some(blocks) = registry {
        return ids_of(blocks.get("ids")?, |name| Some(Some(name)));
    }
    // Blocks are named with a leading 1, items with a leading 2.
    ids_of(fml.get("ItemData")?, |name| {
        match name.split_at_checked(1)? {
            ("\u{1}", block) => Some(Some(block)),
            ("\u{2}", _) => Some(None),
            _ => None,
        }
    })
}

/// Reads a list of `{K: name, V: id}`. `block_name` gives the block's name a key holds,
/// `Some(None)` for a key that is not a block's, or `None` for one that lies. `None` when the
/// list lies anywhere: an entry of another shape, a block id twice or past 4095, or air given
/// to anything else.
fn ids_of(
    list: &Tag,
    block_name: impl Fn(&str) -> Option<Option<&str>>,
) -> Option<Vec<Option<bool>>> {
    let Tag::List { items, .. } = list else {
        return None;
    };
    let mut listed = vec![None; IDS];
    for entry in items {
        let (Some(Tag::String(key)), Some(Tag::Int(id))) = (entry.get("K"), entry.get("V")) else {
            return None;
        };
        let key = nbt::decode_string(key)?;
        let Some(name) = block_name(&key)? else {
            continue;
        };
        let slot = listed.get_mut(usize::try_from(*id).ok()?)?;
        if slot.is_some() || (*id == 0 && name != "minecraft:air") {
            return None;
        }
        *slot = Some(!name.starts_with("minecraft:"));
    }
    Some(listed)
}

/// The uncompressed `level.dat` of a Forge world of 1.12, whose block list is `blocks`
/// (`(name, id)`); its item list holds a mod item on id 1, which must not count.
#[cfg(test)]
pub(super) fn level_dat_1_12(blocks: &[(&str, i32)]) -> Vec<u8> {
    tests::with_fml(tests::registries(tests::entries(blocks)))
}

/// The uncompressed `level.dat` of a Forge world of 1.7 to 1.11 whose `ItemData` holds the
/// blocks `blocks` (`(name, id)`) and a mod item on id 1, which must not count.
#[cfg(test)]
pub(super) fn level_dat_1_7(blocks: &[(&str, i32)]) -> Vec<u8> {
    let mut named: Vec<(String, i32)> = blocks
        .iter()
        .map(|(name, id)| (format!("\u{1}{name}"), *id))
        .collect();
    named.push(("\u{2}ic2:item_battery".to_string(), 1));
    let named: Vec<(&str, i32)> = named.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    tests::with_fml(tests::compound(vec![("ItemData", tests::entries(&named))]))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::util::nbt::{self, Root, Tag};

    pub(super) fn compound(entries: Vec<(&str, Tag)>) -> Tag {
        Tag::Compound(
            entries
                .into_iter()
                .map(|(key, tag)| (key.as_bytes().to_vec(), tag))
                .collect(),
        )
    }

    fn string(text: &str) -> Tag {
        Tag::String(nbt::encode_string(text))
    }

    fn list(items: Vec<Tag>) -> Tag {
        Tag::List {
            element: if items.is_empty() { 0 } else { 10 },
            items,
        }
    }

    pub(super) fn entries(named: &[(&str, i32)]) -> Tag {
        list(
            named
                .iter()
                .map(|(k, v)| compound(vec![("K", string(k)), ("V", Tag::Int(*v))]))
                .collect(),
        )
    }

    /// `FML.Registries` with `ids` as the block list, beside an item list.
    pub(super) fn registries(ids: Tag) -> Tag {
        compound(vec![(
            "Registries",
            compound(vec![
                (
                    "minecraft:items",
                    compound(vec![("ids", entries(&[("ic2:item_battery", 1)]))]),
                ),
                (
                    "minecraft:blocks",
                    compound(vec![
                        ("ids", ids),
                        ("aliases", list(vec![])),
                        ("dummied", list(vec![])),
                    ]),
                ),
            ]),
        )])
    }

    /// A `level.dat` with `fml` as its `FML`, beside the world's ordinary data.
    pub(super) fn with_fml(fml: Tag) -> Vec<u8> {
        nbt::write(&Root {
            name: Vec::new(),
            tag: compound(vec![
                (
                    "Data",
                    compound(vec![
                        ("LevelName", string("Modded")),
                        ("version", Tag::Int(19133)),
                    ]),
                ),
                ("FML", fml),
            ]),
        })
    }

    fn vanilla_level_dat() -> Vec<u8> {
        nbt::write(&Root {
            name: Vec::new(),
            tag: compound(vec![(
                "Data",
                compound(vec![("LevelName", string("Vanilla"))]),
            )]),
        })
    }

    fn assert_neither_list(ids: &LegacyIds, what: &str) {
        for id in [0, 1, 200, 255] {
            assert!(!ids.is_mod_block(id), "{what}: {id}");
        }
        for id in [256, 300, 4095, 4096, u16::MAX] {
            assert!(ids.is_mod_block(id), "{what}: {id}");
        }
    }

    #[test]
    fn without_a_list_only_ids_from_256_are_mod_blocks() {
        assert_neither_list(&LegacyIds::from_level_dat(&vanilla_level_dat()), "vanilla");
        assert_neither_list(
            &LegacyIds::from_level_dat(&with_fml(compound(vec![]))),
            "FML without a list",
        );
    }

    #[test]
    fn the_worlds_own_list_says_which_ids_are_mods() {
        let blocks = [
            ("minecraft:air", 0),
            ("minecraft:stone", 1),
            ("ic2:rubber_wood", 200),
            ("minecraft:chorus_flower", 255),
        ];
        for (what, bytes) in [
            ("1.12", level_dat_1_12(&blocks)),
            ("1.7", level_dat_1_7(&blocks)),
        ] {
            let ids = LegacyIds::from_level_dat(&bytes);
            assert!(ids.is_mod_block(200), "{what}");
            assert!(!ids.is_mod_block(0), "{what}");
            // A mod item on id 1 is not a block.
            assert!(!ids.is_mod_block(1), "{what}");
            assert!(!ids.is_mod_block(255), "{what}");
            // Ids the list does not name follow the rule for a world without one.
            assert!(!ids.is_mod_block(100), "{what}");
            assert!(ids.is_mod_block(256), "{what}");
        }
        // A vanilla block the list puts above 255 is not a mod's.
        let ids = LegacyIds::from_level_dat(&level_dat_1_12(&[("minecraft:stone", 300)]));
        assert!(!ids.is_mod_block(300));
    }

    /// The raw bytes of a 1.7 `level.dat` whose `ItemData` names id 200 a mod block and then
    /// holds `items` more item entries.
    fn item_data_of(items: usize) -> Vec<u8> {
        let mut out = vec![10, 0, 0];
        let named = |out: &mut Vec<u8>, id: u8, name: &[u8]| {
            out.push(id);
            out.extend_from_slice(&(name.len() as u16).to_be_bytes());
            out.extend_from_slice(name);
        };
        let entry = |out: &mut Vec<u8>, key: &[u8], value: i32| {
            named(out, 8, b"K");
            out.extend_from_slice(&(key.len() as u16).to_be_bytes());
            out.extend_from_slice(key);
            named(out, 3, b"V");
            out.extend_from_slice(&value.to_be_bytes());
            out.push(0);
        };
        named(&mut out, 10, b"FML");
        named(&mut out, 9, b"ItemData");
        out.push(10);
        out.extend_from_slice(&(items as i32 + 1).to_be_bytes());
        entry(&mut out, b"\x01ic2:rubber_wood", 200);
        for _ in 0..items {
            entry(&mut out, b"\x02ic2:item", 5000);
        }
        out.extend_from_slice(&[0, 0]);
        out
    }

    #[test]
    fn a_list_that_lies_falls_back() {
        let entry = |k: Tag, v: Tag| compound(vec![("K", k), ("V", v)]);
        let rubber = || entry(string("ic2:rubber_wood"), Tag::Int(200));
        let lying_ids = [
            (
                "an id not a number",
                list(vec![rubber(), entry(string("ic2:leaves"), string("201"))]),
            ),
            (
                "an id as a long",
                list(vec![rubber(), entry(string("ic2:leaves"), Tag::Long(201))]),
            ),
            (
                "a name not a string",
                list(vec![rubber(), entry(Tag::Int(3), Tag::Int(201))]),
            ),
            (
                "an entry without a name",
                list(vec![rubber(), compound(vec![("V", Tag::Int(201))])]),
            ),
            (
                "an id past 4095",
                list(vec![rubber(), entry(string("ic2:leaves"), Tag::Int(4096))]),
            ),
            (
                "a negative id",
                list(vec![rubber(), entry(string("ic2:leaves"), Tag::Int(-1))]),
            ),
            (
                "one id twice",
                list(vec![rubber(), entry(string("ic2:leaves"), Tag::Int(200))]),
            ),
            (
                "air given to a mod",
                list(vec![rubber(), entry(string("ic2:leaves"), Tag::Int(0))]),
            ),
            (
                "a list of numbers",
                Tag::List {
                    element: 3,
                    items: vec![Tag::Int(200)],
                },
            ),
            ("not a list", Tag::Int(200)),
        ];
        for (what, ids) in lying_ids {
            let bytes = with_fml(registries(ids));
            assert_neither_list(&LegacyIds::from_level_dat(&bytes), what);
        }

        let lying_item_data = [
            (
                "a name without a kind",
                entries(&[("ic2:rubber_wood", 200)]),
            ),
            (
                "a block not a string",
                list(vec![compound(vec![
                    ("K", Tag::Int(1)),
                    ("V", Tag::Int(200)),
                ])]),
            ),
        ];
        for (what, item_data) in lying_item_data {
            let bytes = with_fml(compound(vec![("ItemData", item_data)]));
            assert_neither_list(&LegacyIds::from_level_dat(&bytes), what);
        }

        for (what, bytes) in [
            ("FML not a compound", with_fml(Tag::Int(1))),
            ("nothing", Vec::new()),
            ("not NBT", b"not nbt at all".to_vec()),
            (
                "cut short",
                level_dat_1_12(&[("ic2:rubber_wood", 200)])[..40].to_vec(),
            ),
        ] {
            assert_neither_list(&LegacyIds::from_level_dat(&bytes), what);
        }

        // A list the reader may walk is read; one past the tag limit is not read at all.
        assert!(LegacyIds::from_level_dat(&item_data_of(1000)).is_mod_block(200));
        let huge = LegacyIds::from_level_dat(&item_data_of(700_000));
        assert_neither_list(&huge, "a list of 2,100,000 tags");
    }
}
