use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize_as_raw_json<S, T>(value: T, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let json = serde_json::to_string(&value).map_err(serde::ser::Error::custom)?;
    s.serialize_str(&json)
}

pub fn deserialize_from_raw_json<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let json = String::deserialize(d)?;
    serde_json::from_str(&json).map_err(serde::de::Error::custom)
}
