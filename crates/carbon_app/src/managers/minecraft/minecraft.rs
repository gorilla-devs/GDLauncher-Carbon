use std::{collections::HashMap, path::PathBuf};

use crate::domain::{
    maven::MavenCoordinates,
    minecraft::minecraft::{
        get_current_os, get_default_jvm_args, is_rule_allowed, library_is_allowed,
    },
};
use daedalus::minecraft::{
    Argument, ArgumentType, ArgumentValue, DownloadType, Library, Version, VersionInfo,
    VersionManifest,
};
use prisma_client_rust::QueryError;
use regex::{Captures, Regex};
use reqwest::Url;
use strum_macros::EnumIter;
use thiserror::Error;
use tokio::process::Child;

use crate::{
    domain::runtime_path::{InstancePath, RuntimePath},
    managers::account::{FullAccount, FullAccountType},
};

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("Could not fetch version meta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Could not execute db query: {0}")]
    QueryError(#[from] QueryError),
}

#[derive(Error, Debug)]
pub enum MinecraftManifestError {
    #[error("Could not fetch minecraft manifest from launchermeta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Manifest database query error: {0}")]
    DBQueryError(#[from] QueryError),
}

pub async fn get_manifest(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    meta_base_url: &Url,
) -> anyhow::Result<VersionManifest> {
    let server_url = meta_base_url.join("minecraft/v0/manifest.json")?;
    let new_manifest = reqwest_client
        .get(server_url)
        .send()
        .await?
        .json::<VersionManifest>()
        .await?;

    Ok(new_manifest)
}

pub async fn get_version(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    manifest_version_meta: Version,
) -> anyhow::Result<VersionInfo> {
    let url = manifest_version_meta.url;
    let version_meta = reqwest_client.get(url).send().await?.json().await?;

    Ok(version_meta)
}

pub async fn save_meta_to_disk(version: VersionInfo, clients_path: PathBuf) -> anyhow::Result<()> {
    tokio::fs::create_dir_all(&clients_path).await?;
    tokio::fs::write(
        clients_path.join(format!("{}.json", version.id)),
        serde_json::to_string(&version)?,
    )
    .await?;

    Ok(())
}

#[cfg(target_os = "windows")]
const CLASSPATH_SEPARATOR: &str = ";";
#[cfg(not(target_os = "windows"))]
const CLASSPATH_SEPARATOR: &str = ":";

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
    ClassPath,
    NativesDirectory,
    LauncherName,
    LauncherVersion,
}

impl From<&str> for ArgPlaceholder {
    fn from(arg: &str) -> Self {
        match arg {
            "auth_player_name" => ArgPlaceholder::AuthPlayerName,
            "version_name" => ArgPlaceholder::VersionName,
            "game_directory" => ArgPlaceholder::GameDirectory,
            "assets_root" => ArgPlaceholder::AssetsRoot,
            "game_assets" => ArgPlaceholder::GameAssets,
            "assets_index_name" => ArgPlaceholder::AssetsIndexName,
            "auth_uuid" => ArgPlaceholder::AuthUuid,
            "auth_access_token" => ArgPlaceholder::AuthAccessToken,
            "auth_session" => ArgPlaceholder::AuthSession,
            "user_type" => ArgPlaceholder::UserType,
            "version_type" => ArgPlaceholder::VersionType,
            "user_properties" => ArgPlaceholder::UserProperties,
            "classpath" => ArgPlaceholder::ClassPath,
            "natives_directory" => ArgPlaceholder::NativesDirectory,
            "launcher_name" => ArgPlaceholder::LauncherName,
            "launcher_version" => ArgPlaceholder::LauncherVersion,
            _ => panic!("Unknown argument placeholder: {arg}"),
        }
    }
}

impl From<ArgPlaceholder> for &str {
    fn from(arg: ArgPlaceholder) -> Self {
        match arg {
            ArgPlaceholder::AuthPlayerName => "auth_player_name",
            ArgPlaceholder::VersionName => "version_name",
            ArgPlaceholder::GameDirectory => "game_directory",
            ArgPlaceholder::AssetsRoot => "assets_root",
            ArgPlaceholder::GameAssets => "game_assets",
            ArgPlaceholder::AssetsIndexName => "assets_index_name",
            ArgPlaceholder::AuthUuid => "auth_uuid",
            ArgPlaceholder::AuthAccessToken => "auth_access_token",
            ArgPlaceholder::AuthSession => "auth_session",
            ArgPlaceholder::UserType => "user_type",
            ArgPlaceholder::VersionType => "version_type",
            ArgPlaceholder::UserProperties => "user_properties",
            ArgPlaceholder::ClassPath => "classpath",
            ArgPlaceholder::NativesDirectory => "natives_directory",
            ArgPlaceholder::LauncherName => "launcher_name",
            ArgPlaceholder::LauncherVersion => "launcher_version",
        }
    }
}

struct ReplacerArgs {
    player_name: String,
    player_token: String,
    version_name: String,
    game_directory: InstancePath,
    game_assets: PathBuf,
    target_directory: PathBuf,
    natives_path: PathBuf,
    assets_root: PathBuf,
    assets_index_name: String,
    auth_uuid: String,
    libraries: String,
    auth_access_token: String,
    auth_session: String,
    user_type: String,
    version_type: String,
    user_properties: String,
}

fn replace_placeholder(replacer_args: &ReplacerArgs, placeholder: ArgPlaceholder) -> String {
    match placeholder {
        ArgPlaceholder::AuthPlayerName => replacer_args.player_name.clone(),
        ArgPlaceholder::VersionName => replacer_args.version_name.clone(),
        ArgPlaceholder::GameDirectory => replacer_args
            .game_directory
            .get_data_path()
            .display()
            .to_string(),
        ArgPlaceholder::AssetsRoot => replacer_args.assets_root.display().to_string(),
        ArgPlaceholder::GameAssets => replacer_args.game_assets.display().to_string(),
        ArgPlaceholder::AssetsIndexName => replacer_args.assets_index_name.clone(),
        ArgPlaceholder::AuthUuid => replacer_args.auth_uuid.clone(),
        ArgPlaceholder::AuthAccessToken => replacer_args.auth_access_token.clone(),
        ArgPlaceholder::AuthSession => replacer_args.auth_session.clone(),
        ArgPlaceholder::UserType => replacer_args.user_type.clone(), // Hardcoded to mojang apparently ?????
        ArgPlaceholder::VersionType => replacer_args.version_type.clone(),
        ArgPlaceholder::UserProperties => replacer_args.user_properties.clone(), // Not sure what this is,
        ArgPlaceholder::ClassPath => replacer_args.libraries.clone(),
        ArgPlaceholder::NativesDirectory => replacer_args.natives_path.display().to_string(),
        ArgPlaceholder::LauncherName => "minecraft-launcher".to_string(),
        ArgPlaceholder::LauncherVersion => "2".to_string(),
    }
}

fn wraps_in_quotes_if_necessary(arg: impl AsRef<str>) -> String {
    let arg = arg.as_ref();
    if arg.contains('=') {
        let mut parts = arg.split('=');
        let key = parts.next().unwrap();
        let value = parts.next().unwrap();
        if value.contains(' ') {
            return format!("{}=\"{}\"", key, value);
        } else {
            return format!("{}={}", key, value);
        }
    }

    arg.to_string()
}

pub async fn generate_startup_command(
    full_account: FullAccount,
    xmx_memory: u64,
    xms_memory: u64,
    runtime_path: &RuntimePath,
    version: VersionInfo,
    instance_path: InstancePath,
) -> Vec<String> {
    let libraries = version
        .libraries
        .iter()
        .filter(|&library| library_is_allowed(library.clone()) && library.include_in_classpath)
        .map(|library| {
            let path = runtime_path
                .get_libraries()
                .get_library_path(MavenCoordinates::try_from(library.name.clone(), None).unwrap());

            path.display().to_string()
        })
        .reduce(|a, b| format!("{a}{CLASSPATH_SEPARATOR}{b}"))
        .unwrap();

    let mut command = Vec::with_capacity(15);

    command.push(format!("-Xmx{xmx_memory}m"));
    command.push(format!("-Xms{xms_memory}m"));

    let arguments = version.arguments.clone().unwrap_or_else(|| {
        let mut arguments = HashMap::new();
        arguments.insert(
            ArgumentType::Game,
            version
                .minecraft_arguments
                .unwrap()
                .split(' ')
                .map(|s| Argument::Normal(s.to_string()))
                .collect(),
        );

        arguments.insert(ArgumentType::Jvm, get_default_jvm_args());

        arguments
    });

    let game_arguments = arguments.get(&ArgumentType::Game).unwrap();
    let jvm_arguments = arguments.get(&ArgumentType::Jvm).unwrap();

    for arg in jvm_arguments.clone() {
        match arg {
            Argument::Normal(string) => command.push(string),
            Argument::Ruled { rules, value } => {
                let is_allowed = rules.iter().all(|rule| is_rule_allowed(rule.clone()));

                if is_allowed {
                    match value {
                        ArgumentValue::Single(string) => command.push(string),
                        ArgumentValue::Many(arr) => command.extend(arr),
                    }
                }
            }
        }
    }

    command.push(version.main_class.clone());

    for arg in game_arguments.clone() {
        match arg {
            Argument::Normal(string) => command.push(string),
            Argument::Ruled { rules, value } => {
                let is_allowed = rules.iter().all(|rule| is_rule_allowed(rule.clone()));

                if is_allowed {
                    match value {
                        ArgumentValue::Single(string) => command.push(string),
                        ArgumentValue::Many(arr) => command.extend(arr),
                    }
                }
            }
        }
    }

    let regex =
        Regex::new(r"--(?P<arg>\S+)\s+\$\{(?P<value>[^}]+)\}|(\$\{(?P<standalone>[^}]+)\})")
            .unwrap();

    let player_name = full_account.username;
    let player_uuid = full_account.uuid;
    let player_token = match full_account.type_ {
        FullAccountType::Offline => "offline".to_owned(),
        FullAccountType::Microsoft { access_token, .. } => access_token,
    };

    let version_name = version.id.clone();
    let game_directory = instance_path;
    let assets_root = runtime_path.get_assets().to_path();
    let game_assets = runtime_path.get_assets().to_path();
    let assets_index_name = version.assets.clone();
    let client_jar_path = runtime_path.get_versions().get_clients_path().join(format!(
        "{}.jar",
        version.downloads.get(&DownloadType::Client).unwrap().sha1
    ));

    let replacer_args = ReplacerArgs {
        player_name,
        player_token: player_token.clone(),
        version_name,
        game_directory,
        game_assets,
        target_directory: PathBuf::new(),
        natives_path: runtime_path.get_natives().get_versioned(&version.id),
        assets_root,
        assets_index_name,
        // Patch libraries adding client jar at the end
        libraries: format!(
            "{}{}{}",
            libraries,
            CLASSPATH_SEPARATOR,
            client_jar_path.display()
        ),
        auth_uuid: player_uuid,
        auth_access_token: player_token.clone(),
        auth_session: player_token,
        user_type: "mojang".to_owned(),
        version_type: version.type_.as_str().to_string(),
        user_properties: "{}".to_owned(),
    };

    command
        .into_iter()
        .map(|argument| {
            regex
                .replace_all(&argument, |caps: &Captures| {
                    if let Some(value) = caps.name("value") {
                        let value = replace_placeholder(&replacer_args, value.as_str().into());
                        return format!("--{} {}", caps.name("arg").unwrap().as_str(), value);
                    } else if let Some(standalone) = caps.name("standalone") {
                        let value = replace_placeholder(&replacer_args, standalone.as_str().into());
                        return value;
                    }
                    if let Some(arg) = caps.name("arg") {
                        return arg.as_str().to_string();
                    } else {
                        unreachable!("No capturing group matched")
                    }
                })
                .to_string()
        })
        .map(|argument| {
            // unescape " and \ characters
            argument.replace("\\\"", "\"").replace("\\\\", "\\")
        })
        .collect()
}

pub async fn launch_minecraft(
    java_binary: PathBuf,
    full_account: FullAccount,
    xmx_memory: u64,
    xms_memory: u64,
    runtime_path: &RuntimePath,
    version: VersionInfo,
    instance_path: InstancePath,
) -> anyhow::Result<Child> {
    let startup_command = generate_startup_command(
        full_account,
        xmx_memory,
        xms_memory,
        runtime_path,
        version,
        instance_path,
    )
    .await;

    println!("Starting Minecraft with command: {:?}", startup_command);

    let mut command_exec = tokio::process::Command::new(java_binary);

    command_exec
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = command_exec.args(startup_command);

    Ok(child.spawn()?)
}

pub async fn extract_natives(runtime_path: &RuntimePath, version: &VersionInfo) {
    async fn extract_single_library_natives(
        runtime_path: &RuntimePath,
        library: &Library,
        version_id: &str,
        native_name: &str,
    ) {
        let maven = MavenCoordinates::try_from(library.name.clone(), Some(native_name.to_string()))
            .unwrap();
        let path = runtime_path.get_libraries().get_library_path(maven);
        let dest = runtime_path.get_natives().get_versioned(version_id);
        tokio::fs::create_dir_all(&dest).await.unwrap();

        println!("Extracting natives from {}", path.display());

        carbon_compression::decompress(path, &dest).await.unwrap();
    }

    for library in version
        .libraries
        .iter()
        .filter(|&lib| library_is_allowed(lib.clone()))
    {
        match &library.natives {
            Some(natives) => {
                if let Some(native_name) = natives.get(&get_current_os()) {
                    extract_single_library_natives(runtime_path, library, &version.id, native_name)
                        .await;
                }
            }
            None => continue,
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::minecraft::minecraft::library_into_natives_downloadable, setup_managers_for_test,
    };

    use super::*;
    use carbon_net::Progress;
    use chrono::Utc;
    use tokio::io::AsyncWriteExt;

    async fn get_account() -> FullAccount {
        FullAccount {
            username: "test".to_owned(),
            uuid: "test-uuid".to_owned(),
            type_: FullAccountType::Offline,
            last_used: Utc::now().into(),
        }
    }

    async fn run_test_generate_startup_command(mc_version: &str) {
        let app = setup_managers_for_test().await;

        let version = app
            .minecraft_manager()
            .get_minecraft_manifest()
            .await
            .unwrap()
            .versions
            .into_iter()
            .find(|v| v.id == "1.16.5")
            .unwrap();

        let version = app
            .minecraft_manager()
            .get_minecraft_version(version)
            .await
            .unwrap();

        let full_account = FullAccount {
            username: "test".to_owned(),
            uuid: "test-uuid".to_owned(),
            type_: FullAccountType::Offline,
            last_used: Utc::now().into(),
        };

        // Mock RuntimePath to have a stable path
        let runtime_path = RuntimePath::new(PathBuf::from("stable_path"));

        let instance_id = InstancePath::new(PathBuf::from("something"));

        let command = generate_startup_command(
            full_account,
            2048,
            2048,
            &runtime_path,
            version,
            instance_id,
        )
        .await;

        // generate a json file with the command
        let command = serde_json::to_string(&command).unwrap();

        // write to file
        let mut file =
            tokio::fs::File::create("./src/managers/minecraft/test_fixtures/test_command.json")
                .await
                .unwrap();

        file.write_all(command.as_bytes()).await.unwrap();
    }

    #[tokio::test]
    async fn test_extract_natives() {
        let app = crate::setup_managers_for_test().await;

        let runtime_path = &app.settings_manager().runtime_path;

        let version = app
            .minecraft_manager()
            .get_minecraft_manifest()
            .await
            .unwrap()
            .versions
            .into_iter()
            .find(|v| v.id == "1.16.5")
            .unwrap();

        let version = app
            .minecraft_manager()
            .get_minecraft_version(version)
            .await
            .unwrap();

        let natives = version
            .libraries
            .iter()
            .filter(|&lib| lib.natives.is_some())
            .collect::<Vec<_>>();

        let mut downloadables = vec![];
        let libraries_path = runtime_path.get_libraries().to_path();
        for native in natives {
            downloadables.extend(library_into_natives_downloadable(
                native.clone(),
                &libraries_path,
            ));
        }
        let progress = tokio::sync::watch::channel(Progress::new());

        carbon_net::download_multiple(downloadables, progress.0)
            .await
            .unwrap();

        extract_natives(runtime_path, &version).await;
    }
}
