use super::Ctx;
use rspc::{Router, RouterBuilderLike, Type};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Type, Serialize)]
struct Java {
    name: String,
    path: PathBuf,
}

pub(super) fn mount() -> impl RouterBuilderLike<()> {
    Router::new().query("getAvailableJavas", |t| {
        t(|ctx: (), _args: ()| async move {
            // let javas = ctx.java_manager.get_available_javas().await?;
            let java = Java {
                name: "Java 8".to_string(),
                path: PathBuf::from("/usr/lib/jvm/java-8-openjdk-amd64"),
            };
            Ok(java)
        })
    })
}
