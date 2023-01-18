use super::Ctx;
use rspc::{Router, RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
#[derive(Type, Serialize)]
enum JavaType {
    Local,
    AutoSetup,
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

pub(super) fn mount() -> impl RouterBuilderLike<()> {
    Router::new()
        .query("getAvailableJavas", |t| {
            t(|ctx: (), _args: ()| async move {
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
                    default_id: "vseuitruihsruthurt".to_string(),
                    java: Vec::new(),
                };
                java11.java.push(JavaDetails {
                    id: "vseuitruihsruthurt".to_string(),
                    version: "11.0.1".to_string(),
                    path: PathBuf::from("C:\\Program Files\\Java\\jre1.8.0_51"),
                    _type: JavaType::Local,
                });
                java11.java.push(JavaDetails {
                    id: "vseuitruihsruuuuuu".to_string(),
                    version: "11.0.1".to_string(),
                    path: PathBuf::from("C:\\Some Path\\\\AppData\\gdlauncher\\Java\\jre1.8.0_51"),
                    _type: JavaType::AutoSetup,
                });

                javas.insert(11, java11);

                Ok(Javas(javas))
            })
        })
        .mutation("setDefaultJava", |t| {
            #[derive(Type, Deserialize)]
            struct Args {
                major_version: u8,
                id: String,
            }
            t(|_, args: Args| {})
        })
        .mutation("autoSetupJava", |t| {
            #[derive(Type, Deserialize)]
            struct Args {
                major_version: u8,
                #[serde(rename = "type")]
                _type: AutoSetupTypes,
            }

            #[derive(Type, Deserialize)]
            enum AutoSetupTypes {
                AdoptOpenJDK,
                MojangJDK,
            }

            t(|_, args: Args| {
                // invalidate_query!("java.autoSetupjavaProgress");
            })
        })
        .query("autoSetupjavaProgress", |t| {
            t(|ctx: (), _args: ()| async move {
                Ok(0) // progress
            })
        })
        .mutation("deleteAutoSetupJava", |t| {
            t(|_, major_version: u8| {
                // invalidate_query!("app.getTheme");
            })
        })
}
