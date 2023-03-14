use std::{path::PathBuf, sync::Arc};

use carbon_domain::{
    maven::MavenCoordinates,
    minecraft::{
        manifest::ManifestVersion,
        version::{Argument, Library, OsName, Value, Version},
    },
};
use prisma_client_rust::QueryError;
use regex::{Captures, Regex};
use strum_macros::EnumIter;
use thiserror::Error;

use crate::{
    db::PrismaClient,
    managers::{
        account::{FullAccount, FullAccountType},
        configuration::runtime_path::{InstancePath, RuntimePath},
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
            .get_root()
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

fn replace_placeholders(
    full_account: FullAccount,
    runtime_path: &RuntimePath,
    command: String,
    version: &Version,
    libraries: String,
) -> String {
    let matches =
        Regex::new(r"--(?P<arg>\S+)\s+\$\{(?P<value>[^}]+)\}|(\$\{(?P<standalone>[^}]+)\})")
            .unwrap();

    let player_name = full_account.username;
    let player_uuid = full_account.uuid;
    let player_token = match full_account.type_ {
        FullAccountType::Offline => "offline".to_owned(),
        FullAccountType::Microsoft { access_token, .. } => access_token,
    };

    let version_name = version.id.clone();
    let game_directory = runtime_path
        .get_instances()
        .get_instance_path("something".to_owned());
    let assets_root = runtime_path.get_assets().to_path();
    let game_assets = runtime_path.get_assets().to_path();
    let assets_index_name = version.assets.clone().unwrap();

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
        libraries,
        auth_uuid: player_uuid,
        auth_access_token: player_token.clone(),
        auth_session: player_token,
        user_type: "mojang".to_owned(),
        version_type: version.type_.as_ref().unwrap().to_owned(),
        user_properties: "{}".to_owned(),
    };

    let new_command = matches.replace_all(&command, |caps: &Captures| {
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
    });

    new_command.to_string()
}

#[cfg(target_os = "windows")]
const CLASSPATH_SEPARATOR: &str = ";";
#[cfg(not(target_os = "windows"))]
const CLASSPATH_SEPARATOR: &str = ":";

pub async fn generate_startup_command(
    full_account: FullAccount,
    xmx_memory: u16,
    xms_memory: u16,
    runtime_path: &RuntimePath,
    version: Version,
) -> String {
    let libraries = version
        .libraries
        .get_libraries()
        .iter()
        .map(|library| {
            let path = runtime_path
                .get_libraries()
                .get_library_path(MavenCoordinates::try_from(library.name.clone(), None).unwrap());

            path.display().to_string()
        })
        .reduce(|a, b| format!("{a}{CLASSPATH_SEPARATOR}{b}"))
        .unwrap();

    let mut command = Vec::with_capacity(libraries.len() * 2);
    command.push("java".to_owned());

    command.push(format!("-Xmx{xmx_memory}m"));
    command.push(format!("-Xms{xms_memory}m"));

    let arguments = version.arguments.clone().unwrap_or_default();

    let game_arguments = arguments.game;
    let jvm_arguments = arguments.jvm;

    for arg in jvm_arguments {
        match arg {
            Argument::String(string) => command.push(string),
            Argument::Complex(rule) => {
                let is_allowed = rule.rules.iter().all(|rule| rule.is_allowed());

                if is_allowed {
                    match rule.value {
                        Value::String(string) => command.push(string),
                        Value::StringArray(arr) => command.extend(arr),
                    }
                }
            }
        }
    }

    // command.push("-Dlog4j.configurationFile=C:\Users\david\AppData\Roaming\gdlauncher_next\datastore\assets\objects\bd\client-1.12.xml".to_owned());

    command.push(version.main_class.clone());

    for arg in game_arguments {
        match arg {
            Argument::String(string) => command.push(string),
            Argument::Complex(rule) => {
                let is_allowed = rule.rules.iter().all(|rule| rule.is_allowed());

                if is_allowed {
                    match rule.value {
                        Value::String(string) => command.push(string),
                        Value::StringArray(arr) => command.extend(arr),
                    }
                }
            }
        }
    }

    // command.push("--username killpowa --version 1.19.3 --gameDir ..\..\instances\Minecraft vanilla --assetsDir ..\..\datastore\assets --assetIndex 2 --uuid 3b40f99969e64dbcabd01f87cddcb1fd --accessToken __HIDDEN_TOKEN__ --clientId ${clientid} --xuid ${auth_xuid} --userType mojang --versionType release --width=854 --height=480".to_owned());
    let command_string = command.join(" ");

    replace_placeholders(
        full_account,
        runtime_path,
        command_string,
        &version,
        libraries,
    )
}

async fn extract_natives(runtime_path: &RuntimePath, version: &Version) {
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

    for library in version.libraries.get_libraries() {
        match &library.natives {
            Some(natives) => {
                if cfg!(target_os = "windows") {
                    match natives.windows.as_ref() {
                        Some(native_name) => {
                            extract_single_library_natives(
                                runtime_path,
                                library,
                                &version.id,
                                native_name,
                            )
                            .await
                        }
                        None => continue,
                    }
                } else if cfg!(target_os = "linux") {
                    match natives.linux.as_ref() {
                        Some(native_name) => {
                            extract_single_library_natives(
                                runtime_path,
                                library,
                                &version.id,
                                native_name,
                            )
                            .await
                        }
                        None => continue,
                    }
                } else if cfg!(target_os = "macos") {
                    match natives.osx.as_ref() {
                        Some(native_name) => {
                            extract_single_library_natives(
                                runtime_path,
                                library,
                                &version.id,
                                native_name,
                            )
                            .await
                        }
                        None => continue,
                    }
                } else {
                    panic!("Unsupported platform");
                }
            }
            None => continue,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbon_domain::minecraft::manifest::MinecraftManifest;
    use carbon_net::{IntoDownloadable, Progress};

    #[tokio::test]
    async fn test_generate_startup_command() {
        let manifest = MinecraftManifest::fetch().await.unwrap();

        let version = manifest
            .versions
            .into_iter()
            .find(|v| v.id == "1.16.5")
            .unwrap()
            .fetch()
            .await
            .unwrap();

        let full_account = FullAccount {
            username: "test".to_owned(),
            uuid: "test-uuid".to_owned(),
            type_: FullAccountType::Offline,
        };

        // Mock RuntimePath to have a stable path
        let runtime_path = RuntimePath::new(PathBuf::from("stable_path"));

        let command =
            generate_startup_command(full_account, 2048, 2048, &runtime_path, version).await;

        let fixture: &str = if cfg!(target_os = "macos") {
            "java -Xmx2048m -Xms2048m -XstartOnFirstThread -Djava.library.path=stable_path/natives/1.16.5 -Dminecraft.launcher.brand=minecraft-launcher -Dminecraft.launcher.version=2 -cp stable_path/libraries/com.mojang/patchy/1.3.9:stable_path/libraries/oshi-project/oshi-core/1.1:stable_path/libraries/net.java.dev.jna/jna/4.4.0:stable_path/libraries/net.java.dev.jna/platform/3.4.0:stable_path/libraries/com.ibm.icu/icu4j/66.1:stable_path/libraries/com.mojang/javabridge/1.0.22:stable_path/libraries/net.sf.jopt-simple/jopt-simple/5.0.3:stable_path/libraries/io.netty/netty-all/4.1.25.Final:stable_path/libraries/com.google.guava/guava/21.0:stable_path/libraries/org.apache.commons/commons-lang3/3.5:stable_path/libraries/commons-io/commons-io/2.5:stable_path/libraries/commons-codec/commons-codec/1.10:stable_path/libraries/net.java.jinput/jinput/2.0.5:stable_path/libraries/net.java.jutils/jutils/1.0.0:stable_path/libraries/com.mojang/brigadier/1.0.17:stable_path/libraries/com.mojang/datafixerupper/4.0.26:stable_path/libraries/com.google.code.gson/gson/2.8.0:stable_path/libraries/com.mojang/authlib/2.1.28:stable_path/libraries/org.apache.commons/commons-compress/1.8.1:stable_path/libraries/org.apache.httpcomponents/httpclient/4.3.3:stable_path/libraries/commons-logging/commons-logging/1.1.3:stable_path/libraries/org.apache.httpcomponents/httpcore/4.3.2:stable_path/libraries/it.unimi.dsi/fastutil/8.2.1:stable_path/libraries/org.apache.logging.log4j/log4j-api/2.8.1:stable_path/libraries/org.apache.logging.log4j/log4j-core/2.8.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.2:stable_path/libraries/org.lwjgl/lwjgl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.2:stable_path/libraries/com.mojang/text2speech/1.11.3:stable_path/libraries/com.mojang/text2speech/1.11.3:stable_path/libraries/ca.weblite/java-objc-bridge/1.0.0:stable_path/libraries/ca.weblite/java-objc-bridge/1.0.0 net.minecraft.client.main.Main --username test --version 1.16.5 --gameDir stable_path/instances/something --assetsDir stable_path/assets --assetIndex 1.16 --uuid test-uuid --accessToken offline --userType mojang --versionType release"
        } else if cfg!(target_os = "linux") {
            "java -Xmx2048m -Xms2048m -Djava.library.path=stable_path/natives/1.16.5 -Dminecraft.launcher.brand=minecraft-launcher -Dminecraft.launcher.version=2 -cp stable_path/libraries/com.mojang/patchy/1.3.9:stable_path/libraries/oshi-project/oshi-core/1.1:stable_path/libraries/net.java.dev.jna/jna/4.4.0:stable_path/libraries/net.java.dev.jna/platform/3.4.0:stable_path/libraries/com.ibm.icu/icu4j/66.1:stable_path/libraries/com.mojang/javabridge/1.0.22:stable_path/libraries/net.sf.jopt-simple/jopt-simple/5.0.3:stable_path/libraries/io.netty/netty-all/4.1.25.Final:stable_path/libraries/com.google.guava/guava/21.0:stable_path/libraries/org.apache.commons/commons-lang3/3.5:stable_path/libraries/commons-io/commons-io/2.5:stable_path/libraries/commons-codec/commons-codec/1.10:stable_path/libraries/net.java.jinput/jinput/2.0.5:stable_path/libraries/net.java.jutils/jutils/1.0.0:stable_path/libraries/com.mojang/brigadier/1.0.17:stable_path/libraries/com.mojang/datafixerupper/4.0.26:stable_path/libraries/com.google.code.gson/gson/2.8.0:stable_path/libraries/com.mojang/authlib/2.1.28:stable_path/libraries/org.apache.commons/commons-compress/1.8.1:stable_path/libraries/org.apache.httpcomponents/httpclient/4.3.3:stable_path/libraries/commons-logging/commons-logging/1.1.3:stable_path/libraries/org.apache.httpcomponents/httpcore/4.3.2:stable_path/libraries/it.unimi.dsi/fastutil/8.2.1:stable_path/libraries/org.apache.logging.log4j/log4j-api/2.8.1:stable_path/libraries/org.apache.logging.log4j/log4j-core/2.8.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.2:stable_path/libraries/org.lwjgl/lwjgl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.2:stable_path/libraries/com.mojang/text2speech/1.11.3:stable_path/libraries/com.mojang/text2speech/1.11.3:stable_path/libraries/ca.weblite/java-objc-bridge/1.0.0:stable_path/libraries/ca.weblite/java-objc-bridge/1.0.0 net.minecraft.client.main.Main --username test --version 1.16.5 --gameDir stable_path/instances/something --assetsDir stable_path/assets --assetIndex 1.16 --uuid test-uuid --accessToken offline --userType mojang --versionType release"
        } else {
            r#"java -Xmx2048m -Xms2048m -XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump -Dos.name=Windows 10 -Dos.version=10.0 -Djava.library.path=stable_path\\natives\\1.16.5 -Dminecraft.launcher.brand=minecraft-launcher -Dminecraft.launcher.version=2 -cp stable_path\\libraries\\com.mojang\\patchy\\1.3.9;stable_path\\libraries\\oshi-project\\oshi-core\\1.1;stable_path\\libraries\\net.java.dev.jna\\jna\\4.4.0;stable_path\\libraries\\net.java.dev.jna\\platform\\3.4.0;stable_path\\libraries\\com.ibm.icu\\icu4j\\66.1;stable_path\\libraries\\com.mojang\\javabridge\\1.0.22;stable_path\\libraries\\net.sf.jopt-simple\\jopt-simple\\5.0.3;stable_path\\libraries\\io.netty\\netty-all\\4.1.25.Final;stable_path\\libraries\\com.google.guava\\guava\\21.0;stable_path\\libraries\\org.apache.commons\\commons-lang3\\3.5;stable_path\\libraries\\commons-io\\commons-io\\2.5;stable_path\\libraries\\commons-codec\\commons-codec\\1.10;stable_path\\libraries\\net.java.jinput\\jinput\\2.0.5;stable_path\\libraries\\net.java.jutils\\jutils\\1.0.0;stable_path\\libraries\\com.mojang\\brigadier\\1.0.17;stable_path\\libraries\\com.mojang\\datafixerupper\\4.0.26;stable_path\\libraries\\com.google.code.gson\\gson\\2.8.0;stable_path\\libraries\\com.mojang\\authlib\\2.1.28;stable_path\\libraries\\org.apache.commons\\commons-compress\\1.8.1;stable_path\\libraries\\org.apache.httpcomponents\\httpclient\\4.3.3;stable_path\\libraries\\commons-logging\\commons-logging\\1.1.3;stable_path\\libraries\\org.apache.httpcomponents\\httpcore\\4.3.2;stable_path\\libraries\\it.unimi.dsi\\fastutil\\8.2.1;stable_path\\libraries\\org.apache.logging.log4j\\log4j-api\\2.8.1;stable_path\\libraries\\org.apache.logging.log4j\\log4j-core\\2.8.1;stable_path\\libraries\\org.lwjgl\\lwjgl\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-jemalloc\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-jemalloc\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-openal\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-openal\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-opengl\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-opengl\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-glfw\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-glfw\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-stb\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-stb\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-tinyfd\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-tinyfd\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-jemalloc\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-jemalloc\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-openal\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-openal\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-opengl\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-opengl\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-glfw\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-glfw\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-stb\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-tinyfd\\3.2.2;stable_path\\libraries\\org.lwjgl\\lwjgl-tinyfd\\3.2.1;stable_path\\libraries\\org.lwjgl\\lwjgl-stb\\3.2.2;stable_path\\libraries\\com.mojang\\text2speech\\1.11.3;stable_path\\libraries\\com.mojang\\text2speech\\1.11.3;stable_path\\libraries\\ca.weblite\\java-objc-bridge\\1.0.0;stable_path\\libraries\\ca.weblite\\java-objc-bridge\\1.0.0 net.minecraft.client.main.Main --username test --version 1.16.5 --gameDir stable_path\\instances\\something --assetsDir stable_path\\assets --assetIndex 1.16 --uuid test-uuid --accessToken offline --userType mojang --versionType release"#
        };

        assert_eq!(command, fixture);
    }

    #[tokio::test]
    async fn test_extract_natives() {
        let app = crate::setup_managers_for_test().await;

        let runtime_path = &app.configuration_manager().runtime_path;

        let manifest = MinecraftManifest::fetch().await.unwrap();
        let version = manifest
            .versions
            .into_iter()
            .find(|v| v.id == "1.16.5")
            .unwrap()
            .fetch()
            .await
            .unwrap();

        let natives = version
            .libraries
            .get_libraries()
            .iter()
            .filter(|lib| lib.is_native_artifact())
            .collect::<Vec<_>>();

        let mut downloadables = vec![];
        let libraries_path = runtime_path.get_libraries().to_path();
        for native in natives {
            downloadables.push(native.clone().into_downloadable(&libraries_path));
        }

        let progress = tokio::sync::watch::channel(Progress::new());

        println!("{:#?}", downloadables);
        carbon_net::download_multiple(downloadables, progress.0)
            .await
            .unwrap();

        extract_natives(runtime_path, &version).await;
    }
}
