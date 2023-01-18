use super::Ctx;
use rspc::{Router, RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Type, Serialize)]
struct Instance {
    id: String,
    name: String,
}

#[derive(Type, Serialize)]
struct Instances(Vec<Instance>);

pub(super) fn mount() -> impl RouterBuilderLike<()> {
    Router::new()
        .query("getInstances", |t| {
            t(|ctx: (), _args: ()| async move {
                let mut instances = Vec::new();
                instances.push(Instance {
                    id: "88r39459345939453".to_string(),
                    name: "My first instance".to_string(),
                });
                instances.push(Instance {
                    id: "88r39459345939456".to_string(),
                    name: "My second instance".to_string(),
                });
                instances.push(Instance {
                    id: "88r39459345939451".to_string(),
                    name: "Instance with a very long name".to_string(),
                });
                instances.push(Instance {
                    id: "88r39459345336457".to_string(),
                    name: "Vanilla Minecraft".to_string(),
                });
                instances.push(Instance {
                    id: "84439459345336457".to_string(),
                    name: "Forge Minecraft".to_string(),
                });
                instances.push(Instance {
                    id: "82h39459345336457".to_string(),
                    name: "All The Mods 6".to_string(),
                });

                let final_instances = Instances(instances);

                Ok(final_instances)
            })
        })
        .query("getInstance", |t| {
            t(|ctx: (), args: String| async move {
                let instance = Instance {
                    id: "82h39459345336457".to_string(),
                    name: "All The Mods 6".to_string(),
                };

                Ok(instance)
            })
        })
        .mutation("updateInstanceName", |t| {
            #[derive(Type, Deserialize)]
            struct Args {
                id: String,
                new_name: String,
            }
            t(|_, args: Args| {})
        })
}
