use carbon_net::IntoVecDownloadable;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetIndex {
    pub objects: HashMap<String, Object>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Object {
    pub hash: String,
    pub size: i64,
}

impl IntoVecDownloadable for AssetIndex {
    fn into_vec_downloadable(self, base_path: &Path) -> Vec<carbon_net::Downloadable> {
        let mut files: Vec<carbon_net::Downloadable> = vec![];

        for (_, object) in self.objects.iter() {
            // TODO: handle directories for different versions (virtual legacy)
            let asset_path = base_path
                .join("assets")
                .join("objects")
                .join(&object.hash[0..2])
                .join(&object.hash);

            files.push(
                carbon_net::Downloadable::new(
                    format!(
                        "https://resources.download.minecraft.net/{}/{}",
                        &object.hash[0..2],
                        &object.hash
                    ),
                    asset_path,
                )
                .with_checksum(Some(carbon_net::Checksum::Sha1(object.hash.clone())))
                .with_size(object.size as u64),
            );
        }

        files
    }
}
