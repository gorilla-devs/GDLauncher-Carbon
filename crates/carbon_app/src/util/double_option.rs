use serde::{Deserialize, Deserializer};

/// Deserialize an `Option<Option<T>>` field where an absent key means "leave
/// unchanged" and an explicit `null` means "clear it" (the "double option"
/// pattern). Plain derive(Deserialize) can't tell those apart: both an absent
/// key and an explicit `null` collapse to the outer `None`, so a value could
/// never actually be cleared. This function only runs when the key IS
/// present (paired with `#[serde(default)]`, which supplies the outer `None`
/// when it's absent), so a present `null` parses through `Option::deserialize`
/// (giving `None`) and gets wrapped in `Some`, and a present value becomes
/// `Some(Some(value))`.
///
/// Use as `#[serde(default, deserialize_with =
/// "crate::util::deserialize_double_option")]`.
pub fn deserialize_double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}
