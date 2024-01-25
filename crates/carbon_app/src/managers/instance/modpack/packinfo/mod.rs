use std::collections::HashMap;

use serde::{Deserialize, Serialize};

mod scan;
mod v1;

pub use scan::scan_dir;

pub struct PackInfo {
    pub files: HashMap<String, FileHashes>,
}

#[derive(Debug, Clone)]
pub struct FileHashes {
    pub sha512: [u8; 64],
    pub md5: [u8; 16],
    pub murmur2: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "_version")]
enum PackInfoWrapper {
    #[serde(rename = "1")]
    V1(v1::PackInfo),
}

pub fn parse_packinfo(packinfo_str: &str) -> Result<PackInfo, serde_json::Error> {
    let packinfo = serde_json::from_str::<PackInfoWrapper>(packinfo_str)?;

    Ok(match packinfo {
        PackInfoWrapper::V1(packinfo) => packinfo.into(),
    })
}

pub fn make_packinfo(info: PackInfo) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&PackInfoWrapper::V1(info.into()))
}
