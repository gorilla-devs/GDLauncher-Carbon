use std::{array, collections::HashMap};

use serde::{
    de::Visitor,
    de::{self, Deserializer},
    Deserialize, Serialize, Serializer,
};

#[derive(Serialize, Deserialize)]
pub struct PackInfo {
    pub files: HashMap<String, FileHashes>,
}

#[derive(Serialize, Deserialize)]
pub struct FileHashes {
    pub sha512: HexSha512,
    pub md5: HexMd5,
}

/// Will panic when serialized if N2 != N * 2
#[derive(Debug, Clone, Copy)]
pub struct Hex<const N: usize, const N2: usize>([u8; N]);

pub type HexSha512 = Hex<64, { 64 * 2 }>;
pub type HexMd5 = Hex<16, { 16 * 2 }>;

impl<'a, const N: usize, const N2: usize> Deserialize<'a> for Hex<N, N2> {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        struct HexVisitor<const N: usize, const N2: usize>;

        impl<'a, const N: usize, const N2: usize> Visitor<'a> for HexVisitor<N, N2> {
            type Value = Hex<N, N2>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "{N} byte hexidecimal string")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                let mut buf = [0u8; N];

                hex::decode_to_slice(v, &mut buf[..])
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))?;

                Ok(Hex(buf))
            }
        }

        deserializer.deserialize_str(HexVisitor::<N, N2>)
    }
}

impl<const N: usize, const N2: usize> Serialize for Hex<N, N2> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut buf = [0u8; N2];
        hex::encode_to_slice(self.0, &mut buf);

        // SAFETY: encode_to_slice will write 2N bytes of valid utf8
        let strbuf = unsafe { std::str::from_utf8_unchecked(&mut buf) };

        serializer.serialize_str(strbuf)
    }
}

impl From<PackInfo> for super::PackInfo {
    fn from(value: PackInfo) -> Self {
        Self {
            files: value
                .files
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}

impl From<FileHashes> for super::FileHashes {
    fn from(value: FileHashes) -> Self {
        Self {
            sha512: value.sha512.0,
            md5: value.md5.0,
        }
    }
}

impl From<super::PackInfo> for PackInfo {
    fn from(value: super::PackInfo) -> Self {
        Self {
            files: value
                .files
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}

impl From<super::FileHashes> for FileHashes {
    fn from(value: super::FileHashes) -> Self {
        Self {
            sha512: Hex(value.sha512),
            md5: Hex(value.md5),
        }
    }
}
