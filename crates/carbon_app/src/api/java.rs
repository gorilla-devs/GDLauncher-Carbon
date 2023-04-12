use crate::api::managers::App;
use crate::api::router::router;
use crate::domain::java::JavaComponentType;
use crate::{api::keys::java::*, domain::java::Java};
use anyhow::anyhow;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_AVAILABLE[app, _args: ()] {
            let all_javas = app.java_manager().get_available_javas().await?;
            let default_javas = app.java_manager().get_default_javas().await?;

            let mut result = HashMap::new();
            for (major, javas) in all_javas {
                result.insert(major, FEAvailableJavas {
                    default_id: default_javas.get(&major).map(|s| s.to_string()).ok_or(anyhow!("No default java for major version {}", major))?,
                    javas: javas.into_iter().map(FEJavaComponent::from).collect(),
                });
            }

            Ok(result)
        }

        mutation SET_DEFAULT[_, _args: SetDefaultArgs] { Ok(()) }

        mutation SETUP_CONTROLLED[_, _args: SetupArgs] {
            // invalidate_query!("java.autoSetupjavaProgress");
            Ok(())
        }

        query GET_CONTROLLED_INSTALL_STATUS[_, _args: ()] {
            Ok(0) // progress
        }

        mutation DELETE_CONTROLLED[_, _major_version: u8] { Ok(()) }
    }
}

#[derive(Type, Serialize)]
struct Javas(HashMap<u8, FEAvailableJavas>);

#[derive(Type, Serialize)]
struct FEAvailableJavas {
    default_id: String,
    javas: Vec<FEJavaComponent>,
}

#[derive(Type, Serialize)]
struct FEJavaComponent {
    path: String,
    version: String,
    #[serde(rename = "type")]
    _type: FEJavaComponentType,
    is_valid: bool,
}

impl From<Java> for FEJavaComponent {
    fn from(java: Java) -> Self {
        Self {
            path: java.component.path,
            version: String::from(java.component.version),
            _type: FEJavaComponentType::from(java.component._type),
            is_valid: java.is_valid,
        }
    }
}

#[derive(Type, Serialize)]
enum FEJavaComponentType {
    Local,
    Managed,
    Custom,
}

impl From<JavaComponentType> for FEJavaComponentType {
    fn from(t: JavaComponentType) -> Self {
        match t {
            JavaComponentType::Local => Self::Local,
            JavaComponentType::Managed => Self::Managed,
            JavaComponentType::Custom => Self::Custom,
        }
    }
}

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
