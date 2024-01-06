use std::{fmt, str::FromStr};

use itertools::Itertools;
use serde::{de::Visitor, Deserialize, Serialize};

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
}

impl FromStr for ModChannel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "alpha" => Ok(Self::Alpha),
            "beta" => Ok(Self::Beta),
            "stable" => Ok(Self::Stable),
            _ => Err(anyhow::anyhow!("unexpected ModChannel '{s}'")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlatformModChannel {
    Curseforge(ModChannel),
    Modrinth(ModChannel),
}

impl PlatformModChannel {
    pub fn list_to_string(pmcs: &[Self]) -> String {
        pmcs.iter().map(Self::to_string).join(",")
    }

    pub fn str_to_vec(pmcs: &str) -> anyhow::Result<Vec<Self>> {
        pmcs.split(',')
            .map(Self::from_str)
            .collect::<Result<Vec<_>, _>>()
    }
}

impl ToString for PlatformModChannel {
    fn to_string(&self) -> String {
        match self {
            Self::Curseforge(channel) => format!("curseforge.{}", channel.as_str(),),
            Self::Modrinth(channel) => format!("modrinth.{}", channel.as_str(),),
        }
    }
}

impl FromStr for PlatformModChannel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((platform, channel)) = s.split_once('.') else {
            return Err(anyhow::anyhow!(
                "PlatformModChannel string '{s}' was not in the form 'platform.channel'"
            ));
        };

        let channel = ModChannel::from_str(channel)?;

        match platform {
            "curseforge" => Ok(Self::Curseforge(channel)),
            "modrinth" => Ok(Self::Modrinth(channel)),
            _ => Err(anyhow::anyhow!(
                "PlatformModChannel platform '{platform}' not recognised"
            )),
        }
    }
}

impl<'a> Deserialize<'a> for PlatformModChannel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        struct PMCVisitor;

        impl<'a> Visitor<'a> for PMCVisitor {
            type Value = PlatformModChannel;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "platform mod channel string (<platform>.<channel>)"
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                PlatformModChannel::from_str(v).map_err(|e| serde::de::Error::custom(e))
            }
        }

        deserializer.deserialize_str(PMCVisitor)
    }
}

impl Serialize for PlatformModChannel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
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
