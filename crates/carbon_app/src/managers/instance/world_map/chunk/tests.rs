use super::super::colors::{UNKNOWN, legacy_look_of, look_of};
use super::super::legacy::tests::{level_dat_1_7, level_dat_1_12, vanilla_level_dat};
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
        super::surface(
            &bytes,
            Roof::Open,
            Some(&LegacyIds::from_level_dat(&vanilla_level_dat()))
        ),
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
fn chunk_1_13(sections: Vec<(i8, Vec<&str>, Vec<u16>)>, status: &str, spanning: bool) -> Vec<u8> {
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
            let bytes = chunk_1_13(vec![(y, vec!["minecraft:stone"], vec![])], "full", spanning);
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

#[test]
fn a_palette_of_one_with_an_empty_data_array_is_that_block() {
    let newer = compound(vec![
        ("Y", Tag::Byte(0)),
        (
            "block_states",
            compound(vec![
                ("palette", names(&["minecraft:dirt"])),
                ("data", Tag::LongArray(vec![])),
            ]),
        ),
    ]);
    let older = compound(vec![
        ("Y", Tag::Byte(0)),
        ("Palette", names(&["minecraft:dirt"])),
        ("BlockStates", Tag::LongArray(vec![])),
    ]);
    for (what, bytes) in [
        ("newer shape", chunk_with(vec![newer], FULL)),
        ("older shape", level_chunk_with(vec![older], "full")),
    ] {
        let surface = surface(&bytes, Roof::Open, None);
        assert!(
            surface
                .iter()
                .all(|column| *column == top("minecraft:dirt", 15)),
            "{what}"
        );
    }
}

/// A section of numeric ids from its arrays as they are, light included.
fn numeric_section_of(y: i8, blocks: Vec<u8>, data: Vec<u8>, add: Option<Vec<u8>>) -> Tag {
    let mut entries = vec![
        ("Y", Tag::Byte(y)),
        ("Blocks", Tag::ByteArray(blocks)),
        ("Data", Tag::ByteArray(data)),
    ];
    if let Some(add) = add {
        entries.push(("Add", Tag::ByteArray(add)));
    }
    entries.push(("BlockLight", Tag::ByteArray(vec![0; 2048])));
    entries.push(("SkyLight", Tag::ByteArray(vec![0xFF; 2048])));
    compound(entries)
}

/// A section of numeric ids, `block_at(x, y, z)` giving `(id, data)`; an `Add` array only
/// when an id needs one.
fn numeric_section(y: i8, block_at: impl Fn(usize, usize, usize) -> (u16, u8)) -> Tag {
    let (mut blocks, mut data, mut add) = (vec![0u8; 4096], vec![0u8; 2048], vec![0u8; 2048]);
    for i in 0..4096 {
        let (id, value) = block_at(i % 16, i / 256, (i / 16) % 16);
        let shift = (i % 2) * 4;
        blocks[i] = id as u8;
        data[i / 2] |= (value & 0xF) << shift;
        add[i / 2] |= ((id >> 8) as u8 & 0xF) << shift;
    }
    let add = add.iter().any(|&byte| byte != 0).then_some(add);
    numeric_section_of(y, blocks, data, add)
}

/// A chunk of 1.2 to 1.12.2, with `TerrainPopulated` when `populated` is given.
fn numeric_chunk(sections: Vec<Tag>, populated: Option<i8>) -> Vec<u8> {
    let mut level = vec![
        ("xPos", Tag::Int(0)),
        ("zPos", Tag::Int(0)),
        ("LastUpdate", Tag::Long(1234)),
        ("LightPopulated", Tag::Byte(1)),
        ("InhabitedTime", Tag::Long(0)),
        ("HeightMap", Tag::IntArray(vec![64; 256])),
        ("Sections", compounds(sections)),
        ("Entities", compounds(vec![])),
        ("TileEntities", compounds(vec![])),
    ];
    if let Some(populated) = populated {
        level.push(("TerrainPopulated", Tag::Byte(populated)));
    }
    nbt::write(&Root {
        name: Vec::new(),
        tag: compound(vec![
            ("DataVersion", Tag::Int(1343)),
            ("Level", compound(level)),
        ]),
    })
}

fn legacy_top(id: u16, data: u8, y: i16) -> Option<Top> {
    let (color, water) = match legacy_look_of(id, data) {
        Some(Look::Color(rgb)) => (rgb, false),
        Some(Look::Water(rgb)) => (rgb, true),
        other => panic!("{id}:{data} is {other:?}"),
    };
    Some(Top { color, y, water })
}

fn grey(y: i16) -> Option<Top> {
    Some(Top {
        color: UNKNOWN,
        y,
        water: false,
    })
}

/// Stone up to 63, then on layer 64: grass where `x < 4` (a poppy over it at x = 0, a gold
/// block at x = 1, z = 2, y = 70), still water where `x` is 4 to 7, and wool where `x` is
/// 8 to 15: red (data 14) at even `x` and blue (data 11) at odd `x`, written by hand.
fn numeric_terrain() -> Vec<Tag> {
    let mut sections: Vec<Tag> = (0..4)
        .map(|y| numeric_section(y, |_, _, _| (1, 0)))
        .collect();
    let mut blocks = vec![0u8; 4096];
    let mut data = vec![0u8; 2048];
    for z in 0..16 {
        for x in 0..16 {
            blocks[z * 16 + x] = match x {
                0..=3 => 2,
                4..=7 => 9,
                _ => 35,
            };
            // Index z * 16 + x: x = 8 and 9 share byte z * 8 + 4, x = 8 in its low nibble.
            if x >= 8 && x % 2 == 0 {
                data[z * 8 + x / 2] = 0xBE;
            }
        }
    }
    blocks[256] = 38;
    blocks[6 * 256 + 2 * 16 + 1] = 41;
    sections.push(numeric_section_of(4, blocks, data, None));
    sections
}

#[test]
fn numeric_ids_are_read() {
    for populated in [Some(1), Some(0), None] {
        let bytes = numeric_chunk(numeric_terrain(), populated);
        let surface = surface(&bytes, Roof::Open, None);
        for z in 0..16 {
            for x in 0..16 {
                let expected = match x {
                    1 if z == 2 => legacy_top(41, 0, 70),
                    0..=3 => legacy_top(2, 0, 64),
                    4..=7 => legacy_top(9, 0, 64),
                    _ if x % 2 == 0 => legacy_top(35, 14, 64),
                    _ => legacy_top(35, 11, 64),
                };
                assert_eq!(surface[z * 16 + x], expected, "{x},{z} {populated:?}");
            }
        }
        assert!(surface[4].unwrap().water);
        assert_ne!(surface[8], surface[9]);
    }
}

#[test]
fn an_add_array_gives_ids_above_255() {
    // Id 1 throughout the top layer; by hand, Add byte 1920 holds the nibbles of x = 0
    // (low, 0) and x = 1 (high, 1), so x = 1 is id 257. The builder puts 300 at x = 2.
    let mut blocks = vec![1u8; 4096];
    blocks[15 * 256 + 2] = 300_u16 as u8;
    let mut add = vec![0u8; 2048];
    add[1920] = 0x10;
    add[1921] = 0x01;
    let by_hand = numeric_section_of(0, blocks, vec![0; 2048], Some(add));
    let built = numeric_section(1, |x, y, z| match (x, y, z) {
        (3, 0, 0) => (300, 0),
        (4, 0, 0) => (1, 0),
        _ => (0, 0),
    });
    let bytes = numeric_chunk(vec![by_hand, built], Some(1));
    let vanilla = LegacyIds::from_level_dat(&vanilla_level_dat());
    for legacy in [None, Some(&vanilla)] {
        let surface = surface(&bytes, Roof::Open, legacy);
        assert_eq!(surface[0], legacy_top(1, 0, 15));
        assert_eq!(surface[1], grey(15));
        assert_eq!(surface[2], grey(15));
        assert_eq!(surface[3], grey(16));
        assert_eq!(surface[4], legacy_top(1, 0, 16));
        assert_eq!(surface[5], legacy_top(1, 0, 15));
    }
}

#[test]
fn a_mod_block_named_by_the_worlds_list_is_grey_even_below_256() {
    // Id 200 is the chorus flower in the game itself; in these worlds a mod's block.
    let bytes = numeric_chunk(
        vec![numeric_section(
            0,
            |x, _, _| if x < 8 { (200, 0) } else { (1, 0) },
        )],
        Some(1),
    );
    let named = [
        ("minecraft:air", 0),
        ("minecraft:stone", 1),
        ("ic2:rubber_wood", 200),
    ];
    let vanilla = LegacyIds::from_level_dat(&vanilla_level_dat());
    let from_1_12 = LegacyIds::from_level_dat(&level_dat_1_12(&named));
    let from_1_7 = LegacyIds::from_level_dat(&level_dat_1_7(&named));
    let chorus = LegacyIds::from_level_dat(&level_dat_1_12(&[
        ("minecraft:stone", 1),
        ("minecraft:chorus_flower", 200),
    ]));
    for (what, legacy, mod_block) in [
        ("no list given", None, false),
        ("a world without a list", Some(&vanilla), false),
        ("a 1.12 list", Some(&from_1_12), true),
        ("a 1.7 list", Some(&from_1_7), true),
        ("a list naming the game's own block", Some(&chorus), false),
    ] {
        let surface = surface(&bytes, Roof::Open, legacy);
        let expected = if mod_block {
            grey(15)
        } else {
            legacy_top(200, 0, 15)
        };
        assert_eq!(surface[0], expected, "{what}");
        assert_eq!(surface[15], legacy_top(1, 0, 15), "{what}");
    }
}

#[test]
fn an_id_the_worlds_list_does_not_name_is_grey() {
    // Id 100 (a mushroom block in the game itself) where x < 8, stone elsewhere.
    let bytes = numeric_chunk(
        vec![numeric_section(
            0,
            |x, _, _| if x < 8 { (100, 0) } else { (1, 0) },
        )],
        Some(1),
    );
    let named = [("minecraft:air", 0), ("minecraft:stone", 1)];
    let vanilla = LegacyIds::from_level_dat(&vanilla_level_dat());
    let from_1_12 = LegacyIds::from_level_dat(&level_dat_1_12(&named));
    let from_1_7 = LegacyIds::from_level_dat(&level_dat_1_7(&named));
    for (what, legacy, expected) in [
        ("no list given", None, legacy_top(100, 0, 15)),
        (
            "a world without a list",
            Some(&vanilla),
            legacy_top(100, 0, 15),
        ),
        ("a 1.12 list", Some(&from_1_12), grey(15)),
        ("a 1.7 list", Some(&from_1_7), grey(15)),
    ] {
        let surface = surface(&bytes, Roof::Open, legacy);
        assert_eq!(surface[0], expected, "{what}");
        assert_eq!(surface[15], legacy_top(1, 0, 15), "{what}");
    }

    // Bedrock the list does not name is no roof to look under.
    let sections = (0..8)
        .map(|section_y: i8| {
            numeric_section(section_y, move |_, local, _| {
                match i32::from(section_y) * 16 + local as i32 {
                    127 => (7, 0),
                    40..=99 => (0, 0),
                    _ => (87, 0),
                }
            })
        })
        .collect();
    let bytes = numeric_chunk(sections, Some(1));
    let netherrack = LegacyIds::from_level_dat(&level_dat_1_12(&[
        ("minecraft:air", 0),
        ("minecraft:netherrack", 87),
    ]));
    let surface = surface(&bytes, Roof::Open, Some(&netherrack));
    assert!(surface.iter().all(|column| *column == grey(127)));
}

#[test]
fn a_list_that_cannot_be_trusted_greys_every_block_but_air() {
    // Stone up to 15, grass at 16 where x < 8, air above, water at 17 where x = 15.
    let bytes = numeric_chunk(
        vec![
            numeric_section(0, |_, _, _| (1, 0)),
            numeric_section(1, |x, y, _| match (x, y) {
                (0..8, 0) => (2, 0),
                (15, 1) => (9, 0),
                _ => (0, 0),
            }),
            numeric_section(2, |_, _, _| (0, 0)),
        ],
        Some(1),
    );
    let one_id_twice = level_dat_1_12(&[("ic2:rubber_wood", 200), ("ic2:leaves", 200)]);
    for (what, level_dat) in [
        ("one id twice", one_id_twice),
        ("a level.dat that is not NBT", b"not nbt".to_vec()),
    ] {
        let legacy = LegacyIds::from_level_dat(&level_dat);
        let surface = surface(&bytes, Roof::Open, Some(&legacy));
        for x in 0..16 {
            let expected = match x {
                0..8 => grey(16),
                15 => grey(17),
                _ => grey(15),
            };
            assert_eq!(surface[x], expected, "{what}: {x}");
        }
    }
}

#[test]
fn a_numeric_section_outside_0_to_15_empties_the_chunk() {
    let ground = numeric_section(0, |_, _, _| (1, 0));
    for y in [16, -1, 126, -127] {
        let bytes = numeric_chunk(
            vec![ground.clone(), numeric_section(y, |_, _, _| (1, 0))],
            Some(1),
        );
        assert_empty(&bytes, &format!("Y {y}"));
    }
    let bytes = numeric_chunk(
        vec![ground.clone(), numeric_section(15, |_, _, _| (1, 0))],
        Some(1),
    );
    assert_eq!(surface(&bytes, Roof::Open, None)[0], legacy_top(1, 0, 255));
}

#[test]
fn wrong_array_lengths_empty_the_chunk() {
    // Each bad section lies over good stone, which must not show through it.
    let ground = numeric_section(0, |_, _, _| (1, 0));
    let good = || {
        vec![
            ("Y", Tag::Byte(1)),
            ("Blocks", Tag::ByteArray(vec![1; 4096])),
            ("Data", Tag::ByteArray(vec![0; 2048])),
        ]
    };
    let with = |key: &'static str, tag: Tag| {
        let mut entries = good();
        entries.retain(|(name, _)| *name != key);
        entries.push((key, tag));
        compound(entries)
    };
    let without = |key: &str| {
        let mut entries = good();
        entries.retain(|(name, _)| *name != key);
        compound(entries)
    };
    let bad = [
        (
            "Blocks of 4095",
            with("Blocks", Tag::ByteArray(vec![1; 4095])),
        ),
        (
            "Blocks of 4097",
            with("Blocks", Tag::ByteArray(vec![1; 4097])),
        ),
        (
            "Blocks of 8192",
            with("Blocks", Tag::ByteArray(vec![1; 8192])),
        ),
        (
            "Blocks not bytes",
            with("Blocks", Tag::IntArray(vec![1; 4096])),
        ),
        ("Blocks missing", without("Blocks")),
        ("Data of 2047", with("Data", Tag::ByteArray(vec![0; 2047]))),
        ("Data of 4096", with("Data", Tag::ByteArray(vec![0; 4096]))),
        ("Data missing", without("Data")),
        ("Add of 2047", with("Add", Tag::ByteArray(vec![0; 2047]))),
        ("Add of 4096", with("Add", Tag::ByteArray(vec![0; 4096]))),
        ("Add not bytes", with("Add", Tag::Int(0))),
        (
            "Blocks16 of an id mod",
            with("Blocks16", Tag::ByteArray(vec![0; 8192])),
        ),
        (
            "Add2 of an id mod",
            with("Add2", Tag::ByteArray(vec![0; 2048])),
        ),
        (
            "a Palette beside Blocks",
            with("Palette", Tag::IntArray(vec![1, 2])),
        ),
        (
            "BlockStates beside Blocks",
            with("BlockStates", Tag::LongArray(vec![0; 256])),
        ),
    ];
    for (what, section) in bad {
        let bytes = numeric_chunk(vec![ground.clone(), section], Some(1));
        assert_empty(&bytes, what);
    }

    // The good section itself shows, so the cases above fail for their own reason.
    let bytes = numeric_chunk(vec![ground.clone(), compound(good())], Some(1));
    assert_eq!(surface(&bytes, Roof::Open, None)[0], legacy_top(1, 0, 31));

    // Sections of both shapes in one chunk, either way round.
    let palette = level_section(1, &["minecraft:stone"], Some(vec![0; 256]));
    for sections in [
        vec![ground.clone(), palette.clone()],
        vec![palette.clone(), numeric_section(2, |_, _, _| (1, 0))],
    ] {
        assert_empty(&numeric_chunk(sections, Some(1)), "both shapes");
    }

    // A section with only light is air in this shape too.
    let light_only = compound(vec![
        ("Y", Tag::Byte(1)),
        ("SkyLight", Tag::ByteArray(vec![0; 2048])),
    ]);
    let gold = numeric_section(2, |x, y, _| if x == 0 && y == 0 { (41, 0) } else { (0, 0) });
    let bytes = numeric_chunk(vec![ground.clone(), light_only, gold], Some(1));
    let surface = surface(&bytes, Roof::Open, None);
    assert_eq!(surface[0], legacy_top(41, 0, 32));
    assert_eq!(surface[1], legacy_top(1, 0, 15));
}

#[test]
fn a_chunk_without_sections_is_empty() {
    assert_empty(&numeric_chunk(vec![], Some(1)), "an empty section list");
    assert_empty(
        &numeric_chunk(vec![compound(vec![("Y", Tag::Byte(0))])], Some(1)),
        "sections without blocks",
    );
    // Before 1.2: one array of 32768 blocks under `Level`, no sections.
    let before = nbt::write(&Root {
        name: Vec::new(),
        tag: compound(vec![(
            "Level",
            compound(vec![
                ("xPos", Tag::Int(0)),
                ("zPos", Tag::Int(0)),
                ("Blocks", Tag::ByteArray(vec![1; 32768])),
                ("Data", Tag::ByteArray(vec![0; 16384])),
                ("HeightMap", Tag::ByteArray(vec![64; 256])),
                ("TerrainPopulated", Tag::Byte(1)),
            ]),
        )]),
    });
    assert_empty(&before, "a chunk of before 1.2");
    // Cubic Chunks: a cube's blocks under `Level.Section`, no `Sections`.
    let cube = nbt::write(&Root {
        name: Vec::new(),
        tag: compound(vec![(
            "Level",
            compound(vec![
                ("x", Tag::Int(0)),
                ("y", Tag::Int(4)),
                ("z", Tag::Int(0)),
                (
                    "Section",
                    compound(vec![
                        ("Blocks", Tag::ByteArray(vec![1; 4096])),
                        ("Data", Tag::ByteArray(vec![0; 2048])),
                    ]),
                ),
            ]),
        )]),
    });
    assert_empty(&cube, "a cube of Cubic Chunks");
}

#[test]
fn a_bedrock_roof_of_numeric_ids_is_looked_under() {
    // Bedrock (7) at 127, netherrack (87) 100 to 126, air 40 to 99, netherrack below.
    let sections = (0..8)
        .map(|section_y: i8| {
            numeric_section(section_y, move |_, local, _| {
                match i32::from(section_y) * 16 + local as i32 {
                    127 => (7, 0),
                    100..=126 => (87, 0),
                    40..=99 => (0, 0),
                    _ => (87, 0),
                }
            })
        })
        .collect();
    let bytes = numeric_chunk(sections, Some(1));
    for roof in [Roof::Open, Roof::Ceiling] {
        let surface = surface(&bytes, roof, None);
        assert!(
            surface
                .iter()
                .all(|column| *column == legacy_top(87, 0, 39)),
            "{roof:?}"
        );
    }
}

/// Whether a real chunk is finished by its own status; one of 1.12.2 and earlier has none
/// and is always drawn.
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
    match status {
        None => root.tag.get("Level").is_some(),
        Some(Tag::String(s)) => s == b"minecraft:full" || s == b"full",
        Some(_) => false,
    }
}

/// The numeric id and data value at `x`, `y`, `z` of a real chunk of 1.12.2 and earlier.
fn real_numeric_block(bytes: &[u8], x: usize, y: i16, z: usize) -> Option<(u16, u8)> {
    let root = nbt::read(bytes).ok()?;
    let Some(Tag::List { items, .. }) = root.tag.get("Level")?.get("Sections") else {
        return None;
    };
    let section = items
        .iter()
        .find(|section| section.get("Y") == Some(&Tag::Byte((y / 16) as i8)))?;
    let i = (y as usize % 16) * 256 + z * 16 + x;
    let nibble = |key: &str| match section.get(key) {
        Some(Tag::ByteArray(array)) => (array[i / 2] >> ((i % 2) * 4)) & 0xF,
        _ => 0,
    };
    let Some(Tag::ByteArray(blocks)) = section.get("Blocks") else {
        return None;
    };
    Some((
        u16::from(blocks[i]) | u16::from(nibble("Add")) << 8,
        nibble("Data"),
    ))
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
    for (world, numeric) in [
        ("1.12.2/region", true),
        ("1.16.5/region", false),
        ("1.20.1/region", false),
        ("1.21.1/region", false),
        ("26.3/dimensions/minecraft/overworld/region", false),
    ] {
        // A world of numeric ids is read with its own id list, as the map reads it.
        let legacy = numeric.then(|| {
            let level_dat = root.join(world).with_file_name("level.dat");
            let inflated = super::super::super::world_level::inflate_level_dat(&level_dat).unwrap();
            LegacyIds::from_level_dat(&inflated)
        });
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
        let mut ids = std::collections::HashMap::<(u16, u8), (usize, Rgb)>::new();
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
                    let surface = surface(&bytes, Roof::Open, legacy.as_ref());
                    if !real_status_is_full(&bytes) {
                        continue;
                    }
                    full += 1;
                    columns += 256;
                    for (i, top) in surface.iter().enumerate() {
                        let Some(top) = top else {
                            continue;
                        };
                        shown += 1;
                        grey += usize::from(top.color == UNKNOWN && !top.water);
                        world_water |= top.water;
                        world_colors.insert(top.color);
                        if numeric {
                            let id = real_numeric_block(&bytes, i % 16, top.y, i / 16)
                                .expect("the top block");
                            ids.entry(id).or_insert((0, top.color)).0 += 1;
                        }
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
        if numeric {
            let mut ids: Vec<_> = ids.into_iter().collect();
            ids.sort_by_key(|(id, (count, _))| (std::cmp::Reverse(*count), *id));
            println!("{world}: most frequent top blocks (id:data count colour):");
            for ((id, data), (count, [r, g, b])) in ids.iter().take(10) {
                println!("  {id}:{data} {count} #{r:02x}{g:02x}{b:02x}");
            }
        }
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
