use super::UpdateValue;
use crate::managers::App;
use crate::managers::instance::modpack::packinfo::PackInfo;
use anyhow::Context;
use carbon_net::{DownloadOptions, Downloadable, Progress};
use carbon_platforms::modrinth::version::{ModpackIndex, ModrinthEnvironmentSupport, VersionFile};
use carbon_rt_path::InstancePath;
use std::borrow::BorrowMut;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::task::spawn_blocking;

#[derive(Error, Debug)]
pub enum PathTraversalError {
    #[error("Path `{0}` has a root component and joining it will cause a path traversal")]
    PathHasRoot(PathBuf),
    #[error("Path `{0}` climbs above it's root and joining it will cause a path traversal")]
    PathClimbsAboveRoot(PathBuf),
}

/// 1. Reduce multiple slashes to a single slash.
/// 2. Eliminate `.` path name elements (the current directory).
/// 3. Eliminate `..` path name elements (the parent directory) and the non-`.` non-`..`,
/// element that precedes them.
/// 4. Eliminate `..` elements that begin a rooted path, that is, replace `/..` by `/` at the
/// beginning of a path.
/// 5. Leave intact `..` elements that begin a non-rooted path.
///
/// If the result of this process is an empty string, return the relative path `"."`, representing the
/// current directory.
pub fn path_clean<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut out = Vec::new();
    for comp in path.as_ref().components() {
        match comp {
            Component::CurDir => (),
            Component::ParentDir => match out.last() {
                Some(Component::RootDir) => (),
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                _ => out.push(comp),
            },
            comp => out.push(comp),
        }
    }

    if out.is_empty() {
        PathBuf::from(".")
    } else {
        out.iter().collect()
    }
}

/// lexically checks that the join operation does not climb above the root
/// the returned bath is guaranteed to be under the provided root baring the influence of symbolic
/// links. This should be later checked by calling `canonicalize` after we are sure the parent
/// directories exist.
///
pub fn secure_path_join<P1: AsRef<Path>, P2: AsRef<Path>>(
    root: P1,
    unsafe_path: P2,
) -> Result<PathBuf, PathTraversalError> {
    let unsafe_path = unsafe_path.as_ref();
    if unsafe_path.has_root() {
        return Err(PathTraversalError::PathHasRoot(unsafe_path.to_path_buf()));
    } else if unsafe_path.starts_with("..") {
        return Err(PathTraversalError::PathClimbsAboveRoot(
            unsafe_path.to_path_buf(),
        ));
    }
    let clean_root = path_clean(root);

    // clean path first to prevent traversing above
    let clean_path = path_clean(unsafe_path);

    // join two clean paths
    let result_path = clean_root.join(clean_path);

    // reclean to resolve remaining indirection
    let clean_result = path_clean(result_path);

    // double check to make sure we haven't climbed out
    if !clean_result.starts_with(clean_root) {
        Err(PathTraversalError::PathClimbsAboveRoot(
            unsafe_path.to_path_buf(),
        ))
    } else {
        Ok(clean_result)
    }
}

/// Ceiling on how many decompressed bytes a single hand-rolled zip extraction
/// pass (one call site's whole override loop) writes to disk. Rejects a
/// decompression bomb: a tiny compressed entry inflating without bound.
///
/// Deliberately not tied to the share size cap, which limits the *compressed*
/// archive GDL uploads for its own share feature. Third-party modpacks are
/// bound by nothing GDL controls, and one bundling worlds, resource packs or
/// shader packs legitimately reaches gigabytes of uncompressed overrides, so
/// borrowing that number aborted real imports partway through with an error no
/// retry could get past.
pub const MAX_EXTRACTED_OVERRIDE_BYTES: u64 = 16 * 1024 * 1024 * 1024;

/// Ceiling on a single archive entry read fully into memory (rather than
/// streamed to disk) so it can be hashed. Far below
/// [`MAX_EXTRACTED_OVERRIDE_BYTES`] because this path only matches small
/// unresolved files: anything larger is not a candidate anyway, and the bound is
/// what keeps a bomb from being allocated in full just to hash it.
pub const MAX_HASHED_ENTRY_BYTES: u64 = 64 * 1024 * 1024;

/// True if a zip entry's Unix mode indicates it's a symlink (`S_IFLNK`). Pass
/// `entry.unix_mode()` directly. A hand-rolled extractor that materializes a
/// symlink entry as a regular file ends up writing the link's *target path text*
/// as the file's contents -- silent data corruption, distinct from path
/// traversal (handled separately via `secure_path_join`). `None` (no Unix
/// external attributes, e.g. an archive built on Windows) is never a symlink.
pub fn is_symlink_mode(unix_mode: Option<u32>) -> bool {
    unix_mode.is_some_and(|mode| mode & 0o170000 == 0o120000)
}

/// Copies `reader` into `writer`, erroring out instead of copying more than
/// `limit` bytes. Guards a hand-rolled zip extractor against a decompression
/// bomb: a tiny compressed entry that expands to an enormous amount of data
/// would otherwise be written to disk (or read into memory, e.g. to hash it)
/// with no bound at all. Returns the number of bytes actually copied so callers
/// can thread a shrinking budget across multiple entries.
pub fn copy_bounded<R: std::io::Read, W: std::io::Write>(
    reader: &mut R,
    writer: &mut W,
    limit: u64,
) -> anyhow::Result<u64> {
    // UFCS avoids the ambiguity between `R`'s own `Read` impl (which would move
    // `*reader`) and the blanket `Read for &mut R` impl by unifying `Self` with
    // `reader`'s actual type (`&mut R`) directly.
    let mut limited = std::io::Read::take(reader, limit + 1);
    let copied = std::io::copy(&mut limited, writer)?;
    if copied > limit {
        anyhow::bail!("Archive entry exceeds the {limit} byte decompressed size limit");
    }
    Ok(copied)
}

#[derive(Debug, Copy, Clone)]
pub enum ProgressState {
    Idle,
    ExtractingPackOverrides(u64, u64),
    AcquiringPackMetadata(u64, u64),
}

#[derive(Debug)]
pub struct ModpackInfo {
    pub index: ModpackIndex,
    // (downloadable, existing path)
    pub downloadables: Vec<(Downloadable, Option<String>)>,
}

pub async fn download_mrpack(
    app: &App,
    mrpack_file: &VersionFile,
    target_path: &Path,
    progress_percentage_sender: tokio::sync::watch::Sender<UpdateValue<(u64, u64)>>,
) -> anyhow::Result<()> {
    let _pack_download_url = mrpack_file.url.clone();

    // generate uuid
    let file = app
        .settings_manager()
        .runtime_path
        .get_temp()
        .maketmpfile()
        .await?;
    let file_downloadable = Downloadable::new(&mrpack_file.url.to_string(), file.to_path_buf())
        .with_size(mrpack_file.size as u64);

    tokio::fs::create_dir_all(
        &file
            .parent()
            .ok_or(anyhow::anyhow!("Failed to get parent"))?,
    )
    .await?;

    let (download_progress_sender, mut download_progress_recv) =
        tokio::sync::watch::channel(Progress::new());

    tokio::spawn(async move {
        while download_progress_recv.borrow_mut().changed().await.is_ok() {
            let p = download_progress_recv.borrow();
            progress_percentage_sender
                .send_modify(|progress| progress.set((p.current_size, p.total_size)));
        }

        Ok::<_, anyhow::Error>(progress_percentage_sender)
    });

    carbon_net::download_multiple(
        &[file_downloadable],
        DownloadOptions::builder()
            .concurrency(1)
            .progress_sender(download_progress_sender)
            .build(),
    )
    .await
    .with_context(|| {
        format!(
            "Failed to download modrinth modpack from url: {}",
            mrpack_file.url
        )
    })?;

    file.try_rename_or_move(target_path).await?;
    Ok(())
}

#[tracing::instrument(skip(app, packinfo, progress_percentage_sender))]
pub async fn prepare_modpack_from_mrpack(
    app: &App,
    mrpack_path: &Path,
    instance_path: &InstancePath,
    skip_overlays: bool,
    packinfo: Option<&PackInfo>,
    progress_percentage_sender: tokio::sync::watch::Sender<ProgressState>,
) -> anyhow::Result<ModpackInfo> {
    let file_path_clone = mrpack_path.to_path_buf();
    let (mut archive, index) = spawn_blocking(move || {
        let file = std::fs::File::open(file_path_clone)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let index: ModpackIndex = {
            let file = archive.by_name("modrinth.index.json")?;
            serde_json::from_reader(file)?
        };

        Ok::<_, anyhow::Error>((archive, index))
    })
    .await??;

    let required_files: Vec<_> = index
        .files
        .iter()
        .filter(|&file| {
            file.env.as_ref().map_or(true, |env| match env.client {
                ModrinthEnvironmentSupport::Required => true,
                _ => false,
            })
        })
        .cloned()
        .collect();

    let _optional_files: Vec<_> = index
        .files
        .iter()
        .filter(|&file| {
            file.env.as_ref().map_or(false, |env| match env.client {
                ModrinthEnvironmentSupport::Optional => true,
                _ => false,
            })
        })
        .cloned()
        .collect();

    let downloadables = {
        let files_len = required_files.len() as u64;

        let data_path = instance_path.get_data_path();
        tokio::fs::create_dir_all(&data_path).await?;

        let instance_path = instance_path.clone();

        required_files
            .into_iter()
            .enumerate()
            .map(|(i, file)| {
                let _app = app.clone();

                let data_path = instance_path.get_data_path();

                let existing_path = packinfo
                    .map(|packinfo| {
                        let mut sha512 = [0u8; 64];
                        hex::decode_to_slice(&file.hashes.sha512, &mut sha512).ok()?;

                        let packinfo_path = format!("/{}", file.path);

                        match packinfo.files.get(&packinfo_path) {
                            Some(hashes) if sha512 == hashes.sha512 => Some(packinfo_path),
                            _ => None,
                        }
                    })
                    .flatten();

                let target_path = secure_path_join(&data_path, &file.path)?;

                // Modrinth manifest carries the sha512 of every file — verify
                // it on download to defend against MITM / CDN poisoning.
                let checksum = if !file.hashes.sha512.is_empty() {
                    Some(carbon_net::Checksum::Sha512(file.hashes.sha512.clone()))
                } else if !file.hashes.sha1.is_empty() {
                    Some(carbon_net::Checksum::Sha1(file.hashes.sha1.clone()))
                } else {
                    None
                };

                let downloadable = Downloadable::new(
                    file.downloads
                        .first()
                        .ok_or(anyhow::anyhow!("Failed to get download url for mod"))?
                        .to_string(),
                    target_path,
                )
                .with_size(file.file_size as u64)
                .with_checksum(checksum);

                progress_percentage_sender
                    .send(ProgressState::AcquiringPackMetadata(i as u64, files_len))?;

                Ok::<_, anyhow::Error>((downloadable, existing_path))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    if !skip_overlays {
        let data_path = instance_path.get_data_path();
        let overrides_folder_name = "overrides";
        spawn_blocking(move || {
            let total_archive_files = archive.len() as u64;
            // Cumulative across the whole pass, not per entry: without this, many
            // entries each just under a per-entry cap could still add up to an
            // unbounded amount written to disk.
            let mut extracted_bytes: u64 = 0;
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                if !(file.name().starts_with(&overrides_folder_name)) {
                    continue;
                }

                let out_path = match file.enclosed_name() {
                    Some(path) => match path.strip_prefix(overrides_folder_name) {
                        Ok(stripped) => secure_path_join(Path::new(&data_path), stripped)?,
                        // The name begins with the prefix string but is not inside the
                        // overrides directory (e.g. "overrides-extra/..."); skip it instead
                        // of panicking on the non-matching path component.
                        Err(_) => continue,
                    },
                    None => continue,
                };

                if file.name().ends_with('/') {
                    continue;
                } else if is_symlink_mode(file.unix_mode()) {
                    // A symlink entry materialized as a regular file would end up
                    // containing the link's target path text instead of real data.
                    tracing::warn!(
                        "Skipping modrinth override entry `{}`: symlinks are not extracted",
                        file.name()
                    );
                    continue;
                } else {
                    if let Some(parent) = out_path.parent() {
                        if !parent.exists() {
                            std::fs::create_dir_all(parent)?;
                        }
                    }
                    let mut out_file = std::fs::File::create(&out_path)?;

                    let remaining_budget =
                        MAX_EXTRACTED_OVERRIDE_BYTES.saturating_sub(extracted_bytes);
                    extracted_bytes += copy_bounded(&mut file, &mut out_file, remaining_budget)?;
                }

                progress_percentage_sender.send(ProgressState::ExtractingPackOverrides(
                    i as u64,
                    total_archive_files,
                ))?;
            }

            Ok::<(), anyhow::Error>(())
        })
        .await??;
    }

    Ok(ModpackInfo {
        index,
        downloadables,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_symlink_mode_recognises_the_symlink_file_type_bits() {
        // S_IFLNK | 0o777, the mode `zip` reports for a typical symlink entry.
        assert!(is_symlink_mode(Some(0o120777)));
        // A regular file (S_IFREG) and a directory (S_IFDIR) are not symlinks.
        assert!(!is_symlink_mode(Some(0o100644)));
        assert!(!is_symlink_mode(Some(0o040755)));
        // No Unix external attributes at all (e.g. an archive built on Windows).
        assert!(!is_symlink_mode(None));
    }

    #[test]
    fn copy_bounded_allows_data_within_the_limit() {
        let data = b"hello world";
        let mut out = Vec::new();
        let copied = copy_bounded(&mut &data[..], &mut out, data.len() as u64).unwrap();
        assert_eq!(copied, data.len() as u64);
        assert_eq!(out, data);
    }

    /// Regression test: a decompression bomb (a small compressed entry that
    /// expands to far more data than expected) must error out instead of being
    /// copied through without limit.
    #[test]
    fn copy_bounded_rejects_data_over_the_limit() {
        let data = vec![0u8; 1024];
        let mut out = Vec::new();
        assert!(copy_bounded(&mut &data[..], &mut out, 100).is_err());
    }

    #[test]
    fn copy_bounded_accepts_data_exactly_at_the_limit() {
        let data = vec![0u8; 100];
        let mut out = Vec::new();
        let copied = copy_bounded(&mut &data[..], &mut out, 100).unwrap();
        assert_eq!(copied, 100);
    }

    #[test]
    fn extraction_limits_are_independent_of_the_share_size_cap() {
        use crate::managers::instance::export::MAX_SHARE_SIZE_BYTES;

        // The share cap limits the compressed archive GDL uploads; these bound
        // uncompressed extraction of third-party packs. Tying them together
        // aborted real modpack imports, so they must stay separate — and the
        // extraction budget has to leave room for packs that ship worlds or
        // resource packs.
        assert!(
            MAX_EXTRACTED_OVERRIDE_BYTES > MAX_SHARE_SIZE_BYTES,
            "override extraction must not be capped at the share upload limit"
        );
        assert!(
            MAX_HASHED_ENTRY_BYTES < MAX_EXTRACTED_OVERRIDE_BYTES,
            "an entry buffered in memory must be bounded far below a whole extraction pass"
        );
    }
}
