use std::sync::Arc;

use carbon_domain::{
    maven::MavenCoordinates,
    minecraft::{
        manifest::ManifestVersion,
        version::{Libraries, Version},
    },
};
use prisma_client_rust::QueryError;
use regex::{Captures, Regex};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use thiserror::Error;

use crate::{
    db::PrismaClient,
    managers::{
        configuration::runtime_path::{NativesPath, RuntimePath},
        AppRef, Managers, ManagersInner,
    },
};

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("Could not fetch version meta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Could not execute db query: {0}")]
    QueryError(#[from] QueryError),
}

pub async fn get_meta(
    db: Arc<PrismaClient>,
    manifest_version_meta: ManifestVersion,
) -> Result<Version, VersionError> {
    let url = manifest_version_meta.url;

    let version_meta = reqwest::get(url).await?.json::<Version>().await?;

    let bytes = serde_json::to_vec(&version_meta).unwrap();

    db.minecraft_version()
        .upsert(
            crate::db::minecraft_version::id::equals(version_meta.id.clone()),
            crate::db::minecraft_version::create(version_meta.id.clone(), bytes.clone(), vec![]),
            vec![crate::db::minecraft_version::json::set(bytes)],
        )
        .exec()
        .await?;

    Ok(version_meta)
}

#[derive(EnumIter, Debug, PartialEq)]
enum ArgPlaceholder {
    AuthPlayerName,
    VersionName,
    GameDirectory,
    AssetsRoot,
    GameAssets,
    AssetsIndexName,
    AuthUuid,
    AuthAccessToken,
    AuthSession,
    UserType,
    VersionType,
    UserProperties,
}

impl ArgPlaceholder {
    fn from_str(s: &str) -> Option<ArgPlaceholder> {
        match s {
            "auth_player_name" => Some(ArgPlaceholder::AuthPlayerName),
            "version_name" => Some(ArgPlaceholder::VersionName),
            "game_directory" => Some(ArgPlaceholder::GameDirectory),
            "assets_root" => Some(ArgPlaceholder::AssetsRoot),
            "game_assets" => Some(ArgPlaceholder::GameAssets),
            "assets_index_name" => Some(ArgPlaceholder::AssetsIndexName),
            "auth_uuid" => Some(ArgPlaceholder::AuthUuid),
            "auth_access_token" => Some(ArgPlaceholder::AuthAccessToken),
            "auth_session" => Some(ArgPlaceholder::AuthSession),
            "user_type" => Some(ArgPlaceholder::UserType),
            "version_type" => Some(ArgPlaceholder::VersionType),
            "user_properties" => Some(ArgPlaceholder::UserProperties),
            _ => None,
        }
    }

    fn get_argument(&self) -> String {
        match self {
            ArgPlaceholder::AuthPlayerName => "--username".to_owned(),
            ArgPlaceholder::VersionName => "--version".to_owned(),
            ArgPlaceholder::GameDirectory => "--gameDir".to_owned(),
            ArgPlaceholder::AssetsRoot => "--assetsDir".to_owned(),
            ArgPlaceholder::GameAssets => "--gameAssets".to_owned(),
            ArgPlaceholder::AssetsIndexName => "--assetIndex".to_owned(),
            ArgPlaceholder::AuthUuid => "--uuid".to_owned(),
            ArgPlaceholder::AuthAccessToken => "--accessToken".to_owned(),
            ArgPlaceholder::AuthSession => "--userProperties".to_owned(),
            ArgPlaceholder::UserType => "--userType".to_owned(),
            ArgPlaceholder::VersionType => "--versionType".to_owned(),
            ArgPlaceholder::UserProperties => "--userProperties".to_owned(),
        }
    }

    fn get_placeholder(&self) -> String {
        match self {
            ArgPlaceholder::AuthPlayerName => "auth_player_name".to_owned(),
            ArgPlaceholder::VersionName => "version_name".to_owned(),
            ArgPlaceholder::GameDirectory => "game_directory".to_owned(),
            ArgPlaceholder::AssetsRoot => "assets_root".to_owned(),
            ArgPlaceholder::GameAssets => "game_assets".to_owned(),
            ArgPlaceholder::AssetsIndexName => "assets_index_name".to_owned(),
            ArgPlaceholder::AuthUuid => "auth_uuid".to_owned(),
            ArgPlaceholder::AuthAccessToken => "auth_access_token".to_owned(),
            ArgPlaceholder::AuthSession => "auth_session".to_owned(),
            ArgPlaceholder::UserType => "user_type".to_owned(),
            ArgPlaceholder::VersionType => "version_type".to_owned(),
            ArgPlaceholder::UserProperties => "user_properties".to_owned(),
        }
    }
}

fn get_default_placeholder(version: &Version) -> String {
    // TODO: Do more logic to only add required arguments for the version
    let mut args = String::new();
    for arg in ArgPlaceholder::iter() {
        args.push_str(&format!("{} {}", arg.get_argument(), arg.get_placeholder()));
    }

    args
}

fn replace_placeholder(app: Arc<ManagersInner>, placeholder: &str, version: &Version) -> String {
    // let player_name = app.account_manager.get_player_name();
    let version_name = version.id.clone();
    let game_directory = app
        .configuration_manager
        .get_runtime_path()
        .get_instances()
        .get_instance_path("something".to_owned());
    let assets_root = app
        .configuration_manager
        .get_runtime_path()
        .get_assets()
        .to_path();
    let game_assets = app
        .configuration_manager
        .get_runtime_path()
        .get_assets()
        .get_legacy_path();
    let assets_index_name = version.assets.clone();

    match placeholder {
        "auth_player_name" => unimplemented!(),
        "version_name" => unimplemented!(),
        "game_directory" => unimplemented!(),
        "assets_root" => unimplemented!(),
        "game_assets" => unimplemented!(),
        "assets_index_name" => unimplemented!(),
        "auth_uuid" => unimplemented!(),
        "auth_access_token" => unimplemented!(),
        "auth_session" => unimplemented!(),
        "user_type" => unimplemented!(),
        "version_type" => unimplemented!(),
        "user_properties" => unimplemented!(),
        _ => todo!(),
    }
}

fn replace_placeholders(app: Arc<ManagersInner>, command: String, version: &Version) -> String {
    let matches =
        Regex::new(r"--(?P<arg>\S+)\s+\$\{(?P<value>[^}]+)\}|(\$\{(?P<standalone>[^}]+)\})")
            .unwrap();

    let new_command = matches.replace_all(&command, |caps: &Captures| {
        if let Some(value) = caps.name("value") {
            let value = replace_placeholder(app.clone(), value.as_str(), version);
            return format!("{} {}", caps.name("arg").unwrap().as_str(), value);
        } else if let Some(standalone) = caps.name("standalone") {
            let value = replace_placeholder(app.clone(), standalone.as_str(), version);
            return value;
        }
        if let Some(arg) = caps.name("arg") {
            return arg.as_str().to_string();
        } else {
            unreachable!("No capturing group matched")
        }
    });

    new_command.to_string()
}

pub fn generate_startup_command(
    app: Arc<ManagersInner>,
    version: Version,
    runtime_path: RuntimePath,
) -> String {
    let libraries = version.libraries.as_ref().unwrap();

    let mut command = Vec::with_capacity(libraries.get_libraries().len() * 2);
    command.push("java".to_owned());
    command.push(
        "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump"
            .to_owned(),
    );
    command.push("-Dos.name=Windows 10".to_owned());
    command.push("-Dos.version=10.0".to_owned());
    let natives_path = runtime_path.get_natives().get_versioned(&version.id);
    let natives_path = format!("-Djava.library.path={}", natives_path.to_string_lossy());
    command.push(natives_path);
    command.push("-cp".to_owned());

    let classpath_separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };

    for library in libraries.get_libraries() {
        let path = runtime_path
            .get_libraries()
            .get_library_path(MavenCoordinates::try_from(library.name.clone()).unwrap());

        command.push(format!("{}{}", path.display(), classpath_separator));
    }

    command.push("-Xmx4096m".to_string());
    command.push("-Xms4096m".to_string());

    command.push(format!(
        "-Dminecraft.applet.TargetDirectory={}",
        runtime_path.get_root().to_path().display()
    ));

    // command.push("-Dlog4j.configurationFile=C:\Users\david\AppData\Roaming\gdlauncher_next\datastore\assets\objects\bd\client-1.12.xml".to_owned());

    command.push("-Dfml.ignorePatchDiscrepancies=true".to_owned());
    command.push("-Dfml.ignoreInvalidMinecraftCertificates=true".to_owned());

    command.push(version.main_class.clone());

    // check if arguments.jvm is there, otherwise inject defaults

    match &version.arguments {
        Some(arguments) => {
            if let Some(jvm) = &arguments.jvm {
                command.push("".to_string());
            } else {
                get_default_placeholder(&version);
            }
        }
        None => {
            get_default_placeholder(&version);
        }
    }

    // command.push("--username killpowa --version 1.19.3 --gameDir ..\..\instances\Minecraft vanilla --assetsDir ..\..\datastore\assets --assetIndex 2 --uuid 3b40f99969e64dbcabd01f87cddcb1fd --accessToken __HIDDEN_TOKEN__ --clientId ${clientid} --xuid ${auth_xuid} --userType mojang --versionType release --width=854 --height=480".to_owned());

    command.join(" ")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::setup_managers_for_test;

    use super::*;

    // #[tokio::test]
    // async fn test_replace_placeholder() {
    //     let app = setup_managers_for_test().await;
    //     let version = Version::default();
    //     let placeholder = "auth_player_name";
    //     let result = replace_placeholder(app, placeholder, &version);
    //     assert_eq!(result, "killpowa");
    // }

    // #[tokio::test]
    // async fn test_replace_placeholders() {
    //     let app = setup_managers_for_test().await;
    //     let version = Version::default();
    //     let command = "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --clientId ${clientid} --xuid ${auth_xuid} --userType ${user_type} --versionType ${version_type} --width=854 --height=480".to_owned();
    //     let result = replace_placeholders(app, command, &version);
    //     assert_eq!(result, "--username killpowa --version 1.19.3 --gameDir ..\\..\\instances\\Minecraft vanilla --assetsDir ..\\..\\datastore\\assets --assetIndex 2 --uuid 3b40f99969e64dbcabd01f87cddcb1fd --accessToken __HIDDEN_TOKEN__ --clientId ${clientid} --xuid ${auth_xuid} --userType mojang --versionType release --width=854 --height=480".to_owned());
    // }
}
