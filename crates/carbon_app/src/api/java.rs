use crate::api::keys::java::*;
use crate::api::managers::Managers;
use crate::api::router::router;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Type, Serialize)]
enum JavaType {
    Local,
    Controlled,
}

#[derive(Type, Serialize)]
struct Java {
    default_id: String,
    java: Vec<JavaDetails>,
}

#[derive(Type, Serialize)]
struct JavaDetails {
    id: String,
    version: String,
    path: PathBuf,
    #[serde(rename = "type")]
    _type: JavaType,
}

#[derive(Type, Serialize)]
struct Javas(HashMap<u8, Java>);

#[derive(Type, Deserialize)]
struct SetupArgs {
    major_version: u8,
    #[serde(rename = "type")]
    _type: AutoSetupTypes,
}

#[derive(Type, Deserialize)]
enum AutoSetupTypes {
    AdoptOpenJDK,
    MojangJDK,
}

#[derive(Type, Deserialize)]
struct SetDefaultArgs {
    major_version: u8,
    id: String,
}

pub(super) fn mount() -> impl RouterBuilderLike<Managers> {
    router! {
        query GET_AVAILABLE[_, _args: ()] {
            let mut javas = HashMap::new();
            let mut java8 = Java {
                default_id: "vseuitruihsruthurt".to_string(),
                java: Vec::new(),
            };
            java8.java.push(JavaDetails {
                id: "vseuitruihsruthurt".to_string(),
                version: "1.8.0_51".to_string(),
                path: PathBuf::from("C:\\Program Files\\Java\\jre1.8.0_51"),
                _type: JavaType::Local,
            });
            java8.java.push(JavaDetails {
                id: "vxeuwwruihhrtthurt".to_string(),
                version: "1.8.0_55".to_string(),
                path: PathBuf::from("C:\\Program Files\\Java\\jre1.8.0_55"),
                _type: JavaType::Local,
            });
            javas.insert(8, java8);

            let mut java11 = Java {
                default_id: "vseuitruihsruthurt12".to_string(),
                java: Vec::new(),
            };
            java11.java.push(JavaDetails {
                id: "vseuitruihsruthurt12".to_string(),
                version: "11.0.1".to_string(),
                path: PathBuf::from("C:\\Program Files\\Java\\jre1.8.0_51"),
                _type: JavaType::Local,
            });
            java11.java.push(JavaDetails {
                id: "vseuitruihsruuuuuugg".to_string(),
                version: "11.0.1".to_string(),
                path: PathBuf::from("C:\\Some Path\\\\AppData\\gdlauncher\\Java\\jre1.8.0_51"),
                _type: JavaType::Controlled,
            });

            javas.insert(11, java11);

            Ok(Javas(javas))
        }

        mutation SET_DEFAULT[_, _args: SetDefaultArgs] {}

        mutation SETUP_CONTROLLED[_, _args: SetupArgs] {
            // invalidate_query!("java.autoSetupjavaProgress");
        }

        query GET_CONTROLLED_INSTALL_STATUS[_, _args: ()] {
            Ok(0) // progress
        }

        mutation DELETE_CONTROLLED[_, _major_version: u8] {}
    }
}
