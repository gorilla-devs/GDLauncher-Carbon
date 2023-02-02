use carbon_minecraft::instance::Instance;
use rspc::Type;
use serde::Serialize;
use std::time::UNIX_EPOCH;

#[derive(Type, Serialize)]
pub struct ModDetails {
    id: String,
    mod_name: String,
}

#[derive(Type, Serialize)]
pub struct ModLoadersDetails {
    mod_loader_name: String,
    mod_loader_version: String,
}

#[derive(Type, Serialize)]
pub struct InstanceDetails {
    id: String,
    name: String,
    mc_version: String,
    mod_loaders: Vec<ModLoadersDetails>,
    mods: Vec<ModDetails>,
    played_time: String,
    last_played: Option<String>,
    notes: String,
}

impl Into<InstanceDetails> for Instance {
    fn into(self) -> InstanceDetails {
        let instance = &self;
        let last_played = instance.last_played.map(|system_time| {
            system_time
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_millis().to_string())
                .unwrap_or("unable to retrieve last played time".to_string())
        });
        let mod_loaders = instance
            .minecraft_package
            .mod_loaders
            .iter()
            .map(|mod_loader| ModLoadersDetails {
                mod_loader_name: mod_loader.to_string(),
                mod_loader_version: mod_loader.get_version(),
            })
            .collect();
        let mut mods = instance
            .minecraft_package
            .mods
            .iter()
            .map(|minecraft_mod| ModDetails {
                id: minecraft_mod.id.clone().to_string(),
                mod_name: minecraft_mod.name.clone(),
            })
            .collect();
        InstanceDetails {
            id: instance.id.to_string(),
            name: instance.name.clone(),
            mc_version: instance.minecraft_package.version.clone(),
            mod_loaders,
            mods,
            played_time: instance.played_time.as_millis().to_string(),
            last_played,
            notes: self.notes,
        }
    }
}
