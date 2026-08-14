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

/// Whether an existing output looks complete rather than half-written.
///
/// Processors stream their target out of a JVM subprocess, so an interruption
/// leaves a zero-byte or truncated file at exactly the path an existence check
/// accepts. Hashes cannot stand in for this on a normal launch — generated jars
/// legitimately hash differently across environments — so the test is confined
/// to what is unambiguous: a file with no bytes, or an archive with no
/// end-of-central-directory record, cannot be a finished output.
fn is_structurally_complete(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if metadata.len() == 0 {
        return false;
    }

    let is_archive = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("jar") || e.eq_ignore_ascii_case("zip"));
    if !is_archive {
        return true;
    }

    has_zip_end_of_central_directory(path, metadata.len())
}

/// Scans the tail of `path` for the ZIP end-of-central-directory signature,
/// which a complete archive always carries within its last 64KiB (22 bytes of
/// record plus a comment of at most `u16::MAX`).
fn has_zip_end_of_central_directory(path: &Path, len: u64) -> bool {
    use std::io::{Read, Seek, SeekFrom};

    const EOCD_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    const MAX_EOCD_SPAN: u64 = 22 + u16::MAX as u64;

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let span = len.min(MAX_EOCD_SPAN);
    if file.seek(SeekFrom::End(-(span as i64))).is_err() {
        return false;
    }
    let mut tail = Vec::with_capacity(span as usize);
    if file.take(span).read_to_end(&mut tail).is_err() {
        return false;
    }
    tail.windows(EOCD_SIGNATURE.len())
        .any(|w| w == EOCD_SIGNATURE)
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
            if !is_structurally_complete(&file.path) {
                tracing::info!(
                    "Processor output is empty or truncated, regenerating: {:?}",
                    file.path
                );
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
    use serde::{Deserialize, Serialize};

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

    /// End-of-central-directory record of an empty archive.
    const EOCD: &[u8] =
        b"PK\x05\x06\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

    /// A structurally complete archive whose leading bytes callers vary to
    /// control its hash.
    fn write_jar(dir: &Path, rel: &str, body: &[u8]) -> PathBuf {
        let mut contents = body.to_vec();
        contents.extend_from_slice(EOCD);
        write_file(dir, rel, &contents)
    }

    #[tokio::test]
    async fn missing_files_reports_absent_and_keeps_present() {
        let dir = tempfile::tempdir().unwrap();
        let present = write_jar(dir.path(), "a/b/1/b-1.jar", b"hello");
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
    async fn empty_output_counts_as_missing_on_a_normal_launch() {
        // A processor writes its target incrementally; an interruption after the
        // file is created leaves a zero-byte jar at the path existence alone
        // accepts, and the launch then fails with a missing-dependency error
        // that never regenerates it.
        let dir = tempfile::tempdir().unwrap();
        let empty = write_file(dir.path(), "a/b/1/b-1.jar", b"");
        let required = vec![RequiredFile {
            path: empty.clone(),
            expected_sha1: None,
        }];

        assert_eq!(missing_files(&required, false).await, vec![empty]);
    }

    #[tokio::test]
    async fn truncated_jar_counts_as_missing_on_a_normal_launch() {
        // Non-empty but cut short: no end-of-central-directory record, so the
        // JVM cannot open it as an archive.
        let dir = tempfile::tempdir().unwrap();
        let truncated = write_file(dir.path(), "a/b/1/b-1.jar", b"PK\x03\x04 partial...");
        let required = vec![RequiredFile {
            path: truncated.clone(),
            expected_sha1: None,
        }];

        assert_eq!(missing_files(&required, false).await, vec![truncated]);
    }

    #[tokio::test]
    async fn intact_jar_is_kept_on_a_normal_launch() {
        // A minimal but valid archive: empty central directory + EOCD record.
        let dir = tempfile::tempdir().unwrap();
        let intact = write_jar(dir.path(), "a/b/1/b-1.jar", b"");
        let required = vec![RequiredFile {
            path: intact,
            expected_sha1: None,
        }];

        assert!(missing_files(&required, false).await.is_empty());
    }

    #[tokio::test]
    async fn non_jar_output_is_only_checked_for_emptiness() {
        // Processors also emit plain files; only archives get the EOCD check.
        let dir = tempfile::tempdir().unwrap();
        let txt = write_file(dir.path(), "a/b/1/b-1.txt", b"not an archive");
        let required = vec![RequiredFile {
            path: txt,
            expected_sha1: None,
        }];

        assert!(missing_files(&required, false).await.is_empty());
    }

    #[tokio::test]
    async fn hash_mismatch_ignored_on_normal_launch() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jar(dir.path(), "a/b/1/b-1.jar", b"wrong contents");
        let required = vec![RequiredFile {
            path: path.clone(),
            // SHA-1 of a different archive
            expected_sha1: Some("fe0d5cde59fd57282571f339e0a9aedd85dbfb54".into()),
        }];

        let missing = missing_files(&required, false).await;
        assert!(missing.is_empty());
        assert!(path.is_file(), "normal launches never delete files");
    }

    #[tokio::test]
    async fn deep_check_deletes_mismatched_and_reports_missing() {
        let dir = tempfile::tempdir().unwrap();
        let bad = write_jar(dir.path(), "a/b/1/b-1.jar", b"wrong contents");
        let good = write_jar(dir.path(), "a/g/1/g-1.jar", b"hello");
        let unhashed = write_jar(dir.path(), "a/u/1/u-1.jar", b"anything");

        let required = vec![
            RequiredFile {
                path: bad.clone(),
                // SHA-1 of `good`, so `bad` mismatches.
                expected_sha1: Some("fe0d5cde59fd57282571f339e0a9aedd85dbfb54".into()),
            },
            RequiredFile {
                path: good.clone(),
                expected_sha1: Some("fe0d5cde59fd57282571f339e0a9aedd85dbfb54".into()),
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

    // -----------------------------------------------------------------
    // Golden cross-check against the TS port
    // -----------------------------------------------------------------
    //
    // `required_files` is load-bearing production code (called from
    // `managers/instance/run/minecraft.rs` to decide whether Forge/NeoForge
    // processors need to re-run at launch), and the e2e suite
    // (`apps/desktop/e2e-tests/loaderInstall.spec.ts`) carries an
    // independent TypeScript port of it
    // (`apps/desktop/e2e-tests/helpers/processorOutputs.ts`) so the
    // processor-artifact assertion can run without a Rust binding into the
    // Playwright process. Two independent implementations of the same
    // logic drift silently unless something forces them to agree: this
    // test computes `required_files`'s real output for a fixed, committed
    // input fixture and compares it byte-for-byte against a committed
    // golden output file; `processorOutputs.test.ts` reads the exact same
    // two files (`../../../../crates/carbon_app/fixtures/processor_outputs_golden/`
    // from its own location) and asserts its port produces the same
    // (order-normalized) result. A behavior change here either breaks this
    // test (if the golden wasn't regenerated) or, once the golden is
    // deliberately regenerated to reflect an intended change, breaks the TS
    // test until that port is updated to match — either way a human is
    // told, rather than the two implementations quietly disagreeing while
    // each individually keeps passing its own tests.
    //
    // `outputs` on `Processor` is a `HashMap`, so multiple entries in one
    // processor's `outputs` map can be visited in different orders across
    // runs — this normalizes that away by sorting on `relative_path` before
    // serializing, on both sides, so the comparison is over the *set* of
    // required files, not incidental Rust HashMap iteration order.

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct GoldenCase {
        name: String,
        processors: Vec<Processor>,
        data: HashMap<String, SidedDataEntry>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct GoldenRequiredFile {
        relative_path: String,
        expected_sha1: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct GoldenOutputCase {
        name: String,
        required: Vec<GoldenRequiredFile>,
    }

    fn golden_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/processor_outputs_golden")
    }

    /// Converts `required_files`'s output to the portable golden shape:
    /// forward-slash paths (Rust's own `PathBuf` join uses `\` on Windows,
    /// which would make the committed golden file platform-dependent
    /// otherwise — the TS port always produces `/`-joined paths when run on
    /// Linux/macOS CI, and this keeps the comparison meaningful regardless
    /// of which OS generated or reads the golden), sorted by path for the
    /// HashMap-ordering reason above.
    fn to_golden(required: &[RequiredFile]) -> Vec<GoldenRequiredFile> {
        let mut out: Vec<GoldenRequiredFile> = required
            .iter()
            .map(|f| GoldenRequiredFile {
                relative_path: f.path.to_string_lossy().replace('\\', "/"),
                expected_sha1: f.expected_sha1.clone(),
            })
            .collect();
        out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        out
    }

    /// Computes `required_files` for every case in the committed input
    /// fixture, in the same golden shape the committed output file stores.
    fn compute_golden_output(cases: &[GoldenCase]) -> Vec<GoldenOutputCase> {
        cases
            .iter()
            .map(|case| {
                // `Path::new("")` mirrors `requiredLibraryPaths`'s TS side
                // (which never joins a libraries root at all): joining onto
                // an empty base leaves `RequiredFile::path` exactly the
                // relative path, so nothing here needs to strip a prefix
                // back off before comparing.
                let required = required_files(&case.processors, Some(&case.data), Path::new(""));
                GoldenOutputCase {
                    name: case.name.clone(),
                    required: to_golden(&required),
                }
            })
            .collect()
    }

    /// Compares `required_files`'s real output against a committed golden
    /// file (see the module-level comment above this test for why this
    /// exists). Run with `UPDATE_GOLDEN_PROCESSOR_OUTPUTS=1` to regenerate
    /// the golden after a deliberate behavior change — review the diff like
    /// any other source change, and update `processorOutputs.test.ts`'s
    /// port to match before committing it, since that test will otherwise
    /// go red against the new golden.
    #[test]
    fn required_files_matches_committed_golden() {
        let dir = golden_dir();
        let input_path = dir.join("input.json");
        let output_path = dir.join("output.json");

        let input_json = std::fs::read_to_string(&input_path)
            .unwrap_or_else(|e| panic!("failed to read golden input {input_path:?}: {e}"));
        let cases: Vec<GoldenCase> = serde_json::from_str(&input_json)
            .unwrap_or_else(|e| panic!("failed to parse golden input {input_path:?}: {e}"));
        assert!(
            !cases.is_empty(),
            "golden input fixture {input_path:?} has no cases"
        );

        let computed = compute_golden_output(&cases);
        let computed_json = serde_json::to_string_pretty(&computed).unwrap() + "\n";

        if std::env::var_os("UPDATE_GOLDEN_PROCESSOR_OUTPUTS").is_some() {
            std::fs::write(&output_path, &computed_json)
                .unwrap_or_else(|e| panic!("failed to write golden output {output_path:?}: {e}"));
            eprintln!("Regenerated golden output at {output_path:?}");
            return;
        }

        let golden_json = std::fs::read_to_string(&output_path).unwrap_or_else(|e| {
            panic!(
                "failed to read committed golden output {output_path:?}: {e} \
                 (run with UPDATE_GOLDEN_PROCESSOR_OUTPUTS=1 to generate it)"
            )
        });

        assert_eq!(
            computed_json, golden_json,
            "required_files' output no longer matches the committed golden at \
             {output_path:?}. If this is an intended behavior change: re-run with \
             UPDATE_GOLDEN_PROCESSOR_OUTPUTS=1 cargo test -p carbon_app \
             required_files_matches_committed_golden, review the diff, update \
             apps/desktop/e2e-tests/helpers/processorOutputs.test.ts's port to \
             match, and commit both together."
        );
    }
}
