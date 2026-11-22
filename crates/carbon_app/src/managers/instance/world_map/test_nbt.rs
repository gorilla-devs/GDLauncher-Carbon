//! Builders of NBT tags for the tests of the map readers.

use crate::util::nbt::{self, Tag};

pub(super) fn compound(entries: Vec<(&str, Tag)>) -> Tag {
    Tag::Compound(
        entries
            .into_iter()
            .map(|(key, tag)| (key.as_bytes().to_vec(), tag))
            .collect(),
    )
}

pub(super) fn string(text: &str) -> Tag {
    Tag::String(nbt::encode_string(text))
}

/// A list of compounds; an empty one has the element type the game writes for it.
pub(super) fn compounds(items: Vec<Tag>) -> Tag {
    Tag::List {
        element: if items.is_empty() { 0 } else { 10 },
        items,
    }
}
