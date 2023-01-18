use super::Ctx;
use rspc::{Router, RouterBuilderLike, Type};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Type, Serialize)]
struct Java {
    id: String,
    name: String,
    version: String,
    path: PathBuf,
    default: bool,
}

#[derive(Type, Serialize)]
struct Javas(Vec<Java>);

pub(super) fn mount() -> impl RouterBuilderLike<()> {
    Router::new().query("getAvailableJavas", |t| {
        t(|ctx: (), _args: ()| async move {
            let mut javas = Vec::new();
            javas.push(Java {
                id: "java-8".to_string(),
                name: "Java 8".to_string(),
                version: "1.8.0_275".to_string(),
                path: PathBuf::from("/usr/lib/jvm/java-8-openjdk-amd64"),
                default: true,
            });
            javas.push(Java {
                id: "java-11".to_string(),
                name: "Java 11".to_string(),
                version: "1.11.0_33".to_string(),
                path: PathBuf::from("/usr/lib/jvm/java-11-openjdk-amd64"),
                default: true,
            });
            javas.push(Java {
                id: "java-11".to_string(),
                name: "Java 11".to_string(),
                version: "1.11.0_35".to_string(),
                path: PathBuf::from("/usr/lib/jvm/java-11_35-openjdk-amd64"),
                default: false,
            });
            javas.push(Java {
                id: "java-16".to_string(),
                name: "Java 16".to_string(),
                version: "1.16.0_33".to_string(),
                path: PathBuf::from("/usr/lib/jvm/java-16-openjdk-amd64"),
                default: false,
            });

            let final_javas = Javas(javas);

            Ok(final_javas)
        })
    })
}
