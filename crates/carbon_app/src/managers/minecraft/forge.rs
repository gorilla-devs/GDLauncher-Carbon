use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::bail;
use prisma_client_rust::QueryError;
use thiserror::Error;
use tokio::process::Command;

use crate::{
    domain::{
        maven::MavenCoordinates,
        minecraft::modded::{ModdedManifest, Processor, SidedDataEntry},
        runtime_path::{InstancePath, LibrariesPath},
    },
    managers::java::utils::PATH_SEPARATOR,
};

#[derive(Error, Debug)]
pub enum ForgeManifestError {
    #[error("Could not fetch forge manifest from launchermeta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Manifest database query error: {0}")]
    DBQueryError(#[from] QueryError),
}

pub async fn get_manifest(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    meta_base_url: &reqwest::Url,
) -> anyhow::Result<ModdedManifest> {
    let server_url = meta_base_url.join("forge/v0/manifest.json")?;
    let new_manifest = reqwest_client
        .get(server_url)
        .send()
        .await?
        .json::<ModdedManifest>()
        .await?;

    Ok(new_manifest)
}

pub async fn get_version(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    manifest_version_meta: crate::domain::minecraft::modded::ModdedManifestLoaderVersion,
) -> anyhow::Result<crate::domain::minecraft::modded::PartialVersionInfo> {
    let server_url = reqwest::Url::parse(&manifest_version_meta.url)?;
    let new_manifest = reqwest_client
        .get(server_url)
        .send()
        .await?
        .json::<crate::domain::minecraft::modded::PartialVersionInfo>()
        .await?;

    Ok(new_manifest)
}

fn get_class_paths_jar(libraries_path: &Path, libraries: &[String]) -> anyhow::Result<String> {
    let cps = libraries
        .iter()
        .map(|library| {
            let library_path = MavenCoordinates::try_from(library.to_owned(), None)?.into_path();
            let library_path = libraries_path.join(library_path);
            if !library_path.exists() {
                bail!("Library {} does not exist", library);
            }

            Ok(library_path.to_string_lossy().to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(cps.join(PATH_SEPARATOR))
}

async fn get_processor_main_class(path: String) -> anyhow::Result<Option<String>> {
    let main_class = tokio::task::spawn_blocking(move || {
        let zipfile = std::fs::File::open(&path)?;
        let mut archive = zip::ZipArchive::new(zipfile)
            .map_err(|_| anyhow::anyhow!("Cannot read processor at {}", path))?;

        let file = archive
            .by_name("META-INF/MANIFEST.MF")
            .map_err(|_| anyhow::anyhow!("Cannot read processor manifest at {}", path))?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            let mut line = line?;
            line.retain(|c| !c.is_whitespace());

            if line.starts_with("Main-Class:") {
                if let Some(class) = line.split(':').nth(1) {
                    return Ok(Some(class.to_string()));
                }
            }
        }

        Ok::<Option<String>, anyhow::Error>(None)
    })
    .await??;

    Ok(main_class)
}

fn get_processor_arguments<T: AsRef<str>>(
    libraries_path: &Path,
    arguments: &[T],
    data: &HashMap<String, SidedDataEntry>,
) -> anyhow::Result<Vec<String>> {
    let mut new_arguments = Vec::new();

    for argument in arguments {
        let trimmed_arg = &argument.as_ref()[1..argument.as_ref().len() - 1];
        if argument.as_ref().starts_with('{') {
            if let Some(entry) = data.get(trimmed_arg) {
                new_arguments.push(if entry.client.starts_with('[') {
                    libraries_path
                        .join(
                            MavenCoordinates::try_from(
                                entry.client[1..entry.client.len() - 1].to_string(),
                                None,
                            )?
                            .into_path(),
                        )
                        .to_string_lossy()
                        .to_string()
                } else {
                    entry.client.clone()
                })
            }
        } else if argument.as_ref().starts_with('[') {
            new_arguments.push(
                libraries_path
                    .join(MavenCoordinates::try_from(trimmed_arg.to_string(), None)?.into_path())
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            new_arguments.push(argument.as_ref().to_string())
        }
    }

    Ok(new_arguments)
}

macro_rules! augment_data {
    ($dest:expr; $($name:literal : client => $client:expr, server => $server:expr;)+) => {
        $(std::collections::HashMap::insert(
            $dest,
            String::from($name),
            SidedDataEntry {
                client: $client.to_string(),
                server: $server.to_string(),
            },
        );)+
    }
}

pub async fn execute_processors(
    processors: &Vec<Processor>,
    data: &HashMap<String, SidedDataEntry>,
    java_binary: PathBuf,
    instance_path: InstancePath,
    client_path: PathBuf,
    game_version: String,
    libraries_path: LibrariesPath,
) -> anyhow::Result<()> {
    let mut data = data.clone();
    augment_data! {
        &mut data;
        "SIDE":
            client => "client",
            server => "";
        "MINECRAFT_JAR" :
            client => client_path.to_string_lossy(),
            server => "";
        "MINECRAFT_VERSION":
            client => game_version,
            server => "";
        "ROOT":
            client => instance_path.get_data_path().to_string_lossy(),
            server => "";
        "LIBRARY_DIR":
            client => libraries_path.to_path().to_string_lossy(),
            server => "";
    }

    for processor in processors {
        if let Some(sides) = &processor.sides {
            if !sides.contains(&"client".to_string()) {
                continue;
            }
        }
        let mut classpath = vec![];
        classpath.extend(processor.classpath.clone());
        classpath.push(processor.jar.clone());

        let classpath = get_class_paths_jar(&libraries_path.to_path(), &classpath)?;
        let main_class_path = libraries_path
            .to_path()
            .join(MavenCoordinates::try_from(processor.jar.clone(), None)?.into_path());
        let main_class = get_processor_main_class(main_class_path.to_string_lossy().to_string())
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Could not find processor main class for {}", processor.jar)
            })?;
        let arguments = get_processor_arguments(&libraries_path.to_path(), &processor.args, &data)?;

        let child = Command::new(java_binary.to_string_lossy().to_string())
            .arg("-cp")
            .arg(classpath)
            .arg(main_class)
            .args(arguments)
            .output()
            .await
            .map_err(|err| {
                anyhow::anyhow!("Could not execute processor {}: {}", processor.jar, err)
            })?;

        // println!("{}", String::from_utf8_lossy(&child.stdout));

        if !child.status.success() {
            bail!(
                "Processor {} exited with code {}",
                processor.jar,
                child.status.code().unwrap_or(-1)
            );
        }
    }
    Ok(())
}
