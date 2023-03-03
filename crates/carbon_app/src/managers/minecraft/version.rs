use std::{path::PathBuf, sync::Arc};

use carbon_domain::{
    maven::MavenCoordinates,
    minecraft::{
        manifest::ManifestVersion,
        version::{GameElement, Version},
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
        }
    }
}

struct ReplacerArgs {
    player_name: String,
    player_token: String,
    version_name: String,
    game_directory: InstancePath,
    game_assets: PathBuf,
    assets_root: PathBuf,
    assets_index_name: String,
    auth_uuid: String,
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
    }
}

fn replace_placeholders(
    full_account: FullAccount,
    runtime_path: &RuntimePath,
    command: String,
    version: &Version,
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
        assets_root,
        assets_index_name,
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

pub async fn generate_startup_command(
    full_account: FullAccount,
    runtime_path: &RuntimePath,
    version: Version,
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

    let libraries = libraries
        .get_libraries()
        .iter()
        .map(|library| {
            let path = runtime_path
                .get_libraries()
                .get_library_path(MavenCoordinates::try_from(library.name.clone()).unwrap());

            format!("{}{}", path.display(), classpath_separator)
        })
        .reduce(|a, b| format!("{a}{b}"))
        .unwrap();

    command.push(libraries);

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
    let mut mc_command = Vec::new();

    if let Some(arguments) = &version.arguments {
        if let Some(game) = &arguments.game {
            for arg in game {
                if let GameElement::String(s) = arg {
                    mc_command.push(s.clone());
                }
            }
        }
    } else if let Some(arguments) = &version.minecraft_arguments {
        mc_command.push(arguments.clone());
    }

    command.extend(mc_command);
    // command.push("--username killpowa --version 1.19.3 --gameDir ..\..\instances\Minecraft vanilla --assetsDir ..\..\datastore\assets --assetIndex 2 --uuid 3b40f99969e64dbcabd01f87cddcb1fd --accessToken __HIDDEN_TOKEN__ --clientId ${clientid} --xuid ${auth_xuid} --userType mojang --versionType release --width=854 --height=480".to_owned());
    let command_string = command.join(" ");

    replace_placeholders(full_account, runtime_path, command_string, &version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbon_domain::minecraft::manifest::MinecraftManifest;

    async fn get_account() -> FullAccount {
        FullAccount {
            username: "test".to_owned(),
            uuid: "test-uuid".to_owned(),
            type_: FullAccountType::Offline,
        }
    }

    // Test with cargo test -- --nocapture --exact managers::minecraft::version::tests::test_generate_startup_command
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

        let full_account = get_account().await;

        // Mock RuntimePath to have a stable path
        let runtime_path = RuntimePath::new(PathBuf::from("stable_path"));

        let command = generate_startup_command(full_account, &runtime_path, version).await;

        let fixture = "java -XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump -Dos.name=Windows 10 -Dos.version=10.0 -Djava.library.path=stable_path/natives/1.16.5 -cp stable_path/libraries/com.mojang/patchy/1.3.9:stable_path/libraries/oshi-project/oshi-core/1.1:stable_path/libraries/net.java.dev.jna/jna/4.4.0:stable_path/libraries/net.java.dev.jna/platform/3.4.0:stable_path/libraries/com.ibm.icu/icu4j/66.1:stable_path/libraries/com.mojang/javabridge/1.0.22:stable_path/libraries/net.sf.jopt-simple/jopt-simple/5.0.3:stable_path/libraries/io.netty/netty-all/4.1.25.Final:stable_path/libraries/com.google.guava/guava/21.0:stable_path/libraries/org.apache.commons/commons-lang3/3.5:stable_path/libraries/commons-io/commons-io/2.5:stable_path/libraries/commons-codec/commons-codec/1.10:stable_path/libraries/net.java.jinput/jinput/2.0.5:stable_path/libraries/net.java.jutils/jutils/1.0.0:stable_path/libraries/com.mojang/brigadier/1.0.17:stable_path/libraries/com.mojang/datafixerupper/4.0.26:stable_path/libraries/com.google.code.gson/gson/2.8.0:stable_path/libraries/com.mojang/authlib/2.1.28:stable_path/libraries/org.apache.commons/commons-compress/1.8.1:stable_path/libraries/org.apache.httpcomponents/httpclient/4.3.3:stable_path/libraries/commons-logging/commons-logging/1.1.3:stable_path/libraries/org.apache.httpcomponents/httpcore/4.3.2:stable_path/libraries/it.unimi.dsi/fastutil/8.2.1:stable_path/libraries/org.apache.logging.log4j/log4j-api/2.8.1:stable_path/libraries/org.apache.logging.log4j/log4j-core/2.8.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.2:stable_path/libraries/org.lwjgl/lwjgl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-jemalloc/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-openal/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-opengl/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-glfw/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.2:stable_path/libraries/org.lwjgl/lwjgl-tinyfd/3.2.1:stable_path/libraries/org.lwjgl/lwjgl-stb/3.2.2:stable_path/libraries/com.mojang/text2speech/1.11.3:stable_path/libraries/com.mojang/text2speech/1.11.3:stable_path/libraries/ca.weblite/java-objc-bridge/1.0.0:stable_path/libraries/ca.weblite/java-objc-bridge/1.0.0: -Xmx4096m -Xms4096m -Dminecraft.applet.TargetDirectory=stable_path -Dfml.ignorePatchDiscrepancies=true -Dfml.ignoreInvalidMinecraftCertificates=true net.minecraft.client.main.Main --username test --version 1.16.5 --gameDir stable_path/instances/something --assetsDir stable_path/assets --assetIndex 1.16 --uuid test --accessToken offline --userType mojang --versionType release";

        assert_eq!(command, fixture);
    }

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
