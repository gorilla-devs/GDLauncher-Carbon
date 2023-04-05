use crate::api::managers::App;
use crate::api::router::router;
use crate::{api::keys::java::*, managers::java::JavaComponent};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_AVAILABLE[app, _args: ()] {
            let all_javas = app.java_manager().get_available_javas().await?;

            Ok(all_javas)
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
}

#[derive(Type, Serialize)]
enum FEJavaComponentType {
    Local,
    Managed,
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
