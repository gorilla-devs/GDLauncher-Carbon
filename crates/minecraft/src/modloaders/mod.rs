pub mod vanilla;
pub mod forge;

type ModLoaderVersion = String;
pub enum ModLoaderType {
    Vanilla,
    Forge,
}

