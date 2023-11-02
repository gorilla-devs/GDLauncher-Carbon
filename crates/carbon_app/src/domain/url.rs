use serde::{Serialize, Serializer};

pub fn serialize_as_raw_json<S, T>(value: T, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let json =
        serde_json::to_string(&value).map_err(serde::ser::Error::custom)?;
    s.serialize_str(&json)
}
