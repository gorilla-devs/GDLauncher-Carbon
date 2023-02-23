use std::sync::Arc;

use carbon_domain::{
    maven::MavenCoordinates,
    minecraft::{
        manifest::ManifestVersion,
        version::{Libraries, Version},
    },
};
use prisma_client_rust::QueryError;
use regex::Regex;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use thiserror::Error;

use crate::{
    db::PrismaClient,
    managers::configuration::runtime_path::{NativesPath, RuntimePath},
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
    // fn get_matching_enum

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
            ArgPlaceholder::AuthPlayerName => "${auth_player_name}".to_owned(),
            ArgPlaceholder::VersionName => "${version_name}".to_owned(),
            ArgPlaceholder::GameDirectory => "${game_directory}".to_owned(),
            ArgPlaceholder::AssetsRoot => "${assets_root}".to_owned(),
            ArgPlaceholder::GameAssets => "${game_assets}".to_owned(),
            ArgPlaceholder::AssetsIndexName => "${assets_index_name}".to_owned(),
            ArgPlaceholder::AuthUuid => "${auth_uuid}".to_owned(),
            ArgPlaceholder::AuthAccessToken => "${auth_access_token}".to_owned(),
            ArgPlaceholder::AuthSession => "${auth_session}".to_owned(),
            ArgPlaceholder::UserType => "${user_type}".to_owned(),
            ArgPlaceholder::VersionType => "${version_type}".to_owned(),
            ArgPlaceholder::UserProperties => "${user_properties}".to_owned(),
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

fn replace_placeholders(command: String) -> String {
    let matches =
        Regex::new(r"--(?P<name>\S+)\s+\$\{(?P<value>[^}]+)\}|(\$\{(?P<standalone>[^}]+)\})")
            .unwrap();

    for cap in matches.captures_iter(&command) {
        if let Some(name) = cap.name("name") {
            println!("Name: {}", name.as_str());
        }
        if let Some(value) = cap.name("value") {
            println!("Value: {}", value.as_str());
        }
        if let Some(standalone) = cap.name("standalone") {
            println!("Standalone: {}", standalone.as_str());
        }
    }

    todo!()
}

pub fn generate_startup_command(version: Version, runtime_path: RuntimePath) -> String {
    let libraries = version.libraries.unwrap();

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

    for library in libraries.get_libraries() {
        let path = runtime_path
            .get_libraries()
            .get_library_pathbuf(MavenCoordinates::try_from(library.name.clone()).unwrap());

        command.push(format!("{};", path.display()));
    }

    command.push("-Xmx4096m".to_string());
    command.push("-Xms4096m".to_string());

    command.push(format!(
        "-Dminecraft.applet.TargetDirectory={}",
        runtime_path.get_root().to_pathbuf().display()
    ));

    // command.push("-Dlog4j.configurationFile=C:\Users\david\AppData\Roaming\gdlauncher_next\datastore\assets\objects\bd\client-1.12.xml".to_owned());

    command.push("-Dfml.ignorePatchDiscrepancies=true".to_owned());
    command.push("-Dfml.ignoreInvalidMinecraftCertificates=true".to_owned());

    command.push("net.minecraft.client.main.Main".to_owned());

    // check if arguments.jvm is there, otherwise inject defaults

    match version.arguments {
        Some(arguments) => {
            if let Some(jvm) = arguments.jvm {
                command.push("".to_string());
            } else {
                get_default_placeholder();
            }
        }
        None => {
            get_default_placeholder();
        }
    }

    // command.push("--username killpowa --version 1.19.3 --gameDir ..\..\instances\Minecraft vanilla --assetsDir ..\..\datastore\assets --assetIndex 2 --uuid 3b40f99969e64dbcabd01f87cddcb1fd --accessToken __HIDDEN_TOKEN__ --clientId ${clientid} --xuid ${auth_xuid} --userType mojang --versionType release --width=854 --height=480".to_owned());

    command.join(" ")
}
