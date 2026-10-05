use crate::managers::account::{FullAccount, FullAccountType};
use crate::{
    app_version::APP_VERSION,
    domain::{
        java::{JavaArch, JavaComponent},
        minecraft::minecraft::{
            ARCH_WIDTH, OsExt, chain_lwjgl_libs_with_base_libs, get_default_jvm_args,
            is_rule_allowed, library_is_allowed,
        },
    },
};
use anyhow::Context;
use carbon_repos::db_error::DbError;
use carbon_repos::db_exec::Db;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::version_meta;
use carbon_rt_path::{InstancePath, RuntimePath};
use chrono::Utc;
use daedalus::minecraft::{
    Argument, ArgumentType, ArgumentValue, Library, LibraryGroup, Os, Version, VersionInfo,
    VersionManifest,
};
use regex::{Captures, Regex};
use reqwest::Url;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use strum_macros::EnumIter;
use thiserror::Error;
use tokio::{process::Child, sync::Mutex, time::Instant};
use tracing::{info, trace, warn};

use super::META_VERSION;

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("Could not fetch version meta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Could not execute db query: {0}")]
    QueryError(#[from] DbError),
}

#[derive(Error, Debug)]
pub enum MinecraftManifestError {
    #[error("Could not fetch minecraft manifest from launchermeta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Manifest database query error: {0}")]
    DBQueryError(#[from] DbError),
}

pub async fn get_manifest(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    meta_base_url: &Url,
) -> anyhow::Result<VersionManifest> {
    let server_url = meta_base_url.join(&format!("minecraft/{}/manifest.json", META_VERSION))?;
    let time = Instant::now();
    let new_manifest = reqwest_client
        .get(server_url)
        .send()
        .await?
        .json::<VersionManifest>()
        .await?;

    let elapsed = time.elapsed();

    tracing::info!("Fetched manifest {}ms", elapsed.as_millis());

    Ok(new_manifest)
}

pub async fn get_version(
    db: &Db,
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    mc_version: &str,
    meta_base_url: &Url,
) -> anyhow::Result<VersionInfo> {
    static LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = LOCK.lock().await;

    let url = meta_base_url
        .join(&format!(
            "minecraft/{}/versions/{}.json",
            META_VERSION, mc_version
        ))
        .unwrap();

    let update_cache = || async {
        let resp = reqwest_client.get(url.clone()).send().await?;

        let status = resp.status();

        if !status.is_success() {
            anyhow::bail!(
                "Failed to fetch minecraft version from `{}`: {}",
                url.clone(),
                status
            );
        }

        let version_meta = resp.bytes().await.with_context(|| {
            format!(
                "Failed to fetch minecraft version from `{}`: {}",
                url.clone(),
                status
            )
        })?;

        // Validate the freshly fetched body before caching it: a 200 response with an
        // unparseable body must not overwrite a previously-good cached version.
        let parsed = serde_json::from_slice::<VersionInfo>(&version_meta)
            .with_context(|| format!("Failed to parse minecraft version from `{}`", url.clone()))?;

        version_meta::upsert_version_info(
            db,
            &mc_version.to_string(),
            &version_meta,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;

        Ok(parsed)
    };

    match update_cache().await {
        Ok(parsed) => Ok(parsed),
        Err(err) => {
            let mc_version_owned = mc_version.to_string();
            let db_cache = version_meta::get_version_info(db, &mc_version_owned)
                .await
                .map_err(|err| anyhow::anyhow!("Failed to query db: {}", err))?;

            if let Some(db_cache) = db_cache {
                let db_cache = serde_json::from_slice(&db_cache.version_info);
                if let Ok(db_cache) = db_cache {
                    trace!("Minecraft version {} found in cache", mc_version);
                    return Ok(db_cache);
                } else {
                    tracing::warn!(
                        "Failed to deserialize minecraft version for {} from cache, re-fetching from {}",
                        mc_version,
                        url.clone()
                    );
                }
            }

            anyhow::bail!(
                "Failed to fetch minecraft version from `{}`: {}",
                url.clone(),
                err
            );
        }
    }
}

pub async fn get_lwjgl_meta(
    db: &Db,
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

    static LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = LOCK.lock().await;

    let update_cache = || async {
        let lwjgl_json_url = meta_base_url.join(&format!(
            "minecraft/{}/libraries/{}/{}.json",
            META_VERSION, version_info_lwjgl_requirement.uid, lwjgl_suggest
        ))?;

        tracing::trace!("LWJGL JSON URL: {}", lwjgl_json_url);

        let resp = reqwest_client.get(lwjgl_json_url.clone()).send().await?;

        let status = resp.status();

        if !status.is_success() {
            anyhow::bail!(
                "Failed to fetch minecraft version from `{}`: {}",
                lwjgl_json_url.clone(),
                status
            );
        }

        let lwjgl = resp.bytes().await.with_context(|| {
            format!(
                "Failed to fetch minecraft version from `{}`: {}",
                lwjgl_json_url.clone(),
                status
            )
        })?;

        // Validate the freshly fetched body before caching it: a 200 response with an
        // unparseable body must not overwrite a previously-good cached LWJGL group.
        let parsed = serde_json::from_slice::<LibraryGroup>(&lwjgl).with_context(|| {
            format!(
                "Failed to parse LWJGL metadata from `{}`",
                lwjgl_json_url.clone()
            )
        })?;

        let db_entry_name = format!("{}-{}", version_info_lwjgl_requirement.uid, lwjgl_suggest);

        version_meta::upsert_lwjgl_meta(
            db,
            &db_entry_name,
            &lwjgl,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;

        Ok(parsed)
    };

    match update_cache().await {
        Ok(parsed) => Ok(parsed),
        Err(err) => {
            let db_entry_name = format!("{}-{}", version_info_lwjgl_requirement.uid, lwjgl_suggest);
            let db_cache = version_meta::get_lwjgl_meta(db, &db_entry_name)
                .await
                .map_err(|err| anyhow::anyhow!("Failed to query db: {}", err))?;

            if let Some(db_cache) = db_cache {
                let lwjgl = serde_json::from_slice(&db_cache.lwjgl);

                if let Ok(lwjgl) = lwjgl {
                    trace!("LWJGL version {} found in cache", lwjgl_suggest);
                    return Ok(lwjgl);
                } else {
                    tracing::warn!(
                        "Failed to deserialize lwjgl version for {} from cache, re-fetching",
                        lwjgl_suggest
                    );
                }
            }

            anyhow::bail!("Failed to fetch lwjgl from `{}`: {}", lwjgl_suggest, err);
        }
    }
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

/// Split a user-provided extra-Java-args string into individual arguments.
///
/// Arguments are separated by whitespace, except whitespace inside a
/// double-quoted span is preserved. Quotes may appear anywhere in an
/// argument (so both `"-javaagent:C:\With Spaces\a.jar"` and the more
/// natural `-javaagent:"C:\With Spaces\a.jar"` work, joining the quoted and
/// unquoted runs into one token). Within an argument, `\"` is a literal quote
/// and `\\` a literal backslash; every other character — including lone
/// backslashes in Windows paths — is taken verbatim. Explicitly quoted empty
/// tokens are retained; blank command strings contain no tokens.
pub(crate) fn parse_extra_java_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut token_present = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if matches!(chars.peek(), Some(&('"' | '\\'))) => {
                token_present = true;
                current.push(chars.next().expect("peeked char is present"));
            }
            '"' => {
                token_present = true;
                in_quotes = !in_quotes;
            }
            c if c.is_whitespace() && !in_quotes => {
                if token_present {
                    args.push(std::mem::take(&mut current));
                    token_present = false;
                }
            }
            c => {
                token_present = true;
                current.push(c);
            }
        }
    }

    if token_present {
        args.push(current);
    }

    args
}

/// The extra JVM arguments as passed to Java. They sit right before the main
/// class, and Java stops reading options at the first argument that is not one,
/// so a blank token (a stray `""`, a lone trailing quote, or a blank element
/// imported from another launcher) would be taken as the class name and the
/// game would not start. No JVM option is blank, so those tokens are dropped.
fn extra_java_launch_args(input: &str) -> Vec<String> {
    let mut args = parse_extra_java_args(input);
    args.retain(|arg| !arg.trim().is_empty());
    args
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

    // chain_lwjgl_libs_with_base_libs already deduplicates by computed library name and
    // preserves a deterministic order; a HashSet pass here would re-randomize that order,
    // which can non-deterministically change which shaded class wins on the classpath.
    let libraries = libraries
        .into_iter()
        .reduce(|a, b| format!("{a}{CLASSPATH_SEPARATOR}{b}"))
        .unwrap();

    let regex =
        Regex::new(r"--(?P<arg>\S+)\s+\$\{(?P<value>[^}]+)\}|(\$\{(?P<standalone>[^}]+)\})")
            .unwrap();

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

    // Extra JVM arguments come from local instance/global settings and imported
    // installed-instance settings, including legacy GDLauncher and CurseForge.
    // They share the persisted representation, so launch cannot infer provenance.
    // Any source-specific sanitization/provenance policy belongs at the import
    // boundary. Preserve the token grammar and intentional agent arguments used
    // by profilers, hot-swap tools and authlib-injector.
    command.extend(extra_java_launch_args(extra_java_args));

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
    wrapper_command: Option<String>,
    ownership: &mut crate::managers::runtime_lifecycle::RuntimeActivity,
) -> anyhow::Result<Child> {
    // Mojang/Forge/Fabric version manifests embed the live MS access token as
    // `--accessToken <token>` (and `--session token:<token>:<uuid>` for legacy
    // versions). Capture the token before moving `full_account` so we can
    // scrub it from the launch-command log line below.
    let secret_to_redact = match &full_account.type_ {
        FullAccountType::Microsoft { access_token, .. } if !access_token.is_empty() => {
            Some(access_token.clone())
        }
        _ => None,
    };

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

    // A wrapper command (e.g. "mangohud gamemoderun") is a full command line, so split it
    // into a program plus arguments and run the java invocation through it. An empty or
    // whitespace-only wrapper is treated as no wrapper.
    let wrapper_tokens = match wrapper_command.as_deref().map(str::trim) {
        Some(raw) if !raw.is_empty() => {
            // shlex uses POSIX backslash escaping, so on Windows a path like `C:\tools\wrap.exe`
            // would be mangled (backslashes dropped) or rejected (a trailing one aborts the
            // split). Escape backslashes first on Windows so they survive the split intact.
            #[cfg(target_os = "windows")]
            let escaped = raw.replace('\\', "\\\\");
            #[cfg(target_os = "windows")]
            let wrapper: &str = &escaped;
            #[cfg(not(target_os = "windows"))]
            let wrapper: &str = raw;

            let tokens = shlex::split(wrapper)
                .with_context(|| format!("Invalid wrapper command: `{wrapper}`"))?;
            (!tokens.is_empty()).then_some(tokens)
        }
        _ => None,
    };

    // On Windows, launch the game with javaw.exe rather than java.exe. The vanilla
    // launcher uses javaw, and game-capture / injection tools (OBS, Insights, Medal,
    // ...) key off the javaw.exe process name; the console-subsystem java.exe can
    // also flash a console window. Redirected stdout/stderr are still delivered to
    // javaw, so the in-app log viewer is unaffected. Fall back to the original
    // binary when a sibling javaw.exe isn't present.
    #[cfg(target_os = "windows")]
    let launch_java_path = {
        let original = std::path::Path::new(&java_component.path);
        match original.file_name().and_then(|n| n.to_str()) {
            Some(name) if name.eq_ignore_ascii_case("java.exe") => {
                let javaw = original.with_file_name("javaw.exe");
                if javaw.is_file() {
                    javaw.to_string_lossy().into_owned()
                } else {
                    java_component.path.clone()
                }
            }
            _ => java_component.path.clone(),
        }
    };
    #[cfg(not(target_os = "windows"))]
    let launch_java_path = java_component.path.clone();

    let (main_command, command_args) = match wrapper_tokens {
        Some(mut tokens) => {
            let program = tokens.remove(0);
            tokens.push(launch_java_path);
            tokens.extend(startup_command);
            (program, tokens)
        }
        None => (launch_java_path, startup_command),
    };

    let logged_command = match &secret_to_redact {
        Some(token) => command_args.join(" ").replace(token.as_str(), "<REDACTED>"),
        None => command_args.join(" "),
    };
    info!(
        "Starting Minecraft with command: {} {}",
        main_command, logged_command
    );

    let mut command_exec = tokio::process::Command::new(&main_command);
    command_exec.current_dir(instance_path.get_data_path());

    command_exec.stdout(std::process::Stdio::piped());
    command_exec.stderr(std::process::Stdio::piped());
    // Deliberately no `kill_on_drop(true)` here: dropping the handle must not
    // end the user's game. A game outlives the launcher session that started
    // it, and the pidfile written immediately after this spawn is how the
    // next core finds it again. Local servers instead have an explicit bounded
    // captured-generation stop/escalation supervisor.

    let child = command_exec.args(command_args);

    Ok(ownership.spawn(
        crate::managers::runtime_lifecycle::ActivityPolicy::Game,
        || child.spawn(),
    )?)
}

// The resolved destination is shared across instances. Weak entries can be retired
// only under this registry lock, so concurrent callers always acquire the same gate.
static NATIVE_CACHE_LOCKS: std::sync::LazyLock<
    std::sync::Mutex<HashMap<PathBuf, std::sync::Weak<Mutex<()>>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

pub async fn extract_natives(
    runtime_path: &RuntimePath,
    version: &VersionInfo,
    lwjgl_group: &LibraryGroup,
    java_arch: &JavaArch,
) -> anyhow::Result<()> {
    let mut archives = Vec::new();
    for library in version
        .libraries
        .iter()
        .chain(lwjgl_group.libraries.iter())
        .filter(|lib| library_is_allowed(lib, java_arch))
    {
        let Some(native) = library
            .natives
            .as_ref()
            .and_then(|n| n.get(&Os::native_arch(java_arch)))
        else {
            continue;
        };
        let native = native.replace("${arch}", ARCH_WIDTH);
        let Some(classifier) = library
            .downloads
            .as_ref()
            .and_then(|d| d.classifiers.as_ref())
            .and_then(|c| c.get(&native))
        else {
            warn!(
                "Library `{}` has no matching native classifier `{native}`; skipping native extraction",
                library.name
            );
            continue;
        };
        archives.push(
            runtime_path
                .get_libraries()
                .get_library_path(classifier.path.clone()),
        );
    }
    let dest = runtime_path.get_natives().get_versioned(&version.id);
    tokio::fs::create_dir_all(&dest).await?;
    let dest = native_cache_key(dest).await;
    let lock = {
        let mut locks = NATIVE_CACHE_LOCKS.lock().unwrap();
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(&dest).and_then(std::sync::Weak::upgrade) {
            lock
        } else {
            let lock = Arc::new(Mutex::new(()));
            locks.insert(dest.clone(), Arc::downgrade(&lock));
            lock
        }
    };
    let ownership = lock.lock_owned().await;
    let arch_tag = format!("{java_arch:?}");
    // The worker owns the gate and staging through all blocking IO, even if its
    // async caller is cancelled. This is in-core coordination, not a cross-core lock.
    tokio::task::spawn_blocking(move || {
        let _ownership = ownership;
        extract_native_cache(&dest, &archives, &arch_tag)
    })
    .await?
}

/// The natives folder as its lock key: its resolved path, so two spellings
/// of one folder share a lock. Some Windows volumes (RAM disks, drives the
/// Mount Manager does not map) cannot report a final path; there the folder
/// as configured is the key, and extraction uses it as 2.0.40 did.
async fn native_cache_key(dest: PathBuf) -> PathBuf {
    match tokio::fs::canonicalize(&dest).await {
        Ok(resolved) => resolved,
        Err(error) => {
            warn!(
                "Could not resolve the natives folder {}: {error}",
                dest.display()
            );
            dest
        }
    }
}

fn selected_native_outputs(
    archives: &[PathBuf],
    dest: &Path,
) -> anyhow::Result<std::collections::BTreeSet<PathBuf>> {
    let mut outputs = std::collections::BTreeSet::new();
    for path in archives {
        let mut archive = match zip::ZipArchive::new(std::fs::File::open(path)?) {
            Ok(archive) => archive,
            Err(_) => {
                // Keep the generic extractor's TAR/TAR.GZ support. These uncommon
                // native formats have no ZIP directory: inventory private output.
                let staging = tempfile::Builder::new()
                    .prefix(".gdl-native-inventory-")
                    .tempdir_in(dest)?;
                carbon_compression::decompress_blocking(path, staging.path())?;
                let mut pending = vec![staging.path().to_path_buf()];
                while let Some(directory) = pending.pop() {
                    for entry in std::fs::read_dir(directory)? {
                        let entry = entry?;
                        let kind = entry.file_type()?;
                        if kind.is_dir() {
                            pending.push(entry.path());
                        } else if kind.is_file() {
                            outputs
                                .insert(entry.path().strip_prefix(staging.path())?.to_path_buf());
                        }
                    }
                }
                continue;
            }
        };
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            if file.name().ends_with('/')
                || file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            {
                continue;
            }
            if let Some(path) = file.enclosed_name() {
                outputs.insert(path.to_path_buf());
            }
        }
    }
    Ok(outputs)
}

fn native_output_present(path: &Path) -> anyhow::Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => Ok(true),
        Ok(_) => anyhow::bail!("Native output is not a regular file: {}", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

// A narrow IO seam; production always uses the real filesystem primitive.
fn native_hard_link(source: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    {
        let hook = NATIVE_LINK_TEST_HOOKS.lock().unwrap().remove(target);
        if let Some(hook) = hook {
            return hook(source, target);
        }
    }
    std::fs::hard_link(source, target)
}

#[cfg(test)]
static NATIVE_LINK_TEST_HOOKS: std::sync::LazyLock<
    std::sync::Mutex<HashMap<PathBuf, Box<dyn FnOnce(&Path, &Path) -> std::io::Result<()> + Send>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

fn publish_missing_native(source: &Path, target: &Path) -> anyhow::Result<()> {
    if native_output_present(target)? {
        return Ok(());
    }
    // Same-filesystem staging permits atomic no-clobber installation. A concurrent
    // publisher wins without us truncating its file (including a loaded DLL).
    match native_hard_link(source, target) {
        Ok(()) => Ok(()),
        Err(e)
            if e.kind() == std::io::ErrorKind::AlreadyExists && native_output_present(target)? =>
        {
            Ok(())
        }
        Err(link_error) => {
            // Some writable volumes cannot create hard links. Use the shared
            // no-replace publication of the complete staged file; never copy
            // directly to the public target. The extraction worker owns
            // staging, including failed publication.
            match crate::util::publish::move_noclobber(source, target) {
                Ok(()) => Ok(()),
                Err(error)
                    if error.kind() == std::io::ErrorKind::AlreadyExists
                        && native_output_present(target)? => Ok(()),
                Err(error) => Err(error).with_context(|| {
                    format!("Failed to publish native {} without replacing it (hard link failed: {link_error})", target.display())
                }),
            }
        }
    }
}

#[cfg(test)]
static NATIVE_CACHE_TEST_HOOKS: std::sync::LazyLock<
    std::sync::Mutex<HashMap<PathBuf, Box<dyn FnOnce() + Send>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

fn extract_native_cache(dest: &Path, archives: &[PathBuf], arch_tag: &str) -> anyhow::Result<()> {
    let marker = dest.join(".gdl_natives_extracted");
    let same_arch = std::fs::read_to_string(&marker)
        .map(|s| s == arch_tag)
        .unwrap_or(false);
    let outputs = selected_native_outputs(archives, dest)?;
    let mut complete = true;
    for path in &outputs {
        complete &= native_output_present(&dest.join(path))?;
    }
    if same_arch && complete {
        return Ok(());
    }

    let staging = tempfile::Builder::new()
        .prefix(".gdl-native-stage-")
        .tempdir_in(dest)?;
    #[cfg(test)]
    {
        let hook = NATIVE_CACHE_TEST_HOOKS.lock().unwrap().remove(dest);
        if let Some(hook) = hook {
            hook();
        }
    }
    // Preserve the shared extractor's formats, enclosed paths, skipped links,
    // permissions and version-then-LWJGL last-entry-wins extraction order.
    for path in archives {
        carbon_compression::decompress_blocking(path, staging.path())
            .with_context(|| format!("Failed to extract native archive {}", path.display()))?;
    }
    for path in &outputs {
        let target = dest.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if same_arch {
            publish_missing_native(&staging.path().join(path), &target)?;
        } else {
            // Keep the existing different/unknown-architecture replacement policy.
            // The shared version path does not provide cross-architecture live isolation.
            std::fs::copy(staging.path().join(path), &target)?;
        }
    }
    for path in &outputs {
        anyhow::ensure!(
            native_output_present(&dest.join(path))?,
            "Native output missing after extraction: {}",
            path.display()
        );
    }
    if let Err(e) = std::fs::write(marker, arch_tag.as_bytes()) {
        warn!("Failed to write natives extraction marker: {e}");
    }
    Ok(())
}

/// Cheap sentinel for "the natives directory still holds real output", not just its
/// completion marker: at least one file with this platform's native library extension
/// (`.dll` / `.so` / `.dylib`, via [`std::env::consts::DLL_EXTENSION`]). This doesn't
/// verify every expected native is present -- that would mean re-deriving the exact
/// expected file list from each native archive -- but it's enough to catch the
/// directory having been emptied or partially wiped out from under an intact marker
/// (the marker itself never has this extension, so its mere presence can't pass this
/// check).
#[cfg(test)]
async fn natives_dir_has_libraries(dest: &Path) -> bool {
    let Ok(mut entries) = tokio::fs::read_dir(dest).await else {
        return false;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let is_native_library = entry.path().extension().and_then(|ext| ext.to_str())
            == Some(std::env::consts::DLL_EXTENSION);

        if is_native_library {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::minecraft::minecraft::library_into_natives_downloadable,
        managers::java::java_checker::{JavaChecker, RealJavaChecker},
        setup_managers_for_test,
    };

    use super::*;
    use carbon_net::{DownloadOptions, Progress};
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
            &app.db,
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

        let java_component = RealJavaChecker::new(app.runtime_lifecycle.clone())
            .get_bin_info(
                &PathBuf::from("java"),
                crate::domain::java::JavaComponentType::Local,
            )
            .await
            .unwrap();

        let assets_dir = crate::managers::minecraft::assets::get_assets_dir(
            &app.db,
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

    #[test]
    fn task14_extra_java_args_retains_explicit_empty_tokens() {
        assert_eq!(parse_extra_java_args(r#"a "" " " b"#), ["a", "", " ", "b"]);
        assert!(parse_extra_java_args(" \t\n ").is_empty());
    }

    /// A blank argument before the main class makes Java exit with "Could not
    /// find or load main class", so launch never passes one.
    #[test]
    fn blank_extra_java_args_never_reach_the_command_line() {
        for input in [
            r#"-XX:+UseG1GC """#,
            r#"-XX:+UseG1GC ""#,
            r#""" -XX:+UseG1GC " ""#,
            "-XX:+UseG1GC \"\t\"",
        ] {
            assert_eq!(extra_java_launch_args(input), ["-XX:+UseG1GC"], "{input}");
        }
        assert_eq!(
            extra_java_launch_args(r#"-Dlabel="My Pack" " padded""#),
            ["-Dlabel=My Pack", " padded"]
        );
        assert!(extra_java_launch_args(r#""" "  ""#).is_empty());
    }

    /// Pins the launch path itself: Java takes the first argument that is not an
    /// option as the main class, so a blank one must not come before it.
    #[tokio::test]
    async fn the_startup_command_has_no_blank_argument_before_the_main_class() {
        let dir = tempfile::tempdir().unwrap();
        let version: VersionInfo = serde_json::from_value(serde_json::json!({
            "id": "fixture",
            "assetIndex": {"id": "test", "sha1": "", "size": 0, "totalSize": 0, "url": "https://fixture.invalid/assets"},
            "assets": "test",
            "downloads": {},
            "libraries": [{
                "name": "fixture:library:1",
                "downloads": {"artifact": {"path": "fixture/library.jar", "sha1": "", "size": 0, "url": "https://fixture.invalid/library.jar"}}
            }],
            "mainClass": "fixture.Main",
            "minimumLauncherVersion": 0,
            "releaseTime": "2025-01-01T00:00:00Z",
            "time": "2025-01-01T00:00:00Z",
            "type": "release"
        }))
        .unwrap();
        let lwjgl_group: LibraryGroup = serde_json::from_value(serde_json::json!({
            "id": "test", "version": "1", "uid": "test", "releaseTime": "2025-01-01T00:00:00Z", "type": "release", "libraries": []
        }))
        .unwrap();
        let java_component = JavaComponent {
            path: "java".to_owned(),
            arch: JavaArch::X86_64,
            os: crate::domain::java::JavaOs::Linux,
            _type: crate::domain::java::JavaComponentType::Local,
            version: crate::domain::java::JavaVersion::from_major(21),
            vendor: "fixture".to_owned(),
        };
        let command = generate_startup_command(
            java_component,
            get_account().await,
            2048,
            1024,
            None,
            r#"-XX:+UseG1GC "" " ""#,
            &RuntimePath::new(dir.path().to_path_buf()),
            version,
            &lwjgl_group,
            InstancePath::new(dir.path().join("instance")),
            crate::managers::minecraft::assets::AssetsDir::Index(dir.path().join("assets")),
        )
        .await
        .unwrap();

        let main_class = command
            .iter()
            .position(|arg| arg == "fixture.Main")
            .unwrap();
        let blank = command[..main_class]
            .iter()
            .find(|arg| arg.trim().is_empty());
        assert_eq!(blank, None, "{command:?}");
        assert_eq!(command[main_class - 1], "-XX:+UseG1GC", "{command:?}");
    }

    #[test]
    fn extra_java_args_pass_agents_through_unmodified() {
        // `-javaagent`/`-agentlib`/`-agentpath` are user-authored JVM args and
        // must be passed through unmodified, never filtered out.
        assert_eq!(
            parse_extra_java_args("-javaagent:/opt/unlim/agent.jar -Xss8m -agentlib:hprof"),
            vec![
                "-javaagent:/opt/unlim/agent.jar".to_string(),
                "-Xss8m".to_string(),
                "-agentlib:hprof".to_string(),
            ],
        );
    }

    #[test]
    fn extra_java_args_support_quoted_paths_with_spaces() {
        // A quoted path containing spaces must stay a single argument. If it
        // split at the space, the JVM would treat the fragment after the space
        // as the main class instead of part of the -javaagent value.
        assert_eq!(
            parse_extra_java_args(r#"-javaagent:"C:\Program Files\Unlim\agent.jar" -Dx=1"#),
            vec![
                r"-javaagent:C:\Program Files\Unlim\agent.jar".to_string(),
                "-Dx=1".to_string(),
            ],
        );
        // Wrapping the entire argument in quotes is also accepted.
        assert_eq!(
            parse_extra_java_args(r#""-javaagent:C:\Program Files\Unlim\agent.jar""#),
            vec![r"-javaagent:C:\Program Files\Unlim\agent.jar".to_string()],
        );
    }

    #[test]
    fn extra_java_args_handle_escapes_and_blanks() {
        assert_eq!(parse_extra_java_args(""), Vec::<String>::new());
        // The global + per-instance args are joined with a space, so an empty
        // config yields a blank string that must produce no arguments.
        assert_eq!(parse_extra_java_args("   "), Vec::<String>::new());
        assert_eq!(
            parse_extra_java_args(r#"-Dmsg="a\"b" -Dpath=C:\\tmp"#),
            vec![r#"-Dmsg=a"b"#.to_string(), r"-Dpath=C:\tmp".to_string()],
        );
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
            &app.db,
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

        carbon_net::download_multiple(
            &downloadables[..],
            DownloadOptions::builder().concurrency(10).build(),
        )
        .await
        .unwrap();

        extract_natives(runtime_path, &version, &lwjgl_group, &JavaArch::X86_64)
            .await
            .unwrap();

        // Regression check: delete the actual extracted libraries but leave the
        // completion marker in place -- exactly what an AV quarantine or an uneven
        // cache wipe leaves behind -- and confirm a second call re-extracts instead
        // of silently trusting the now-stale marker.
        let dest = runtime_path.get_natives().get_versioned(&version.id);
        let native_ext = std::env::consts::DLL_EXTENSION;
        let count_native_files = |dir: &Path| {
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|entry| {
                    entry.path().extension().and_then(|ext| ext.to_str()) == Some(native_ext)
                })
                .count()
        };

        let extracted_before = count_native_files(&dest);
        assert!(
            extracted_before > 0,
            "expected at least one extracted native library"
        );

        for entry in std::fs::read_dir(&dest).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some(native_ext) {
                std::fs::remove_file(&path).unwrap();
            }
        }
        assert_eq!(count_native_files(&dest), 0);

        extract_natives(runtime_path, &version, &lwjgl_group, &JavaArch::X86_64)
            .await
            .unwrap();

        assert_eq!(
            count_native_files(&dest),
            extracted_before,
            "natives must be re-extracted when the marker is present but the actual libraries are missing"
        );
    }

    /// Where a volume cannot report a folder's final path (some Windows RAM
    /// disks and virtual drives), natives extraction keys its lock on the
    /// folder as configured instead of failing every Play.
    #[cfg(unix)]
    #[tokio::test]
    async fn native_cache_key_falls_back_to_the_configured_folder() {
        let dir = tempfile::tempdir().unwrap();
        let natives = dir.path().join("natives/1.20.1");
        std::fs::create_dir_all(&natives).unwrap();
        assert_eq!(
            native_cache_key(natives.clone()).await,
            std::fs::canonicalize(&natives).unwrap()
        );
        // A link that leads to itself cannot be resolved either.
        std::os::unix::fs::symlink("loop", dir.path().join("loop")).unwrap();
        let unresolvable = dir.path().join("loop/1.20.1");
        assert!(std::fs::canonicalize(&unresolvable).is_err());
        assert_eq!(native_cache_key(unresolvable.clone()).await, unresolvable);
    }

    #[tokio::test]
    async fn natives_dir_has_libraries_detects_missing_native_files() {
        let native_ext = std::env::consts::DLL_EXTENSION;

        // Directory doesn't exist at all.
        let base = tempfile::tempdir().unwrap();
        let missing_dir = base.path().join("does-not-exist");
        assert!(!natives_dir_has_libraries(&missing_dir).await);

        // Present but empty.
        let empty = tempfile::tempdir().unwrap();
        assert!(!natives_dir_has_libraries(empty.path()).await);

        // Present with only the completion marker -- exactly what an AV quarantine
        // or an uneven cache wipe leaves behind: the marker survives, the natives
        // don't.
        let marker_only = tempfile::tempdir().unwrap();
        tokio::fs::write(marker_only.path().join(".gdl_natives_extracted"), b"X86_64")
            .await
            .unwrap();
        assert!(!natives_dir_has_libraries(marker_only.path()).await);

        // Present with an actual native library alongside the marker.
        let healthy = tempfile::tempdir().unwrap();
        tokio::fs::write(healthy.path().join(".gdl_natives_extracted"), b"X86_64")
            .await
            .unwrap();
        tokio::fs::write(
            healthy.path().join(format!("liblwjgl.{native_ext}")),
            b"not a real library, just a sentinel-check fixture",
        )
        .await
        .unwrap();
        assert!(natives_dir_has_libraries(healthy.path()).await);
    }
}

#[cfg(test)]
mod task17_native_tests {
    use super::*;
    use std::io::Write;

    fn fixture() -> (tempfile::TempDir, RuntimePath, VersionInfo, LibraryGroup) {
        let dir = tempfile::tempdir().unwrap();
        let runtime = RuntimePath::new(dir.path().to_path_buf());
        let native_os = serde_json::to_value(Os::native_arch(&JavaArch::X86_64)).unwrap();
        let mut natives = serde_json::Map::new();
        natives.insert(
            native_os.as_str().unwrap().into(),
            serde_json::json!("native-${arch}"),
        );
        let lib: Library = serde_json::from_value(serde_json::json!({
            "name":"fixture:natives:1", "natives":natives,
            "downloads":{"classifiers":{format!("native-{}", ARCH_WIDTH):{
                "path":"fixture/native.jar", "sha1":"", "size":0, "url":"https://fixture.invalid/native.jar"
            }}},
        })).unwrap();
        let version: VersionInfo = serde_json::from_value(serde_json::json!({
            "id":"task17", "assetIndex":{"id":"test","sha1":"","size":0,"totalSize":0,"url":"https://fixture.invalid/assets"},
            "assets":"test", "downloads":{}, "libraries":[lib], "mainClass":"fixture.Main", "minimumLauncherVersion":0,
            "releaseTime":"2025-01-01T00:00:00Z", "time":"2025-01-01T00:00:00Z", "type":"release"
        })).unwrap();
        let group: LibraryGroup = serde_json::from_value(serde_json::json!({
            "id":"test", "version":"1", "uid":"test", "releaseTime":"2025-01-01T00:00:00Z", "type":"release", "libraries":[]
        })).unwrap();
        let archive = runtime
            .get_libraries()
            .get_library_path("fixture/native.jar".into());
        std::fs::create_dir_all(archive.parent().unwrap()).unwrap();
        let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
        for (name, bytes) in [
            (format!("one.{}", std::env::consts::DLL_EXTENSION), b"one"),
            (
                format!("nested/two.{}", std::env::consts::DLL_EXTENSION),
                b"two",
            ),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        (dir, runtime, version, group)
    }

    #[tokio::test]
    async fn task17_native_cache_repairs_selected_missing_nested_output() {
        let (_dir, runtime, version, group) = fixture();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        let dest = runtime.get_natives().get_versioned(&version.id);
        let one = dest.join(format!("one.{}", std::env::consts::DLL_EXTENSION));
        let two = dest.join(format!("nested/two.{}", std::env::consts::DLL_EXTENSION));
        let before = std::fs::metadata(&one).unwrap().modified().unwrap();
        std::fs::remove_file(&two).unwrap();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        assert_eq!(std::fs::read(two).unwrap(), b"two");
        assert_eq!(std::fs::metadata(&one).unwrap().modified().unwrap(), before);
        assert_eq!(std::fs::read(one).unwrap(), b"one");
    }

    #[tokio::test]
    async fn task17_native_cache_complete_empty_and_mismatched_controls() {
        let (_dir, runtime, version, group) = fixture();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        let dest = runtime.get_natives().get_versioned(&version.id);
        let one = dest.join(format!("one.{}", std::env::consts::DLL_EXTENSION));
        std::fs::write(&one, b"healthy-live").unwrap();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&one).unwrap(), b"healthy-live");
        std::fs::write(dest.join(".gdl_natives_extracted"), b"different-arch").unwrap();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&one).unwrap(), b"one");
        std::fs::remove_file(&one).unwrap();
        std::fs::remove_file(dest.join(format!("nested/two.{}", std::env::consts::DLL_EXTENSION)))
            .unwrap();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&one).unwrap(), b"one");
    }
    #[tokio::test]
    async fn task17_native_cache_selection_failure_and_occupied_output() {
        let (_dir, runtime, mut version, group) = fixture();
        let mut denied = version.libraries[0].clone();
        denied.rules = Some(vec![
            serde_json::from_value(serde_json::json!({"action":"disallow"})).unwrap(),
        ]);
        denied
            .downloads
            .as_mut()
            .unwrap()
            .classifiers
            .as_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap()
            .path = "absent/denied.jar".into();
        version.libraries.push(denied);
        let mut missing_classifier = version.libraries[0].clone();
        missing_classifier.downloads = None;
        version.libraries.push(missing_classifier);
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        let dest = runtime.get_natives().get_versioned(&version.id);
        let two = dest.join(format!("nested/two.{}", std::env::consts::DLL_EXTENSION));
        std::fs::remove_file(&two).unwrap();
        std::fs::create_dir(&two).unwrap();
        assert!(
            extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
                .await
                .is_err()
        );
        assert!(two.is_dir());
        std::fs::remove_dir(&two).unwrap();
        std::fs::write(
            runtime
                .get_libraries()
                .get_library_path("fixture/native.jar".into()),
            b"invalid archive",
        )
        .unwrap();
        assert!(
            extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
                .await
                .is_err()
        );
        assert_eq!(
            std::fs::read(dest.join(format!("one.{}", std::env::consts::DLL_EXTENSION))).unwrap(),
            b"one"
        );
    }

    #[tokio::test]
    async fn task17_native_cache_cancelled_waiter_keeps_worker_gate_and_stage() {
        let (_dir, runtime, version, group) = fixture();
        let dest = runtime.get_natives().get_versioned(&version.id);
        std::fs::create_dir_all(&dest).unwrap();
        let dest = dest.canonicalize().unwrap();
        let (entered, wait_entered) = tokio::sync::oneshot::channel();
        let (release, wait_release) = std::sync::mpsc::channel();
        NATIVE_CACHE_TEST_HOOKS.lock().unwrap().insert(
            dest.clone(),
            Box::new(move || {
                entered.send(()).unwrap();
                wait_release.recv().unwrap();
            }),
        );
        let task = {
            let runtime = runtime.clone();
            let version = version.clone();
            let group = group.clone();
            tokio::spawn(async move {
                extract_natives(&runtime, &version, &group, &JavaArch::X86_64).await
            })
        };
        wait_entered.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(std::fs::read_dir(&dest).unwrap().any(|p| {
            p.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".gdl-native-stage-")
        }));
        let retry = extract_natives(&runtime, &version, &group, &JavaArch::X86_64);
        tokio::pin!(retry);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut retry)
                .await
                .is_err()
        );
        release.send(()).unwrap();
        retry.await.unwrap();
        assert_eq!(
            std::fs::read(dest.join(format!("nested/two.{}", std::env::consts::DLL_EXTENSION)))
                .unwrap(),
            b"two"
        );
        assert!(!std::fs::read_dir(&dest).unwrap().any(|p| {
            p.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".gdl-native-stage-")
        }));
    }

    #[test]
    fn task17_native_cache_publication_never_clobbers_existing() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        std::fs::write(&source, b"archive").unwrap();
        std::fs::write(&target, b"live").unwrap();
        publish_missing_native(&source, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"live");
    }

    #[tokio::test]
    async fn task17_native_cache_nested_only_is_complete_and_lwjgl_wins() {
        let (_dir, runtime, version, mut group) = fixture();
        let archive = runtime
            .get_libraries()
            .get_library_path("fixture/native.jar".into());
        let nested = "platform/arch/libfixture.dylib";
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        zip.start_file(nested, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"base").unwrap();
        zip.finish().unwrap();
        let mut last = version.libraries[0].clone();
        last.downloads
            .as_mut()
            .unwrap()
            .classifiers
            .as_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap()
            .path = "fixture/lwjgl.jar".into();
        group.libraries.push(last);
        let mut zip = zip::ZipWriter::new(
            std::fs::File::create(
                runtime
                    .get_libraries()
                    .get_library_path("fixture/lwjgl.jar".into()),
            )
            .unwrap(),
        );
        zip.start_file(nested, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"last").unwrap();
        zip.finish().unwrap();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        let output = runtime
            .get_natives()
            .get_versioned(&version.id)
            .join(nested);
        assert_eq!(std::fs::read(&output).unwrap(), b"last");
        std::fs::write(&output, b"healthy nested live").unwrap();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"healthy nested live");
        std::fs::remove_file(&output).unwrap();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        assert_eq!(std::fs::read(output).unwrap(), b"last");
    }
    #[tokio::test]
    async fn task17_native_cache_retains_generic_archive_formats() {
        let cases: &[(&[u8], &str)] = &[
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../carbon_compression/fixtures/compressed.tar"
                )),
                "compressed.txt",
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../carbon_compression/fixtures/compressed.tar.gz"
                )),
                "3588ba2df450f9482cb035d9c2cc894f.txt",
            ),
        ];
        for (bytes, name) in cases {
            let (_dir, runtime, version, group) = fixture();
            let archive = runtime
                .get_libraries()
                .get_library_path("fixture/native.jar".into());
            std::fs::write(&archive, bytes).unwrap();
            extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
                .await
                .unwrap();
            let output = runtime.get_natives().get_versioned(&version.id).join(name);
            assert_eq!(std::fs::read(&output).unwrap(), b"Some compressed file");
            std::fs::write(&output, b"healthy").unwrap();
            extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
                .await
                .unwrap();
            assert_eq!(std::fs::read(&output).unwrap(), b"healthy");
            std::fs::remove_file(&output).unwrap();
            extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
                .await
                .unwrap();
            assert_eq!(std::fs::read(&output).unwrap(), b"Some compressed file");
        }
    }
    async fn rejected_link_repair(empty: bool) {
        let (_dir, runtime, version, group) = fixture();
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .unwrap();
        let dest = runtime
            .get_natives()
            .get_versioned(&version.id)
            .canonicalize()
            .unwrap();
        let one = dest.join(format!("one.{}", std::env::consts::DLL_EXTENSION));
        let two = dest.join(format!("nested/two.{}", std::env::consts::DLL_EXTENSION));
        if empty {
            std::fs::remove_file(&one).unwrap();
        } else {
            std::fs::write(&one, b"healthy sibling").unwrap();
        }
        let healthy_mtime = std::fs::metadata(&one).map(|m| m.modified().unwrap()).ok();
        std::fs::remove_file(&two).unwrap();
        for target in std::iter::once(two.clone()).chain(empty.then_some(one.clone())) {
            NATIVE_LINK_TEST_HOOKS.lock().unwrap().insert(
                target,
                Box::new(move |source, target| {
                    assert!(
                        !target.exists(),
                        "publication must begin without a live partial file"
                    );
                    assert!(source.is_file(), "staging must remain owned");
                    let kind = if empty {
                        std::io::ErrorKind::PermissionDenied
                    } else {
                        std::io::ErrorKind::Unsupported
                    };
                    Err(std::io::Error::new(
                        kind,
                        "controlled writable volume without hard links",
                    ))
                }),
            );
        }
        extract_natives(&runtime, &version, &group, &JavaArch::X86_64)
            .await
            .expect("writable cache must repair even if hard links are unsupported");
        assert_eq!(std::fs::read(&two).unwrap(), b"two");
        if empty {
            assert_eq!(std::fs::read(&one).unwrap(), b"one");
        } else {
            assert_eq!(std::fs::read(&one).unwrap(), b"healthy sibling");
            assert_eq!(
                std::fs::metadata(&one).unwrap().modified().unwrap(),
                healthy_mtime.unwrap()
            );
        }
        assert_eq!(
            std::fs::read_to_string(dest.join(".gdl_natives_extracted")).unwrap(),
            "X86_64"
        );
    }

    #[tokio::test]
    async fn task17_r1_native_partial_cache_repairs_without_hard_links() {
        rejected_link_repair(false).await;
    }

    #[tokio::test]
    async fn task17_r1_native_empty_cache_repairs_without_hard_links() {
        rejected_link_repair(true).await;
    }

    #[test]
    fn task17_r1_native_fallback_preserves_competing_regular_occupant() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("staged");
        let target = dir.path().join("output");
        std::fs::write(&source, b"complete archive output").unwrap();
        NATIVE_LINK_TEST_HOOKS.lock().unwrap().insert(
            target.clone(),
            Box::new(|_, target| {
                assert!(!target.exists());
                std::fs::write(target, b"competing winner").unwrap();
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "controlled hard link denial",
                ))
            }),
        );
        publish_missing_native(&source, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"competing winner");
        assert_eq!(std::fs::read(&source).unwrap(), b"complete archive output");
    }

    #[test]
    fn task17_r1_native_fallback_rejects_competing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("staged");
        let target = dir.path().join("output");
        std::fs::write(&source, b"complete archive output").unwrap();
        NATIVE_LINK_TEST_HOOKS.lock().unwrap().insert(
            target.clone(),
            Box::new(|_, target| {
                assert!(!target.exists());
                std::fs::create_dir(target).unwrap();
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "controlled hard link denial",
                ))
            }),
        );
        assert!(publish_missing_native(&source, &target).is_err());
        assert!(target.is_dir());
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 0);
        assert_eq!(std::fs::read(&source).unwrap(), b"complete archive output");
    }

    #[test]
    fn task17_r1_native_failed_fallback_retains_owned_staging() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("staged");
        let parent = dir.path().join("target-parent");
        std::fs::create_dir(&parent).unwrap();
        let target = parent.join("output");
        std::fs::write(&source, b"complete archive output").unwrap();
        NATIVE_LINK_TEST_HOOKS.lock().unwrap().insert(
            target.clone(),
            Box::new(|_, target| {
                assert!(!target.exists());
                let parent = target.parent().unwrap();
                std::fs::remove_dir(parent).unwrap();
                std::fs::write(parent, b"competing parent occupant").unwrap();
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "controlled hard link denial",
                ))
            }),
        );
        let error = publish_missing_native(&source, &target).unwrap_err();
        assert!(error.to_string().contains("controlled hard link denial"));
        assert!(error.to_string().contains("without replacing"));
        assert!(!target.exists());
        assert_eq!(
            std::fs::read(&parent).unwrap(),
            b"competing parent occupant"
        );
        assert_eq!(std::fs::read(&source).unwrap(), b"complete archive output");
    }
}
