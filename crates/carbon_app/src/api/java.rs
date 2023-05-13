use crate::api::managers::App;
use crate::api::router::router;
use crate::domain::java::JavaComponentType;
use crate::{api::keys::java::*, domain::java::Java};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_ALL_AVAILABLE_JAVAS[app, _args: ()] {
            get_all_available_javas(app, _args).await
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

async fn get_all_available_javas(
    app: App,
    _args: (),
) -> anyhow::Result<HashMap<u8, Vec<FEJavaComponent>>> {
    let all_javas = app.java_manager().get_available_javas().await?;

    let mut result = HashMap::new();
    for (major, javas) in all_javas {
        result.insert(
            major,
            javas.into_iter().map(FEJavaComponent::from).collect(),
        );
    }

    Ok(result)
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct Javas(HashMap<u8, Vec<FEJavaComponent>>);

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
struct SetupArgs {
    major_version: u8,
    #[serde(rename = "type")]
    _type: AutoSetupTypes,
}

#[derive(Type, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AutoSetupTypes {
    AdoptOpenJDK,
    MojangJDK,
}

#[derive(Type, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetDefaultArgs {
    major_version: u8,
    id: String,
}
