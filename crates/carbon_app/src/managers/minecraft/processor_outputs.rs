use std::collections::HashMap;
use std::path::{Path, PathBuf};

use daedalus::GradleSpecifier;
use daedalus::modded::{Processor, SidedDataEntry};

/// A file that a client-side install processor reads or writes. After a
/// successful install every one of these exists on disk, so a missing one
/// means the generated processor outputs were deleted and must be rebuilt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredFile {
    pub path: PathBuf,
    /// Known expected SHA-1 (lowercase hex), when the metadata declares one.
    /// Only consulted by deep-check validation, never on normal launches.
    pub expected_sha1: Option<String>,
}

pub fn required_files(
    processors: &[Processor],
    data: Option<&HashMap<String, SidedDataEntry>>,
    libraries_path: &Path,
) -> Vec<RequiredFile> {
    let mut out: Vec<RequiredFile> = Vec::new();

    let data_ref_path =
        |key: &str| -> Option<PathBuf> { resolve_ref(&data?.get(key)?.client, libraries_path) };
    let data_sha = |key: &str| -> Option<String> { strip_sha_literal(&data?.get(key)?.client) };

    for processor in processors.iter().filter(|p| runs_on_client(p)) {
        for arg in &processor.args {
            if is_maven_ref(arg) {
                if let Some(path) = resolve_ref(arg, libraries_path) {
                    upsert(&mut out, path, None);
                }
                continue;
            }
            for key in data_keys_in(arg) {
                if let Some(path) = data_ref_path(&key) {
                    upsert(&mut out, path, data_sha(&format!("{key}_SHA")));
                }
            }
        }

        if let Some(outputs) = &processor.outputs {
            for (out_key, out_val) in outputs {
                let declared_sha = if is_maven_ref(out_val) {
                    None
                } else {
                    strip_sha_literal(out_val)
                        .or_else(|| data_keys_in(out_val).into_iter().find_map(|k| data_sha(&k)))
                };

                if is_maven_ref(out_key) {
                    if let Some(path) = resolve_ref(out_key, libraries_path) {
                        upsert(&mut out, path, declared_sha.clone());
                    }
                } else {
                    for key in data_keys_in(out_key) {
                        if let Some(path) = data_ref_path(&key) {
                            let sha = declared_sha
                                .clone()
                                .or_else(|| data_sha(&format!("{key}_SHA")));
                            upsert(&mut out, path, sha);
                        }
                    }
                }
            }
        }
    }

    out
}

/// Mirrors the side filter in `execute_processors` exactly
/// (`managers/minecraft/forge.rs` / `neoforge.rs`): no `sides` means every side, otherwise the list
/// must contain "client".
fn runs_on_client(processor: &Processor) -> bool {
    match &processor.sides {
        None => true,
        Some(sides) => sides.iter().any(|s| s == "client"),
    }
}

fn is_maven_ref(s: &str) -> bool {
    s.starts_with('[') && s.ends_with(']')
}

fn resolve_ref(maven_ref: &str, libraries_path: &Path) -> Option<PathBuf> {
    let inner = maven_ref.strip_prefix('[')?.strip_suffix(']')?;
    let spec = inner.parse::<GradleSpecifier>().ok()?;
    Some(libraries_path.join(spec.into_path()))
}

/// Finds `{KEY}` tokens, including ones embedded in longer strings such as
/// `{ROOT}/libraries/`.
fn data_keys_in(s: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start + 1..].find('}') else {
            break;
        };
        keys.push(rest[start + 1..start + 1 + len].to_string());
        rest = &rest[start + 1 + len + 1..];
    }
    keys
}

/// SHA data entries are single-quoted 40-char hex literals, e.g.
/// `'de86b035d2da0f78940796bb95c39a932ed84834'`.
fn strip_sha_literal(s: &str) -> Option<String> {
    let trimmed = s.trim_matches('\'');
    (trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| trimmed.to_ascii_lowercase())
}

fn upsert(out: &mut Vec<RequiredFile>, path: PathBuf, expected_sha1: Option<String>) {
    if let Some(existing) = out.iter_mut().find(|f| f.path == path) {
        if existing.expected_sha1.is_none() {
            existing.expected_sha1 = expected_sha1;
        }
    } else {
        out.push(RequiredFile {
            path,
            expected_sha1,
        });
    }
}

/// Returns the required files that are not on disk. With `verify_hashes`
/// (deep-check / repair only), files whose known SHA-1 does not match are
/// deleted and reported as missing so the caller regenerates them. Normal
/// launches must pass `false`: generated jars can legitimately hash
/// differently across environments (e.g. zlib-ng native compression), so
/// hash mismatches are only acted on in the explicit repair flow.
pub async fn missing_files(required: &[RequiredFile], verify_hashes: bool) -> Vec<PathBuf> {
    let owned = required.to_vec();
    let handle = tokio::task::spawn_blocking(move || {
        let mut missing = Vec::new();
        for file in &owned {
            if !file.path.is_file() {
                missing.push(file.path.clone());
                continue;
            }
            if verify_hashes {
                if let Some(expected) = &file.expected_sha1 {
                    match hash_file_sha1(&file.path) {
                        Ok(actual) if actual.eq_ignore_ascii_case(expected) => {}
                        Ok(actual) => {
                            tracing::info!(
                                "Deleting processor output with unexpected hash \
                                 (expected {expected}, got {actual}): {:?}",
                                file.path
                            );
                            if let Err(remove_err) = std::fs::remove_file(&file.path) {
                                tracing::warn!(
                                    "Failed to delete hash-mismatched processor output {:?}: {remove_err}",
                                    file.path
                                );
                            }
                            missing.push(file.path.clone());
                        }
                        Err(e) => {
                            tracing::warn!("Failed to hash {:?}: {e}", file.path);
                            missing.push(file.path.clone());
                        }
                    }
                }
            }
        }
        missing
    });

    match handle.await {
        Ok(missing) => missing,
        Err(e) => {
            tracing::error!(
                "Processor output validation task failed ({e}); treating all {} required file(s) as missing so regeneration runs",
                required.len()
            );
            required.iter().map(|f| f.path.clone()).collect()
        }
    }
}

fn hash_file_sha1(path: &Path) -> std::io::Result<String> {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed from the live meta.gdl.gg payload for forge 1.20.1-47.2.0
    // (classpath arrays emptied; irrelevant to resolution). Covers: a
    // server-only processor (BUNDLER_EXTRACT -> MC_UNPACKED must be
    // excluded), a client processor with declared outputs+SHAs
    // (jarsplitter), a sides-less processor without SHA (FART), and the
    // K/K_SHA data pairing without declared outputs (binarypatcher ->
    // PATCHED/PATCHED_SHA).
    const FORGE_1_20_1_PROCESSORS: &str = r#"[
        {"jar":"net.minecraftforge:installertools:1.3.0","classpath":[],
         "args":["--task","BUNDLER_EXTRACT","--input","{MINECRAFT_JAR}","--output","{MC_UNPACKED}","--jar-only"],
         "sides":["server"]},
        {"jar":"net.minecraftforge:jarsplitter:1.1.4","classpath":[],
         "args":["--input","{MINECRAFT_JAR}","--slim","{MC_SLIM}","--extra","{MC_EXTRA}","--srg","{MERGED_MAPPINGS}"],
         "outputs":{"{MC_SLIM}":"{MC_SLIM_SHA}","{MC_EXTRA}":"{MC_EXTRA_SHA}"},
         "sides":["client"]},
        {"jar":"net.minecraftforge:ForgeAutoRenamingTool:0.1.22:all","classpath":[],
         "args":["--input","{MC_SLIM}","--output","{MC_SRG}","--names","{MERGED_MAPPINGS}","--ann-fix","--ids-fix","--src-fix","--record-fix"]},
        {"jar":"net.minecraftforge:binarypatcher:1.1.1","classpath":[],
         "args":["--clean","{MC_SRG}","--output","{PATCHED}","--apply","{BINPATCH}"]}
    ]"#;

    const FORGE_1_20_1_DATA: &str = r#"{
        "MC_SLIM":{"client":"[net.minecraft:client:1.20.1-20230612.114412:slim]","server":"[net.minecraft:server:1.20.1-20230612.114412:slim]"},
        "MC_SLIM_SHA":{"client":"'de86b035d2da0f78940796bb95c39a932ed84834'","server":"'9e06bdd77ca6d95b2cced0bf372245f753eeb16a'"},
        "MC_EXTRA":{"client":"[net.minecraft:client:1.20.1-20230612.114412:extra]","server":"[net.minecraft:server:1.20.1-20230612.114412:extra]"},
        "MC_EXTRA_SHA":{"client":"'8c5a95cbce940cfdb304376ae9fea47968d02587'","server":"'13522e3278befd103064d91a199451df4cd2633f'"},
        "MC_SRG":{"client":"[net.minecraft:client:1.20.1-20230612.114412:srg]","server":"[net.minecraft:server:1.20.1-20230612.114412:srg]"},
        "PATCHED":{"client":"[net.minecraftforge:forge:1.20.1-47.2.0:client]","server":"[net.minecraftforge:forge:1.20.1-47.2.0:server]"},
        "PATCHED_SHA":{"client":"'3e175b011146785588f1649a20d1834d10282a7c'","server":"'597ed4e82a3e309572f7ceafa2408571ba7e6c43'"},
        "BINPATCH":{"client":"[net.minecraftforge:forge:1.20.1-forge-47.2.0:client@lzma]","server":"[net.minecraftforge:forge:1.20.1-forge-47.2.0:server@lzma]"},
        "MC_UNPACKED":{"client":"[net.minecraft:client:1.20.1-20230612.114412:unpacked]","server":"[net.minecraft:server:1.20.1-20230612.114412:unpacked]"},
        "MERGED_MAPPINGS":{"client":"[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings-merged@txt]","server":"[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings-merged@txt]"}
    }"#;

    // Trimmed from the live meta.gdl.gg payload for neoforge 26.2.0.23-beta:
    // modern NeoForge declares no outputs and no SHAs at all; the only
    // client processor references {PATCHED}/{BINPATCH} plus augmented keys.
    const NEO_26_PROCESSORS: &str = r#"[
        {"jar":"net.neoforged.installertools:installertools:4.0.12:fatjar","classpath":[],
         "args":["--task","EXTRACT_FILES","--archive","{INSTALLER}","--from","data/run.sh","--to","{ROOT}/run.sh"],
         "sides":["server"]},
        {"jar":"net.neoforged.installertools:installertools:4.0.12:fatjar","classpath":[],
         "args":["--task","PROCESS_MINECRAFT_JAR","--no-mod-manifest","--input","{MINECRAFT_JAR}","--output","{PATCHED}","--extract-libraries-to","{ROOT}/libraries/","--apply-patches","{BINPATCH}"]}
    ]"#;

    const NEO_26_DATA: &str = r#"{
        "PATCHED":{"client":"[net.neoforged:minecraft-client-patched:26.2.0.23-beta]","server":"[net.neoforged:minecraft-server-patched:26.2.0.23-beta]"},
        "BINPATCH":{"client":"[net.minecraftforge:forge:neoforge-26.2.0.23-beta:client@lzma]","server":"[net.minecraftforge:forge:neoforge-26.2.0.23-beta:client@lzma]"}
    }"#;

    fn parse(processors: &str, data: &str) -> (Vec<Processor>, HashMap<String, SidedDataEntry>) {
        (
            serde_json::from_str(processors).unwrap(),
            serde_json::from_str(data).unwrap(),
        )
    }

    fn find<'a>(required: &'a [RequiredFile], suffix: &str) -> &'a RequiredFile {
        required
            .iter()
            .find(|f| f.path.ends_with(suffix))
            .unwrap_or_else(|| panic!("no required file ending in {suffix}: {required:?}"))
    }

    #[test]
    fn forge_1_20_1_required_set_is_exactly_the_six_client_files() {
        let (procs, data) = parse(FORGE_1_20_1_PROCESSORS, FORGE_1_20_1_DATA);
        let required = required_files(&procs, Some(&data), Path::new("/libs"));

        assert_eq!(required.len(), 6, "{required:?}");
        find(
            &required,
            "net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-slim.jar",
        );
        find(
            &required,
            "net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-extra.jar",
        );
        find(
            &required,
            "net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-srg.jar",
        );
        find(
            &required,
            "de/oceanlabs/mcp/mcp_config/1.20.1-20230612.114412/mcp_config-1.20.1-20230612.114412-mappings-merged.txt",
        );
        find(
            &required,
            "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-client.jar",
        );
        find(
            &required,
            "net/minecraftforge/forge/1.20.1-forge-47.2.0/forge-1.20.1-forge-47.2.0-client.lzma",
        );
    }

    #[test]
    fn server_only_data_refs_are_excluded() {
        let (procs, data) = parse(FORGE_1_20_1_PROCESSORS, FORGE_1_20_1_DATA);
        let required = required_files(&procs, Some(&data), Path::new("/libs"));
        assert!(
            !required
                .iter()
                .any(|f| f.path.to_string_lossy().contains("unpacked")),
            "MC_UNPACKED is only referenced by a server-sided processor: {required:?}"
        );
    }

    #[test]
    fn sha_extraction_from_declared_outputs_and_key_sha_pairing() {
        let (procs, data) = parse(FORGE_1_20_1_PROCESSORS, FORGE_1_20_1_DATA);
        let required = required_files(&procs, Some(&data), Path::new("/libs"));

        let slim = find(&required, "client-1.20.1-20230612.114412-slim.jar");
        assert_eq!(
            slim.expected_sha1.as_deref(),
            Some("de86b035d2da0f78940796bb95c39a932ed84834")
        );
        let patched = find(&required, "forge-1.20.1-47.2.0-client.jar");
        assert_eq!(
            patched.expected_sha1.as_deref(),
            Some("3e175b011146785588f1649a20d1834d10282a7c"),
            "PATCHED_SHA comes from the K/K_SHA data pairing, not declared outputs"
        );
        let srg = find(&required, "client-1.20.1-20230612.114412-srg.jar");
        assert_eq!(srg.expected_sha1, None);
        let lzma = find(&required, "forge-1.20.1-forge-47.2.0-client.lzma");
        assert_eq!(lzma.expected_sha1, None);
    }

    #[test]
    fn neoforge_26_required_set_without_outputs_or_shas() {
        let (procs, data) = parse(NEO_26_PROCESSORS, NEO_26_DATA);
        let required = required_files(&procs, Some(&data), Path::new("/libs"));

        assert_eq!(required.len(), 2, "{required:?}");
        let patched = find(
            &required,
            "net/neoforged/minecraft-client-patched/26.2.0.23-beta/minecraft-client-patched-26.2.0.23-beta.jar",
        );
        assert_eq!(patched.expected_sha1, None);
        find(
            &required,
            "net/minecraftforge/forge/neoforge-26.2.0.23-beta/forge-neoforge-26.2.0.23-beta-client.lzma",
        );
    }

    #[test]
    fn augmented_and_unknown_keys_are_skipped() {
        let procs: Vec<Processor> = serde_json::from_str(
            r#"[{"jar":"a:b:1","classpath":[],
                 "args":["{MINECRAFT_JAR}","{ROOT}/libraries/","{SIDE}","{NOT_IN_DATA}","plain-arg"]}]"#,
        )
        .unwrap();
        let data: HashMap<String, SidedDataEntry> = HashMap::new();
        assert!(required_files(&procs, Some(&data), Path::new("/libs")).is_empty());
    }

    #[test]
    fn direct_maven_ref_args_resolve_without_data() {
        let procs: Vec<Processor> = serde_json::from_str(
            r#"[{"jar":"a:b:1","classpath":[],
                 "args":["[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412@zip]",
                         "[net.minecraftforge:forge:neoforge-26.1.0.0-alpha.1+snapshot-1:client@lzma]"]}]"#,
        )
        .unwrap();
        let required = required_files(&procs, None, Path::new("/libs"));
        assert_eq!(required.len(), 2);
        find(
            &required,
            "de/oceanlabs/mcp/mcp_config/1.20.1-20230612.114412/mcp_config-1.20.1-20230612.114412.zip",
        );
        find(
            &required,
            "net/minecraftforge/forge/neoforge-26.1.0.0-alpha.1+snapshot-1/forge-neoforge-26.1.0.0-alpha.1+snapshot-1-client.lzma",
        );
        assert_eq!(required[0].expected_sha1, None);
    }

    #[test]
    fn empty_processors_yield_empty_set() {
        let (_, data) = parse(FORGE_1_20_1_PROCESSORS, FORGE_1_20_1_DATA);
        assert!(required_files(&[], Some(&data), Path::new("/libs")).is_empty());
    }

    fn write_file(dir: &Path, rel: &str, contents: &[u8]) -> PathBuf {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[tokio::test]
    async fn missing_files_reports_absent_and_keeps_present() {
        let dir = tempfile::tempdir().unwrap();
        let present = write_file(dir.path(), "a/b/1/b-1.jar", b"hello");
        let absent = dir.path().join("a/c/1/c-1.jar");

        let required = vec![
            RequiredFile {
                path: present.clone(),
                expected_sha1: None,
            },
            RequiredFile {
                path: absent.clone(),
                expected_sha1: None,
            },
        ];

        let missing = missing_files(&required, false).await;
        assert_eq!(missing, vec![absent]);
        assert!(present.is_file());
    }

    #[tokio::test]
    async fn hash_mismatch_ignored_on_normal_launch() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a/b/1/b-1.jar", b"wrong contents");
        let required = vec![RequiredFile {
            path: path.clone(),
            // SHA-1 of "hello"
            expected_sha1: Some("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d".into()),
        }];

        let missing = missing_files(&required, false).await;
        assert!(missing.is_empty());
        assert!(path.is_file(), "normal launches never delete files");
    }

    #[tokio::test]
    async fn deep_check_deletes_mismatched_and_reports_missing() {
        let dir = tempfile::tempdir().unwrap();
        let bad = write_file(dir.path(), "a/b/1/b-1.jar", b"wrong contents");
        let good = write_file(dir.path(), "a/g/1/g-1.jar", b"hello");
        let unhashed = write_file(dir.path(), "a/u/1/u-1.jar", b"anything");

        let required = vec![
            RequiredFile {
                path: bad.clone(),
                expected_sha1: Some("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d".into()),
            },
            RequiredFile {
                path: good.clone(),
                expected_sha1: Some("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d".into()),
            },
            RequiredFile {
                path: unhashed.clone(),
                expected_sha1: None,
            },
        ];

        let missing = missing_files(&required, true).await;
        assert_eq!(missing, vec![bad.clone()]);
        assert!(
            !bad.exists(),
            "mismatched file is deleted so regeneration replaces it"
        );
        assert!(good.is_file());
        assert!(
            unhashed.is_file(),
            "files without a known SHA are existence-checked only"
        );
    }
}
