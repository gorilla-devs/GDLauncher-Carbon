//! Finds the modloader of a CurseForge server pack that does not carry it in a
//! place the file scan recognises, so the pack is never installed as a plain
//! vanilla server with its mods left unused.

use anyhow::Context;
use std::io::Read;
use std::path::Path;

/// Folders NeoForge installs under: its own, and Forge's layout that it kept
/// for Minecraft 1.20.1.
pub(crate) const NEOFORGE_VENDOR_PATHS: [&str; 2] =
    ["net/neoforged/neoforge", "net/neoforged/forge"];

const LOADERS: [&str; 4] = ["forge", "neoforge", "fabric", "quilt"];
/// Far above any real manifest or jar manifest.
const TEXT_LIMIT: u64 = 16 * 1024 * 1024;

/// Entries archiving tools add next to the pack's own folder.
pub(super) fn is_archive_clutter(first_component: &str) -> bool {
    first_component == "__MACOSX" || first_component == ".DS_Store"
}

/// The Fabric launcher jar at the data root: the name the installer writes, or
/// the name the Fabric site serves it under
/// (`fabric-server-mc.1.20.1-loader.0.15.11-launcher.1.0.1.jar`).
pub(crate) fn fabric_launcher_jar(data_path: &Path) -> Option<String> {
    const INSTALLED: &str = "fabric-server-launch.jar";
    if data_path.join(INSTALLED).is_file() {
        return Some(INSTALLED.to_string());
    }
    let mut served: Vec<String> = std::fs::read_dir(data_path)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("fabric-server-") && name.ends_with(".jar"))
        .collect();
    served.sort();
    served.pop()
}

/// The loader version in a launcher jar name the Fabric site serves.
pub(super) fn fabric_loader_version(jar_name: &str) -> Option<String> {
    let (_, rest) = jar_name.split_once("-loader.")?;
    let (version, _) = rest.split_once("-launcher.")?;
    (!version.is_empty()).then(|| version.to_string())
}

fn known_loader(loader: &str, version: &str) -> Option<(String, String)> {
    let loader = loader.trim().to_ascii_lowercase();
    let version = version.trim();
    (LOADERS.contains(&loader.as_str()) && !version.is_empty())
        .then(|| (loader, version.to_string()))
}

/// The loader the modpack's own `manifest.json` names (`forge-47.2.0`).
pub(super) fn loader_from_manifest(manifest: &str) -> Option<(String, String)> {
    let manifest: carbon_platforms::curseforge::manifest::Manifest =
        serde_json::from_str(manifest).ok()?;
    let loaders = manifest.minecraft.mod_loaders;
    let chosen = loaders
        .iter()
        .find(|loader| loader.primary)
        .or(loaders.first())?;
    let (loader, version) = chosen.id.split_once('-')?;
    known_loader(loader, version)
}

/// The loader in the `variables.txt` that ServerPackCreator packs carry for
/// their start script (`MODLOADER=Forge`, `MODLOADER_VERSION=47.2.0`).
pub(super) fn loader_from_variables(variables: &str) -> Option<(String, String)> {
    let value = |key: &str| {
        variables.lines().find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim() == key).then(|| value.trim().trim_matches(['"', '\'']).to_string())
        })
    };
    known_loader(&value("MODLOADER")?, &value("MODLOADER_VERSION")?)
}

fn zip_text(archive: &Path, name: &str) -> anyhow::Result<String> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
    let mut text = String::new();
    archive
        .by_name(name)?
        .take(TEXT_LIMIT)
        .read_to_string(&mut text)?;
    Ok(text)
}

/// The `manifest.json` of a modpack archive.
pub(super) fn client_manifest(archive: &Path) -> anyhow::Result<String> {
    zip_text(archive, "manifest.json").context("Modpack archive has no readable manifest.json")
}

/// True when the pack brings a `server.jar` that is not Minecraft's own: a
/// loader or bootstrap under that name, which starts the mods by itself.
pub(super) fn ships_own_launcher(data_path: &Path) -> bool {
    let Ok(manifest) = zip_text(&data_path.join("server.jar"), "META-INF/MANIFEST.MF") else {
        return false;
    };
    manifest
        .lines()
        .find_map(|line| line.strip_prefix("Main-Class:"))
        .is_some_and(|main| !main.trim().starts_with("net.minecraft."))
}

/// True when the pack has at least one mod for a loader to run.
pub(super) fn has_mods(data_path: &Path) -> bool {
    std::fs::read_dir(data_path.join("mods")).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry.path().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn jar(path: &Path, main_class: &str) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        zip.start_file(
            "META-INF/MANIFEST.MF",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        write!(zip, "Manifest-Version: 1.0\r\nMain-Class: {main_class}\r\n").unwrap();
        zip.finish().unwrap();
    }

    fn manifest(loaders: &str) -> String {
        format!(
            r#"{{"minecraft":{{"version":"1.20.1","modLoaders":{loaders}}},"manifestType":"minecraftModpack","manifestVersion":1,"name":"Pack","author":"A","overrides":"overrides","files":[]}}"#
        )
    }

    fn pair(loader: &str, version: &str) -> Option<(String, String)> {
        Some((loader.to_string(), version.to_string()))
    }

    #[test]
    fn reads_the_loader_a_modpack_manifest_names() {
        assert_eq!(
            loader_from_manifest(&manifest(r#"[{"id":"forge-47.2.0","primary":true}]"#)),
            pair("forge", "47.2.0")
        );
        assert_eq!(
            loader_from_manifest(&manifest(r#"[{"id":"neoforge-21.1.77","primary":true}]"#)),
            pair("neoforge", "21.1.77")
        );
        assert_eq!(
            loader_from_manifest(&manifest(r#"[{"id":"fabric-0.15.11","primary":true}]"#)),
            pair("fabric", "0.15.11")
        );
        // Quilt versions carry a dash of their own.
        assert_eq!(
            loader_from_manifest(&manifest(
                r#"[{"id":"quilt-0.26.0-beta.1","primary":true}]"#
            )),
            pair("quilt", "0.26.0-beta.1")
        );
    }

    #[test]
    fn prefers_the_primary_loader_and_refuses_what_it_cannot_install() {
        assert_eq!(
            loader_from_manifest(&manifest(
                r#"[{"id":"fabric-0.15.11","primary":false},{"id":"forge-47.2.0","primary":true}]"#
            )),
            pair("forge", "47.2.0")
        );
        assert_eq!(
            loader_from_manifest(&manifest(r#"[{"id":"forge-47.2.0","primary":false}]"#)),
            pair("forge", "47.2.0")
        );
        for unusable in [
            "[]",
            r#"[{"id":"forge","primary":true}]"#,
            r#"[{"id":"forge-","primary":true}]"#,
            r#"[{"id":"liteloader-1.12.2","primary":true}]"#,
        ] {
            assert_eq!(
                loader_from_manifest(&manifest(unusable)),
                None,
                "{unusable}"
            );
        }
        assert_eq!(loader_from_manifest("not json"), None);
    }

    #[test]
    fn reads_the_loader_from_a_start_script_variables_file() {
        let variables =
            "MINECRAFT_VERSION=1.20.1\nMODLOADER=Forge\nMODLOADER_VERSION=47.2.0\nJAVA=\"java\"\n";
        assert_eq!(loader_from_variables(variables), pair("forge", "47.2.0"));
        assert_eq!(
            loader_from_variables("MODLOADER=\"NeoForge\"\r\nMODLOADER_VERSION=\"21.1.77\"\r\n"),
            pair("neoforge", "21.1.77")
        );
        assert_eq!(loader_from_variables("MODLOADER=Forge\n"), None);
        assert_eq!(
            loader_from_variables("MODLOADER=Sponge\nMODLOADER_VERSION=1\n"),
            None
        );
        assert_eq!(loader_from_variables("MODLOADER_VERSION=47.2.0\n"), None);
    }

    #[test]
    fn finds_a_fabric_launcher_under_either_name() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(fabric_launcher_jar(dir.path()), None);
        std::fs::write(dir.path().join("fabric-api-0.92.jar"), b"jar").unwrap();
        assert_eq!(fabric_launcher_jar(dir.path()), None);

        let served = "fabric-server-mc.1.20.1-loader.0.15.11-launcher.1.0.1.jar";
        std::fs::write(dir.path().join(served), b"jar").unwrap();
        assert_eq!(fabric_launcher_jar(dir.path()).as_deref(), Some(served));
        assert_eq!(fabric_loader_version(served).as_deref(), Some("0.15.11"));
        assert_eq!(fabric_loader_version("fabric-server-launch.jar"), None);

        std::fs::write(dir.path().join("fabric-server-launch.jar"), b"jar").unwrap();
        assert_eq!(
            fabric_launcher_jar(dir.path()).as_deref(),
            Some("fabric-server-launch.jar")
        );
    }

    #[test]
    fn tells_a_bootstrap_server_jar_from_minecrafts_own() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!ships_own_launcher(dir.path()));
        for vanilla in [
            "net.minecraft.bundler.Main",
            "net.minecraft.server.Main",
            "net.minecraft.server.MinecraftServer",
        ] {
            jar(&dir.path().join("server.jar"), vanilla);
            assert!(!ships_own_launcher(dir.path()), "{vanilla}");
        }
        for bootstrap in [
            "net.fabricmc.installer.ServerLauncher",
            "net.minecraftforge.fml.relauncher.ServerLaunchWrapper",
            "atm.bloodworkxgaming.serverstarter.ServerStarterKt",
        ] {
            jar(&dir.path().join("server.jar"), bootstrap);
            assert!(ships_own_launcher(dir.path()), "{bootstrap}");
        }
        std::fs::write(dir.path().join("server.jar"), b"not a jar").unwrap();
        assert!(!ships_own_launcher(dir.path()));
    }

    #[test]
    fn counts_only_mod_jars_as_mods() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_mods(dir.path()));
        std::fs::create_dir_all(dir.path().join("mods/sub.jar")).unwrap();
        std::fs::write(dir.path().join("mods/readme.txt"), b"x").unwrap();
        assert!(!has_mods(dir.path()));
        std::fs::write(dir.path().join("mods/Create.JAR"), b"x").unwrap();
        assert!(has_mods(dir.path()));
    }

    #[test]
    fn reads_the_manifest_out_of_a_modpack_archive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file(
            "overrides/config/a.toml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"a").unwrap();
        zip.start_file("manifest.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(manifest(r#"[{"id":"forge-47.2.0","primary":true}]"#).as_bytes())
            .unwrap();
        zip.finish().unwrap();
        assert_eq!(
            loader_from_manifest(&client_manifest(&path).unwrap()),
            pair("forge", "47.2.0")
        );

        let empty = dir.path().join("empty.zip");
        zip::ZipWriter::new(std::fs::File::create(&empty).unwrap())
            .finish()
            .unwrap();
        assert!(client_manifest(&empty).is_err());
    }
}
