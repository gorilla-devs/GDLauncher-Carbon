use std::path::PathBuf;
use rspc::Type;
use serde::Deserialize;

#[derive(Type, Deserialize)]
pub struct CreateInstanceDto {
    pub name: String,
    pub minecraft_version: String,
    pub path_to_save_at: Option<PathBuf>,
}
