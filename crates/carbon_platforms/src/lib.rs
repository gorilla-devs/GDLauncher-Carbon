use std::{fmt, str::FromStr};

use anyhow::{anyhow, bail};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{
    de::{DeserializeOwned, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

pub mod curseforge;
pub mod modrinth;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum ModChannel {
    Alpha = 0,
    Beta,
    Stable,
}

impl ModChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModChannel::Alpha => "alpha",
            ModChannel::Beta => "beta",
            ModChannel::Stable => "stable",
        }
    }

    fn all() -> [Self; 3] {
        [Self::Stable, Self::Beta, Self::Alpha]
    }
}

impl TryFrom<i32> for ModChannel {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::all()
            .get(value as usize)
            .map(|v| *v)
            .ok_or_else(|| anyhow!("invalid mod channel id {value}"))
    }
}

impl FromStr for ModChannel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "alpha" => Ok(Self::Alpha),
            "beta" => Ok(Self::Beta),
            "stable" => Ok(Self::Stable),
            _ => Err(anyhow!("unexpected ModChannel '{s}'")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ModPlatform {
    Curseforge,
    Modrinth,
}

impl ModPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Curseforge => "curseforge",
            Self::Modrinth => "modrinth",
        }
    }
}

impl FromStr for ModPlatform {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "curseforge" => Ok(Self::Curseforge),
            "modrinth" => Ok(Self::Modrinth),
            _ => Err(anyhow!("unexpected ModPlatform '{s}'")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ModChannelWithUsage {
    pub channel: ModChannel,
    pub allow_updates: bool,
}

impl ModChannelWithUsage {
    pub fn slice_to_str(channels: &[Self]) -> String {
        channels.iter().map(ToString::to_string).join(",")
    }

    pub fn str_to_vec(s: &str) -> anyhow::Result<Vec<Self>> {
        s.split(",")
            .filter(|v| !v.is_empty())
            .map(FromStr::from_str)
            .collect::<Result<Vec<_>, _>>()
    }

    /// Validate the channel list is correct.
    ///
    /// Use this when parsing a new value for the channel list.
    pub fn validate_list(channels: &[Self]) -> anyhow::Result<()> {
        let has_duplicates = channels
            .iter()
            .enumerate()
            .any(|(i, c)| channels[..i].iter().any(|c2| c2.channel == c.channel));

        if has_duplicates {
            bail!("channel list {channels:?} contains the same channel multiple times")
        }

        if channels.len() < ModChannel::all().len() {
            bail!("channel list {channels:?} is missing channels")
        }

        Ok(())
    }

    /// Fixup the given list if it is missing entries.
    ///
    /// Adds any missing channel entries in order with updates disabled.
    pub fn fixup_list(channels: &mut Vec<Self>) {
        let missing_channels = ModChannel::all()
            .into_iter()
            .filter(|c| !channels.iter().any(|c2| c2.channel == *c))
            .collect::<Vec<_>>();

        channels.extend(missing_channels.into_iter().map(|c| ModChannelWithUsage {
            channel: c,
            allow_updates: false,
        }));
    }
}

impl FromStr for ModChannelWithUsage {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (channel, updates) = s.split_once(':').ok_or_else(|| {
            anyhow!("ModChannelWithUsage string '{s}' was not in the form 'channel:updates'")
        })?;

        Ok(Self {
            channel: ModChannel::from_str(channel)?,
            allow_updates: bool::from_str(updates)?,
        })
    }
}

impl ToString for ModChannelWithUsage {
    fn to_string(&self) -> String {
        format!("{}:{}", self.channel.as_str(), self.allow_updates)
    }
}

#[derive(Debug, Clone)]
pub struct ModSources {
    pub channels: Vec<ModChannelWithUsage>,
    pub platform_blacklist: Vec<ModPlatform>,
}

pub enum RemoteVersion {
    Curseforge(curseforge::File),
    Modrinth(modrinth::version::Version),
}

impl RemoteVersion {
    pub fn date(&self) -> DateTime<Utc> {
        match self {
            Self::Curseforge(v) => v.file_date,
            Self::Modrinth(v) => v.date_published,
        }
    }

    pub fn channel(&self) -> ModChannel {
        match self {
            Self::Curseforge(v) => v.release_type.into(),
            Self::Modrinth(v) => v.version_type.into(),
        }
    }
}

impl PartialEq for RemoteVersion {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.date(), &other.date())
    }
}

impl Eq for RemoteVersion {}

impl PartialOrd for RemoteVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&other.date(), &self.date())
    }
}

impl Ord for RemoteVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&other.date(), &self.date())
    }
}

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

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};
    use serde_repr::{Deserialize_repr, Serialize_repr};

    #[test]
    fn test_into_query_parameters() {
        #[carbon_macro::into_query_parameters]
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchParameters {
            game_id: i32,
            page: Option<i32>,
            sort_order: Option<SortOrder>,
            class_id: Option<ClassId>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        enum SortOrder {
            #[serde(rename = "asc")]
            Ascending,
            #[serde(rename = "desc")]
            Descending,
        }

        #[derive(Debug, Serialize_repr, Deserialize_repr)]
        #[repr(u16)]
        enum ClassId {
            Mods = 6,
            Modpacks = 4471,
        }

        let search_params = SearchParameters {
            game_id: 432,
            page: None,
            sort_order: Some(SortOrder::Ascending),
            class_id: Some(ClassId::Mods),
        };

        let query = search_params.into_query_parameters().unwrap();

        assert_eq!(query, "gameId=432&sortOrder=asc&classId=6");
    }
}
