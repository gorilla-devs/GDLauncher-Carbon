use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    app_version::APP_VERSION,
    db::PrismaClient,
    domain::{
        java::{JavaArch, JavaComponent},
        minecraft::minecraft::{
            chain_lwjgl_libs_with_base_libs, get_default_jvm_args, is_rule_allowed,
            library_is_allowed, OsExt, ARCH_WIDTH,
        },
    },
};
use anyhow::Context;
use daedalus::minecraft::{
    Argument, ArgumentType, ArgumentValue, Library, LibraryGroup, Os, Version, VersionInfo,
    VersionManifest,
};
use prisma_client_rust::QueryError;
use regex::{Captures, Regex};
use reqwest::Url;
use strum_macros::EnumIter;
use thiserror::Error;
use tokio::process::Child;
use tracing::{info, warn};

use crate::{
    domain::runtime_path::{InstancePath, RuntimePath},
    managers::account::{FullAccount, FullAccountType},
};

use super::META_VERSION;

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
    let server_url = meta_base_url.join(&format!("minecraft/{}/manifest.json", META_VERSION))?;
    let new_manifest = reqwest_client
        .get(server_url)
        .send()
        .await?
        .json::<VersionManifest>()
        .await?;

    Ok(new_manifest)
}

pub async fn get_version(
    db_client: Arc<PrismaClient>,
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    mc_version: &str,
    meta_base_url: &Url,
) -> anyhow::Result<VersionInfo> {
    let db_cache = db_client
        .version_info_cache()
        .find_unique(crate::db::version_info_cache::id::equals(
            mc_version.to_string(),
        ))
        .exec()
        .await
        .map_err(|err| anyhow::anyhow!("Failed to query db: {}", err))?;

    if let Some(db_cache) = db_cache {
        let version_info = serde_json::from_slice(&db_cache.version_info)
            .map_err(|err| anyhow::anyhow!("Failed to deserialize version info: {}", err))?;

        return Ok(version_info);
    }

    let url = meta_base_url
        .join(&format!(
            "minecraft/{}/versions/{}.json",
            META_VERSION, mc_version
        ))
        .unwrap();

    let version_meta = reqwest_client.get(url).send().await?.bytes().await?;

    db_client
        .version_info_cache()
        .create(mc_version.to_string(), version_meta.to_vec(), vec![])
        .exec()
        .await?;

    Ok(serde_json::from_slice(&version_meta)?)
}

pub async fn get_lwjgl_meta(
    db_client: Arc<PrismaClient>,
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    version_info: &VersionInfo,
    meta_base_url: &Url,
) -> anyhow::Result<LibraryGroup> {
    let version_info_lwjgl_requirement = version_info
        .requires
        .as_ref()
        .ok_or(anyhow::anyhow!("Version info requires not provided."))?;
    let version_info_lwjgl_requirement = version_info_lwjgl_requirement
        .first()
        .ok_or(anyhow::anyhow!("Version info requires has no elements."))?;

    let lwjgl_suggest = version_info_lwjgl_requirement
        .rule
        .as_ref()
        .map(|rule| match rule {
            daedalus::minecraft::DependencyRule::Equals(version) => version,
            daedalus::minecraft::DependencyRule::Suggests(version) => version,
        })
        .ok_or(anyhow::anyhow!("Can't find lwjgl version."))?;

    let db_cache = db_client
        .lwjgl_meta_cache()
        .find_unique(crate::db::lwjgl_meta_cache::id::equals(format!(
            "{}-{}",
            version_info_lwjgl_requirement.uid, lwjgl_suggest
        )))
        .exec()
        .await
        .map_err(|err| anyhow::anyhow!("Failed to query db: {}", err))?;

    if let Some(db_cache) = db_cache {
        let lwjgl = serde_json::from_slice(&db_cache.lwjgl)
            .map_err(|err| anyhow::anyhow!("Failed to deserialize lwjgl meta: {}", err))?;

        return Ok(lwjgl);
    }

    let lwjgl_json_url = meta_base_url.join(&format!(
        "minecraft/{}/libraries/{}/{}.json",
        META_VERSION, version_info_lwjgl_requirement.uid, lwjgl_suggest
    ))?;

    tracing::trace!("LWJGL JSON URL: {}", lwjgl_json_url);

    let lwjgl = reqwest_client
        .get(lwjgl_json_url)
        .send()
        .await?
        .json()
        .await?;

    db_client
        .lwjgl_meta_cache()
        .create(
            format!("{}-{}", version_info_lwjgl_requirement.uid, lwjgl_suggest),
            serde_json::to_vec(&lwjgl).unwrap(),
            vec![],
        )
        .exec()
        .await?;

    Ok(lwjgl)
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
    ClasspathSeparator,
    LibraryDirectory,
}

impl TryFrom<&str> for ArgPlaceholder {
    type Error = anyhow::Error;

    fn try_from(arg: &str) -> Result<Self, Self::Error> {
        let res = match arg {
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
            "classpath_separator" => ArgPlaceholder::ClasspathSeparator,
            "library_directory" => ArgPlaceholder::LibraryDirectory,
            _ => anyhow::bail!("Unknown argument placeholder: {arg}"),
        };

        Ok(res)
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
            ArgPlaceholder::ClasspathSeparator => "classpath_separator",
            ArgPlaceholder::LibraryDirectory => "library_directory",
        }
    }
}

struct ReplacerArgs {
    player_name: String,
    player_token: String,
    version_name: String,
    game_directory: InstancePath,
    game_assets: PathBuf,
    library_directory: PathBuf,
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
        ArgPlaceholder::LauncherName => "GDLauncher".to_string(),
        ArgPlaceholder::LauncherVersion => APP_VERSION.to_string(),
        ArgPlaceholder::ClasspathSeparator => CLASSPATH_SEPARATOR.to_string(),
        ArgPlaceholder::LibraryDirectory => replacer_args
            .library_directory
            .to_string_lossy()
            .to_string(),
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

#[allow(clippy::too_many_arguments)]
pub async fn generate_startup_command(
    java_component: JavaComponent,
    full_account: FullAccount,
    xmx_memory: u16,
    xms_memory: u16,
    game_resolution: Option<(u16, u16)>,
    extra_java_args: &str,
    runtime_path: &RuntimePath,
    version: VersionInfo,
    lwjgl_group: &LibraryGroup,
    instance_path: InstancePath,
    assets_dir: super::assets::AssetsDir,
) -> anyhow::Result<Vec<String>> {
    let mut libraries = chain_lwjgl_libs_with_base_libs(
        &version.libraries,
        &lwjgl_group.libraries,
        &java_component.arch,
        &runtime_path.get_libraries(),
        true,
    );

    let tmp_set: HashSet<_> = libraries.drain(..).collect();
    libraries.extend(tmp_set.into_iter());

    let libraries = libraries
        .into_iter()
        .reduce(|a, b| format!("{a}{CLASSPATH_SEPARATOR}{b}"))
        .unwrap();

    let regex =
        Regex::new(r"--(?P<arg>\S+)\s+\$\{(?P<value>[^}]+)\}|(\$\{(?P<standalone>[^}]+)\})")
            .unwrap();

    let extra_args_regex = Regex::new(r#"("(?P<quoted>(\\"|[^"])*)"|(?P<raw>([^ ]+)))"#).unwrap();

    let player_token = match full_account.type_ {
        FullAccountType::Offline => "offline".to_owned(),
        FullAccountType::Microsoft { access_token, .. } => access_token,
    };

    let client_jar_path = runtime_path
        .get_libraries()
        .get_mc_client(version.inherits_from.as_ref().unwrap_or(&version.id));

    let instance_mapped_assets = matches!(&assets_dir, super::assets::AssetsDir::InstanceMapped(_));

    let replacer_args = ReplacerArgs {
        player_name: full_account.username,
        player_token: player_token.clone(),
        version_name: version
            .inherits_from
            .as_ref()
            .unwrap_or(&version.id)
            .clone(),
        game_directory: instance_path.clone(),
        game_assets: assets_dir.to_path_buf(),
        library_directory: runtime_path.get_libraries().to_path(),
        natives_path: runtime_path.get_natives().get_versioned(&version.id),
        assets_root: runtime_path.get_assets().to_path(),
        assets_index_name: version.assets.clone(),
        // Patch libraries adding client jar at the end
        libraries: format!(
            "{}{}{}",
            libraries,
            CLASSPATH_SEPARATOR,
            client_jar_path.display()
        ),
        auth_uuid: full_account.uuid,
        auth_access_token: player_token.clone(),
        auth_session: player_token,
        user_type: "msa".to_owned(),
        version_type: version.type_.as_str().to_string(),
        user_properties: "{}".to_owned(),
    };

    let substitute_argument = |argument: &str| {
        let mut argument = argument.to_string();
        if argument.starts_with("-DignoreList=") {
            argument.push_str(&format!(
                ",{}.jar",
                version.inherits_from.as_ref().unwrap_or(&version.id)
            ));
        }

        regex
            .replace_all(&argument, |caps: &Captures| {
                if let Some(value) = caps.name("value") {
                    let value = match value.as_str().try_into() {
                        Ok(value) => replace_placeholder(&replacer_args, value),
                        Err(err) => {
                            warn!("Failed to parse argument: {}", err);
                            return String::new();
                        }
                    };
                    return format!("--{} {}", caps.name("arg").unwrap().as_str(), value);
                } else if let Some(standalone) = caps.name("standalone") {
                    return match standalone.as_str().try_into() {
                        Ok(standalone) => replace_placeholder(&replacer_args, standalone),
                        Err(err) => {
                            warn!("Failed to parse argument: {}", err);
                            return String::new();
                        }
                    };
                }
                if let Some(arg) = caps.name("arg") {
                    return arg.as_str().to_string();
                } else {
                    unreachable!("No capturing group matched")
                }
            })
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    };

    let substitute_arguments = |command: &mut Vec<String>, arguments: &Vec<Argument>| {
        for arg in arguments {
            match arg {
                Argument::Normal(arg) => {
                    // fix for pre legacy 1.5.x and older minecraft
                    // drop --assetsDir arg, they are found automatically inside the instance
                    // resources folder
                    if instance_mapped_assets
                        && (arg.starts_with("--assetsDir") || arg.starts_with("${game_assets}"))
                    {
                        continue;
                    }

                    command.push(substitute_argument(arg))
                }
                Argument::Ruled { rules, value } => {
                    let is_allowed = rules
                        .iter()
                        .all(|rule| is_rule_allowed(rule, &java_component.arch));

                    match (is_allowed, value) {
                        (false, _) => {}
                        (true, ArgumentValue::Single(arg)) => {
                            command.push(substitute_argument(arg))
                        }
                        (true, ArgumentValue::Many(arr)) => {
                            command.extend(arr.iter().map(|arg| substitute_argument(arg)))
                        }
                    }
                }
            }
        }
    };

    let mut command = Vec::with_capacity(100);

    command.push(format!("-Xmx{xmx_memory}m"));
    command.push(format!("-Xms{xms_memory}m"));

    if let Some(logging_xml) = version.logging {
        if let Some(client) = logging_xml.get(&daedalus::minecraft::LoggingConfigName::Client) {
            let logging_path = runtime_path
                .get_logging_configs()
                .get_client_path(&client.file.id);

            let argument_replaced = client
                .argument
                .replace("${path}", &logging_path.to_string_lossy());

            command.push(argument_replaced);
        }
    }

    let mut arguments = version
        .arguments
        .clone()
        .map(|mut args| {
            let jvm = args.get(&ArgumentType::Jvm);
            if jvm.is_none() {
                args.insert(ArgumentType::Jvm, get_default_jvm_args());
            }

            args
        })
        .unwrap_or_else(|| {
            let mut arguments = HashMap::new();
            arguments.insert(
                ArgumentType::Game,
                version
                    .minecraft_arguments
                    .unwrap_or_default()
                    .split(' ')
                    .map(|s| Argument::Normal(s.to_string()))
                    .collect(),
            );

            arguments.insert(ArgumentType::Jvm, get_default_jvm_args());

            arguments
        });

    // remove --clientId, ${clientid}, --xuid, ${auth_xuid}
    arguments
        .get_mut(&ArgumentType::Game)
        .unwrap()
        .retain(|arg| {
            if let Argument::Normal(arg) = arg {
                !arg.starts_with("--clientId")
                    && !arg.starts_with("--xuid")
                    && !arg.starts_with("${auth_xuid}")
                    && !arg.starts_with("${clientid}")
            } else {
                true
            }
        });

    if let Some(jvm_arguments) = arguments.get(&ArgumentType::Jvm) {
        substitute_arguments(&mut command, jvm_arguments);
    }

    for cap in extra_args_regex.captures_iter(extra_java_args) {
        let ((Some(arg), _) | (_, Some(arg))) = (cap.name("quoted"), cap.name("raw")) else {
            continue;
        };
        command.push(arg.as_str().replace("\\\"", "\"").replace("\\\\", "\\"));
    }

    if Os::native() == Os::Osx {
        let lwjgl_3 = version
            .requires
            .map(|requires| requires.iter().any(|require| require.uid == "org.lwjgl3"))
            .unwrap_or(false);

        let can_find_start_on_first_thread = command
            .iter()
            .any(|arg| arg.contains("XstartOnFirstThread"));

        if !can_find_start_on_first_thread && lwjgl_3 {
            command.push("-XstartOnFirstThread".to_string());
        }
    }

    command.push("-Dorg.lwjgl.util.Debug=true".to_string());

    command.push(version.main_class);

    substitute_arguments(&mut command, arguments.get(&ArgumentType::Game).unwrap());

    if let Some(game_resolution) = game_resolution {
        command.push(format!("--width={}", game_resolution.0));
        command.push(format!("--height={}", game_resolution.1));
    }

    Ok(command)
}

#[allow(clippy::too_many_arguments)]
pub async fn launch_minecraft(
    java_component: JavaComponent,
    full_account: FullAccount,
    xmx_memory: u16,
    xms_memory: u16,
    game_resolution: Option<(u16, u16)>,
    extra_java_args: &str,
    runtime_path: &RuntimePath,
    version: VersionInfo,
    lwjgl_group: &LibraryGroup,
    instance_path: InstancePath,
    assets_dir: super::assets::AssetsDir,
) -> anyhow::Result<Child> {
    let startup_command = generate_startup_command(
        java_component.clone(),
        full_account,
        xmx_memory,
        xms_memory,
        game_resolution,
        extra_java_args,
        runtime_path,
        version,
        lwjgl_group,
        instance_path.clone(),
        assets_dir,
    )
    .await?;

    info!(
        "Starting Minecraft with command: {} {}",
        java_component.path,
        startup_command.join(" ")
    );

    let mut command_exec = tokio::process::Command::new(java_component.path);
    command_exec.current_dir(instance_path.get_data_path());

    command_exec.stdout(std::process::Stdio::piped());
    command_exec.stderr(std::process::Stdio::piped());

    let child = command_exec.args(startup_command);

    Ok(child.spawn()?)
}

pub async fn extract_natives(
    runtime_path: &RuntimePath,
    version: &VersionInfo,
    lwjgl_group: &LibraryGroup,
    java_arch: &JavaArch,
) -> anyhow::Result<()> {
    async fn extract_single_library_natives(
        runtime_path: &RuntimePath,
        library: &Library,
        dest: &Path,
        native_name: &str,
    ) -> anyhow::Result<()> {
        let native_name = native_name.replace("${arch}", ARCH_WIDTH);
        let path = runtime_path.get_libraries().get_library_path({
            library
                .downloads
                .as_ref()
                .unwrap()
                .classifiers
                .as_ref()
                .unwrap()
                .get(&native_name)
                .unwrap()
                .path
                .clone()
        });

        info!("Extracting natives from {}", path.display());

        carbon_compression::decompress(&path, dest)
            .await
            .with_context(|| {
                format!(
                    "Failed to decompress natives from `{}` to `{}`",
                    path.to_string_lossy(),
                    dest.to_string_lossy()
                )
            })?;

        Ok(())
    }

    info!("Start natives extraction for id {}", version.id);

    let dest = runtime_path.get_natives().get_versioned(&version.id);
    tokio::fs::create_dir_all(&dest).await?;

    for library in version
        .libraries
        .iter()
        .chain(lwjgl_group.libraries.iter())
        .filter(|&lib| library_is_allowed(lib, java_arch))
    {
        match &library.natives {
            Some(natives) => {
                if let Some(native_name) = natives.get(&Os::native_arch(java_arch)) {
                    extract_single_library_natives(runtime_path, library, &dest, native_name)
                        .await
                        .with_context(|| {
                            format!(
                                "Failed to extract native `{}` for library `{}`",
                                &native_name, &library.name
                            )
                        })?;
                }
            }
            None => continue,
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::minecraft::minecraft::library_into_natives_downloadable,
        managers::java::java_checker::{JavaChecker, RealJavaChecker},
        setup_managers_for_test,
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

    async fn run_test_generate_startup_command(_mc_version: &str) {
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
            .get_minecraft_version(&version.id)
            .await
            .unwrap();

        let lwjgl_group = get_lwjgl_meta(
            app.prisma_client.clone(),
            &reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build(),
            &version,
            &app.minecraft_manager().meta_base_url,
        )
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

        let java_component = RealJavaChecker
            .get_bin_info(
                &PathBuf::from("java"),
                crate::domain::java::JavaComponentType::Local,
            )
            .await
            .unwrap();

        let assets_dir = crate::managers::minecraft::assets::get_assets_dir(
            app.prisma_client.clone(),
            reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build(),
            &version.asset_index,
            runtime_path.get_assets(),
            instance_id.get_resources_path(),
        )
        .await
        .unwrap();

        let command = generate_startup_command(
            java_component,
            full_account,
            2048,
            2048,
            None,
            "",
            &runtime_path,
            version,
            &lwjgl_group,
            instance_id,
            assets_dir,
        )
        .await
        .unwrap();

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
            .get_minecraft_version(&version.id.as_str())
            .await
            .unwrap();

        let lwjgl_group = get_lwjgl_meta(
            app.prisma_client.clone(),
            &reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build(),
            &version,
            &app.minecraft_manager().meta_base_url,
        )
        .await
        .unwrap();

        let natives = version
            .libraries
            .iter()
            .chain(lwjgl_group.libraries.iter())
            .filter(|&lib| lib.natives.is_some())
            .collect::<Vec<_>>();

        let mut downloadables = vec![];
        let libraries_path = runtime_path.get_libraries().to_path();
        for native in natives {
            downloadables.extend(library_into_natives_downloadable(
                native.clone(),
                &libraries_path,
                &JavaArch::X86_64,
            ));
        }
        let progress = tokio::sync::watch::channel(Progress::new());

        carbon_net::download_multiple(&downloadables[..], Some(progress.0), 10, true, false)
            .await
            .unwrap();

        extract_natives(runtime_path, &version, &lwjgl_group, &JavaArch::X86_64)
            .await
            .unwrap();
    }
}
