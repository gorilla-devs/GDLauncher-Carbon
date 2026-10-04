use crate::{cache_middleware, iridium_client::get_client};

use super::{ManagerRef, UnsafeAppRef};

pub mod curseforge;
pub mod modrinth;

pub struct ModplatformsManager {
    pub curseforge: curseforge::CurseForge,
    pub modrinth: modrinth::Modrinth,
}

impl ModplatformsManager {
    pub fn new(unsafeappref: UnsafeAppRef, gdl_base_api: String) -> Self {
        Self {
            curseforge: curseforge::CurseForge::new(cache_middleware::new_client(
                unsafeappref.clone(),
                get_client(gdl_base_api.clone()),
            )),
            modrinth: modrinth::Modrinth::new(cache_middleware::new_client(
                unsafeappref,
                get_client(gdl_base_api),
            )),
        }
    }
}

impl ManagerRef<'_, ModplatformsManager> {}

/// Picks the manifest entry a pack or an import pins, and returns it as the
/// manifest has it. An entry with the same version wins. Failing that, for
/// Forge only, the one entry with the same build number is taken: GDL's
/// metadata gives some old builds a branch name after the build
/// (`1.7.10-10.13.3.1401-1710ls`, `1.10.2-12.18.0.2007-1.10.0`), while packs
/// and GDL's own CurseForge export name only the build (`10.13.3.1401`).
/// Builds are compared whole, so `47.2.1` never picks `47.2.10`.
fn select_pinned_loader<T>(
    candidates: Vec<T>,
    id: impl Fn(&T) -> &str,
    pin: &str,
    minecraft: &str,
    loader: &str,
) -> Option<T> {
    let index = candidates
        .iter()
        .position(|candidate| pinned_loader_matches(id(candidate), pin, minecraft, loader))
        .or_else(|| {
            if loader != "forge" {
                return None;
            }
            let build = forge_build(pin, minecraft)?;
            let mut same_build = candidates
                .iter()
                .enumerate()
                .filter(|(_, candidate)| forge_build(id(candidate), minecraft) == Some(build))
                .map(|(index, _)| index);
            // Two entries with one build number would be a guess; refuse it.
            match (same_build.next(), same_build.next()) {
                (Some(index), None) => Some(index),
                _ => None,
            }
        })?;
    candidates.into_iter().nth(index)
}

fn pinned_loader_matches(candidate: &str, pin: &str, minecraft: &str, loader: &str) -> bool {
    !pin.is_empty()
        && logical_loader_version(candidate, minecraft, loader)
            == logical_loader_version(pin, minecraft, loader)
}

/// The build number of a Forge version: its logical form up to the first `-`.
fn forge_build<'a>(value: &'a str, minecraft: &str) -> Option<&'a str> {
    logical_loader_version(value, minecraft, "forge")
        .split('-')
        .next()
        .filter(|build| !build.is_empty())
}

fn logical_loader_version<'a>(value: &'a str, minecraft: &str, loader: &str) -> &'a str {
    let value = value.strip_prefix(&format!("{loader}-")).unwrap_or(value);
    // GDL's metadata names Forge builds, and NeoForge's 1.20.1 builds,
    // `<minecraft>-<build>`; other launchers and packwiz often name the build alone.
    let value = value
        .strip_prefix(&format!("{minecraft}-"))
        .unwrap_or(value);
    if loader != "forge" {
        return value;
    }
    let value = value
        .strip_prefix("forge-")
        .or_else(|| value.strip_prefix("Forge"))
        .unwrap_or(value);
    value
        .strip_suffix(&format!("-{minecraft}"))
        .unwrap_or(value)
}

#[cfg(test)]
mod task11_tests {
    fn select<'a>(candidates: &[&'a str], pin: &str, mc: &str, loader: &str) -> Option<&'a str> {
        super::select_pinned_loader(candidates.to_vec(), |id| *id, pin, mc, loader)
    }

    #[test]
    fn task11_loader_pins_do_not_match_neighbouring_versions() {
        for (loader, mc, pin, candidates, expected) in [
            (
                "fabric",
                "1.20.1",
                "0.16.1",
                ["0.16.14", "0.16.1"],
                "0.16.1",
            ),
            ("quilt", "1.20.1", "0.26.1", ["0.26.10", "0.26.1"], "0.26.1"),
            (
                "neoforge",
                "1.21.1",
                "21.1.1",
                ["21.1.10", "21.1.1"],
                "21.1.1",
            ),
            (
                "forge",
                "1.20.1",
                "47.2.1",
                ["1.20.1-47.2.10", "1.20.1-47.2.1"],
                "1.20.1-47.2.1",
            ),
            (
                "neoforge",
                "1.20.1",
                "47.1.7",
                ["1.20.1-47.1.79", "1.20.1-47.1.7"],
                "1.20.1-47.1.7",
            ),
        ] {
            assert_eq!(
                select(&candidates, pin, mc, loader),
                Some(expected),
                "{loader} {pin}"
            );
        }
    }

    // GDL's metadata names every NeoForge 1.20.1 build `1.20.1-47.1.x`.
    // Prism, ATLauncher, the Modrinth App, the FTB App and packwiz name the
    // build alone; the CurseForge app writes `neoforge-1.20.1-47.1.x`.
    #[test]
    fn task11_neoforge_1_20_1_pins_resolve_with_or_without_the_minecraft_prefix() {
        let meta = ["1.20.1-47.1.106", "1.20.1-47.1.790", "1.20.1-47.1.79"];
        for pin in ["47.1.79", "1.20.1-47.1.79", "neoforge-47.1.79"] {
            assert_eq!(
                select(&meta, pin, "1.20.1", "neoforge"),
                Some("1.20.1-47.1.79"),
                "{pin}"
            );
        }
        assert_eq!(
            select(&meta, "47.1.106", "1.20.1", "neoforge"),
            Some("1.20.1-47.1.106")
        );
        for pin in ["47.1.7", "47.1", "1.20.2-47.1.79", ""] {
            assert_eq!(select(&meta, pin, "1.20.1", "neoforge"), None, "{pin}");
        }
        // Later NeoForge lines name builds without the Minecraft version.
        assert_eq!(
            select(
                &["20.2.88", "20.2.8-beta"],
                "20.2.8-beta",
                "1.20.2",
                "neoforge"
            ),
            Some("20.2.8-beta")
        );
    }

    // Ids as GDL's Forge metadata lists them; packs and GDL's own CurseForge
    // export name these builds without the branch suffix.
    #[test]
    fn task11_old_forge_builds_resolve_without_their_branch_suffix() {
        for (mc, pin, candidates, expected) in [
            (
                "1.7.10",
                "10.13.3.1401",
                [
                    "1.7.10-10.13.3.1403-1.7.10",
                    "1.7.10-10.13.3.1401-1710ls",
                    "1.7.10-10.13.3.1400-1.7.10",
                ],
                "1.7.10-10.13.3.1401-1710ls",
            ),
            (
                "1.7.10",
                "10.13.1.1216",
                [
                    "1.7.10-10.13.1.1217-new",
                    "1.7.10-10.13.1.1216-new",
                    "1.7.10-10.13.1.1215-new",
                ],
                "1.7.10-10.13.1.1216-new",
            ),
            (
                "1.7.2",
                "10.12.2.1161",
                [
                    "1.7.2-10.12.2.1161-mc172",
                    "1.7.2-10.12.2.1155-mc172",
                    "1.7.2-10.12.2.1147",
                ],
                "1.7.2-10.12.2.1161-mc172",
            ),
            (
                "1.10.2",
                "12.18.0.2007",
                [
                    "1.10.2-12.18.0.2008",
                    "1.10.2-12.18.0.2007-1.10.0",
                    "1.10.2-12.18.0.2006-1.10.0",
                ],
                "1.10.2-12.18.0.2007-1.10.0",
            ),
            (
                "1.11",
                "13.19.0.2129",
                [
                    "1.11-13.19.0.2130",
                    "1.11-13.19.0.2129-1.11.x",
                    "1.11-13.19.0.2128-1.11.x",
                ],
                "1.11-13.19.0.2129-1.11.x",
            ),
            (
                "1.12.2",
                "14.23.4.2720",
                [
                    "1.12.2-14.23.4.2721",
                    "1.12.2-14.23.4.2720-4627",
                    "1.12.2-14.23.4.2719",
                ],
                "1.12.2-14.23.4.2720-4627",
            ),
            // The full id, and the build with its suffix, still match exactly.
            (
                "1.7.10",
                "1.7.10-10.13.3.1401-1710ls",
                [
                    "1.7.10-10.13.3.1403-1.7.10",
                    "1.7.10-10.13.3.1401-1710ls",
                    "1.7.10-10.13.3.1400-1.7.10",
                ],
                "1.7.10-10.13.3.1401-1710ls",
            ),
            (
                "1.7.10",
                "10.13.3.1401-1710ls",
                [
                    "1.7.10-10.13.3.1403-1.7.10",
                    "1.7.10-10.13.3.1401-1710ls",
                    "1.7.10-10.13.3.1400-1.7.10",
                ],
                "1.7.10-10.13.3.1401-1710ls",
            ),
        ] {
            assert_eq!(
                select(&candidates, pin, mc, "forge"),
                Some(expected),
                "{mc} {pin}"
            );
        }
        // A build is compared whole: a shorter build never picks a longer one.
        for pin in ["10.13.3.140", "10.13.3", "1710ls", "1.7.10"] {
            assert_eq!(
                select(
                    &["1.7.10-10.13.3.1401-1710ls", "1.7.10-10.13.3.1400-1.7.10"],
                    pin,
                    "1.7.10",
                    "forge"
                ),
                None,
                "{pin}"
            );
        }
    }

    #[test]
    fn task11_exact_loader_matches_win_and_shared_builds_are_refused() {
        // The exact id wins even when a suffixed entry of the same build comes first.
        assert_eq!(
            select(
                &["1.20.1-47.2.1-beta", "1.20.1-47.2.1"],
                "47.2.1",
                "1.20.1",
                "forge"
            ),
            Some("1.20.1-47.2.1")
        );
        // Two suffixed entries with one build number: refuse rather than guess.
        assert_eq!(
            select(
                &["1.20.1-47.2.1-a", "1.20.1-47.2.1-b"],
                "47.2.1",
                "1.20.1",
                "forge"
            ),
            None
        );
        for candidates in [
            ["1.20.1-47.2.10-a", "1.20.1-47.2.11"],
            ["1.20.2-47.2.1-a", "1.20.2-47.2.1"],
        ] {
            assert_eq!(select(&candidates, "47.2.1", "1.20.1", "forge"), None);
        }
        // Only Forge falls back to the build number: other loaders' suffixes
        // name different releases.
        assert_eq!(
            select(&["0.17.0-beta.1"], "0.17.0", "1.20.1", "quilt"),
            None
        );
    }

    // GDL's CurseForge export names Forge by its build and NeoForge by its full
    // id; importing that export again has to land on the same entries.
    #[test]
    fn task11_gdl_curseforge_export_resolves_back_to_the_same_loader() {
        use crate::domain::instance::info::{ModLoader, ModLoaderType, StandardVersion};

        for (mc, type_, id, meta) in [
            (
                "1.7.10",
                ModLoaderType::Forge,
                "1.7.10-10.13.3.1401-1710ls",
                ["1.7.10-10.13.3.1403-1.7.10", "1.7.10-10.13.3.1401-1710ls"],
            ),
            (
                "1.20.1",
                ModLoaderType::Forge,
                "1.20.1-47.2.1",
                ["1.20.1-47.2.10", "1.20.1-47.2.1"],
            ),
            (
                "1.20.1",
                ModLoaderType::Neoforge,
                "1.20.1-47.1.79",
                ["1.20.1-47.1.790", "1.20.1-47.1.79"],
            ),
        ] {
            let exported =
                super::curseforge::convert_standard_version_to_cf_version(StandardVersion {
                    release: mc.to_string(),
                    modloaders: [ModLoader {
                        type_,
                        version: id.to_string(),
                    }]
                    .into_iter()
                    .collect(),
                })
                .unwrap();
            let (loader, version) = exported.mod_loaders[0].id.split_once('-').unwrap();
            assert_eq!(
                select(&meta, version, &exported.version, loader),
                Some(id),
                "{}",
                exported.mod_loaders[0].id
            );
        }
    }

    #[test]
    fn task11_loader_pins_preserve_full_legacy_and_absent_controls() {
        for candidate in [
            "1.20.1-47.2.1",
            "forge-47.2.1",
            "Forge47.2.1",
            "47.2.1-1.20.1",
            "forge-1.20.1-47.2.1",
        ] {
            assert!(
                super::pinned_loader_matches(candidate, "47.2.1", "1.20.1", "forge"),
                "{candidate}"
            );
        }
        for candidate in ["1.20.2-47.2.1", "47.2.10", "47.2.1-beta", ""] {
            assert!(!super::pinned_loader_matches(
                candidate, "47.2.1", "1.20.1", "forge"
            ));
        }
        assert!(!super::pinned_loader_matches(
            "47.2.1", "", "1.20.1", "forge"
        ));
        assert!(super::pinned_loader_matches(
            "fabric-0.16.1",
            "0.16.1",
            "1.20.1",
            "fabric"
        ));
    }
}
