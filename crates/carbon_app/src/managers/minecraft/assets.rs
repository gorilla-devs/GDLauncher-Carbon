use crate::domain::minecraft::minecraft::asset_object_location;
use anyhow::Context;
use carbon_repos::db_error::DbError;
use carbon_repos::db_exec::Db;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::version_meta;
use carbon_rt_path::AssetsPath;
use chrono::Utc;
use daedalus::minecraft::{AssetIndex, AssetsIndex};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::trace;

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Can't fetch assets index manifest: {0}")]
    FetchAssetsIndexManifest(#[from] reqwest::Error),
    #[error("Can't execute db query: {0}")]
    QueryError(#[from] DbError),
}

// Asset identities use the existing cross-platform relative-path authority. Index
// IDs are one component: a slash would name another cache/mapping namespace.
fn validate_index_id(id: &str) -> anyhow::Result<()> {
    crate::managers::server::world::validate_relative(id).context("Unsafe asset index ID")?;
    anyhow::ensure!(
        !id.contains('/'),
        "Asset index ID must be one component: {id}"
    );
    Ok(())
}
fn validate_objects(index: &AssetsIndex) -> anyhow::Result<()> {
    for key in index.objects.keys() {
        crate::managers::server::world::validate_relative(key)
            .with_context(|| format!("Unsafe asset object destination: {key}"))?;
    }
    Ok(())
}
/// The first folder strictly between `root` and the last component of
/// `relative` that is a symlink or junction. `root` and everything above it
/// are never inspected: users link `assets`, `assets/indexes`,
/// `assets/virtual` or their instances folder to share files with another
/// launcher or move them to another drive, and 2.0.40 followed those links,
/// as Prism and the vanilla launcher do. The last component is not inspected
/// either: publication renames over it, which replaces a link rather than
/// writing through it. std's `is_symlink` is true on Windows only for
/// name-surrogate reparse points (symlinks, junctions, mount points), so
/// WOF-compressed, deduplicated and cloud placeholder files pass. These
/// checks do not claim immunity to concurrent path replacement.
fn linked_folder_below(root: &Path, relative: &Path) -> std::io::Result<Option<PathBuf>> {
    let mut path = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        if components.peek().is_none() {
            break;
        }
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Ok(Some(path)),
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}
/// Where the asset index key `key` lands inside the mapping folder `root`.
/// Keys are always validated lexically. `owned` is set for the virtual mapping,
/// which the launcher repairs by replacing files and prunes by deleting
/// leftovers, so a linked folder below its root is refused: following it would
/// replace or delete files outside the launcher's folder. The pre-1.6 resources
/// mapping only adds missing files and never replaces or deletes, so it
/// follows links like 2.0.40 did.
fn mapping_destination(root: &Path, key: &str, owned: bool) -> anyhow::Result<PathBuf> {
    crate::managers::server::world::validate_relative(key)
        .with_context(|| format!("Unsafe asset object destination: {key}"))?;
    let mut destination = root.to_path_buf();
    destination.extend(key.split('/'));
    if owned && let Some(link) = linked_folder_below(root, Path::new(key))? {
        anyhow::bail!(
            "Cannot rebuild the game's assets: {} is a symbolic link or junction inside {}, \
             a folder the launcher manages and repairs by replacing and removing files. \
             Remove that link (the folder it points to is left untouched) and press Play \
             again; the launcher recreates the folder.",
            link.display(),
            root.display()
        );
    }
    Ok(destination)
}
async fn publish_index(
    root: &Path,
    id: &str,
    bytes: &[u8],
    expected_size: u64,
) -> anyhow::Result<()> {
    validate_index_id(id)?;
    // A linked assets or indexes folder is followed, as in 2.0.40.
    let path = root.join(format!("{id}.json"));
    let file_ok = match std::fs::metadata(&path) {
        Ok(metadata) => metadata.is_file() && metadata.len() == expected_size,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    if !file_ok {
        tokio::fs::create_dir_all(root).await?;
        let (root, bytes, destination) = (root.to_owned(), bytes.to_vec(), path.clone());
        // Stage beside the index and rename it into place: readers never see a
        // partial index, and a link left at the index's name is replaced
        // instead of written through, so the file it points to stays untouched.
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut staged = crate::util::publish::temp_in(&root)?;
            std::io::Write::write_all(&mut staged, &bytes)?;
            // tempfile creates owner-only files; 2.0.40 wrote indexes 0644.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                staged
                    .as_file()
                    .set_permissions(std::fs::Permissions::from_mode(0o644))?;
            }
            crate::util::durable::sync_file(staged.as_file())?;
            crate::util::publish::persist(staged, &destination)?;
            Ok(())
        })
        .await?
        // One message carrying the OS reason: the instance tile shows only the
        // first cause, which `with_context` would leave without it.
        .map_err(|error| {
            anyhow::anyhow!(
                "Failed to save the asset index {}: {error:#}",
                path.display()
            )
        })?;
    }
    Ok(())
}

pub async fn get_meta(
    db: &Db,
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    version_asset_index: &AssetIndex,
) -> anyhow::Result<(AssetsIndex, Vec<u8>)> {
    // Only the network and the database are read here; the index file on disk
    // is written by publish_index.
    validate_index_id(&version_asset_index.id)?;
    static LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = LOCK.lock().await;

    let update_cache = || async {
        let resp = reqwest_client
            .get(version_asset_index.url.clone())
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "Failed to fetch asset index from `{}`: {}",
                version_asset_index.url,
                resp.status()
            );
        }

        let asset_index = resp.bytes().await.with_context(|| {
            format!(
                "Failed to fetch asset index from `{}`",
                version_asset_index.url
            )
        })?;

        // Validate the freshly fetched body before caching it: a 200 response with an
        // unparseable body must not overwrite a previously-good cached asset index.
        let parsed = serde_json::from_slice::<AssetsIndex>(&asset_index).with_context(|| {
            format!(
                "Failed to parse asset index from `{}`",
                version_asset_index.url
            )
        })?;

        // A structurally-valid but empty index (no objects) is almost certainly a
        // bad response (captive portal, truncated body). Treat it as a failure so
        // the fallback keeps the previously-good cache instead of overwriting it.
        if parsed.objects.is_empty() {
            anyhow::bail!(
                "asset index from `{}` contains no objects",
                version_asset_index.url
            );
        }

        validate_objects(&parsed)?;
        version_meta::upsert_assets_meta(
            db,
            &version_asset_index.id,
            &asset_index,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;

        Ok((parsed, asset_index.to_vec()))
    };

    let asset_index = match update_cache().await {
        Ok(result) => Ok(result),
        Err(err) => {
            let id_owned = version_asset_index.id.clone();
            let db_cache = version_meta::get_assets_meta(db, &id_owned).await?;

            if let Some(db_cache) = db_cache {
                let asset_index = serde_json::from_slice::<AssetsIndex>(&db_cache.assets_index);

                if let Ok(asset_index) = asset_index {
                    validate_objects(&asset_index)?;
                    trace!("Asset index {} found in cache", version_asset_index.id);
                    return Ok((asset_index, db_cache.assets_index));
                } else {
                    tracing::warn!(
                        "Failed to deserialize asset index for {} from cache, re-fetching: {} from {}",
                        version_asset_index.id,
                        db_cache.id,
                        version_asset_index.url.clone()
                    );
                }
            }

            Err(err)
        }
    }?;

    Ok(asset_index)
}

pub enum AssetsDir {
    Index(PathBuf),
    Virtual(PathBuf),
    InstanceMapped(PathBuf),
}

impl AssetsDir {
    pub fn to_path_buf(&self) -> PathBuf {
        match self {
            Self::Index(buf) => buf.clone(),
            Self::Virtual(buf) => buf.clone(),
            Self::InstanceMapped(buf) => buf.clone(),
        }
    }
}

pub async fn get_assets_dir(
    db: &Db,
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    version_assets_index: &AssetIndex,
    assets_path: AssetsPath,
    resources_dir: PathBuf,
) -> anyhow::Result<AssetsDir> {
    let (assets_index, _) = get_meta(db, reqwest_client, version_assets_index).await?;

    if assets_index.map_virtual {
        // get_meta validated the ID as one plain path component.
        Ok(AssetsDir::Virtual(
            assets_path
                .get_virtual_path()
                .join(&version_assets_index.id),
        ))
    } else if assets_index.map_to_resources {
        Ok(AssetsDir::InstanceMapped(resources_dir))
    } else {
        Ok(AssetsDir::Index(assets_path.to_path()))
    }
}

pub async fn reconstruct_assets(
    db: &Db,
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    version_asset_index: &AssetIndex,
    assets_path: AssetsPath,
    resources_dir: PathBuf,
) -> anyhow::Result<()> {
    let (assets_index, assets_index_bytes) =
        get_meta(db, reqwest_client, version_asset_index).await?;

    validate_objects(&assets_index)?;
    publish_index(
        &assets_path.get_indexes_path(),
        &version_asset_index.id,
        &assets_index_bytes,
        version_asset_index.size as u64,
    )
    .await?;

    reconstruct_mappings(
        assets_index,
        assets_path,
        resources_dir,
        &version_asset_index.id,
    )
    .await
}

async fn reconstruct_mappings(
    assets_index: AssetsIndex,
    assets_path: AssetsPath,
    resources_dir: PathBuf,
    index_id: &str,
) -> anyhow::Result<()> {
    validate_index_id(index_id)?;
    validate_objects(&assets_index)?;
    let index_id = index_id.to_owned();
    // Retain publication/cleanup ownership inside the blocking worker, including
    // after requester cancellation. All mapping producers use this same gate.
    tokio::task::spawn_blocking(move || {
        static MAPPING_PUBLICATION: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = MAPPING_PUBLICATION
            .lock()
            .map_err(|_| anyhow::anyhow!("Asset mapping publication panicked"))?;
        // The index ID was validated above as one plain path component.
        let (target_path, owned) = if assets_index.map_virtual {
            (assets_path.get_virtual_path().join(&index_id), true)
        } else if assets_index.map_to_resources {
            (resources_dir, false)
        } else {
            return Ok(());
        };
        // Validate the complete set before creating directories, replacing files
        // or deleting stale entries. Rejecting one key must not partially publish.
        for path in assets_index.objects.keys() {
            mapping_destination(&target_path, path, owned)?;
        }
        std::fs::create_dir_all(&target_path)?;
        let mut existing_files: HashSet<PathBuf> = walkdir::WalkDir::new(&target_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path().to_owned())
            .collect();
        for (path, object) in &assets_index.objects {
            let Some((_, source)) =
                asset_object_location(&assets_path.get_objects_path(), &object.hash)
            else {
                tracing::warn!("Skipping asset with malformed hash {:?}", object.hash);
                continue;
            };
            let destination = mapping_destination(&target_path, path, owned)?;
            existing_files.remove(&destination);
            if !assets_index.map_virtual && destination.try_exists()? {
                continue;
            }
            if assets_index.map_virtual
                && asset_matches(&destination, object.size as u64, &object.hash)?
            {
                continue;
            }
            let parent = destination.parent().context("Asset path has no parent")?;
            std::fs::create_dir_all(parent)?;
            mapping_destination(&target_path, path, owned)?;
            let mut staged = crate::util::publish::temp_in(parent)?;
            let mut input = std::fs::File::open(&source)?;
            std::io::copy(&mut input, &mut staged)?;
            std::io::Write::flush(&mut staged)?;
            // tempfile creates owner-only files; 2.0.40 copied each object
            // with its own mode (usually 0644), which an assets folder shared
            // with another OS account relies on.
            #[cfg(unix)]
            staged
                .as_file()
                .set_permissions(input.metadata()?.permissions())?;
            anyhow::ensure!(
                asset_matches(staged.path(), object.size as u64, &object.hash)?,
                "Asset source does not match index: {}",
                source.display()
            );
            crate::util::durable::sync_file(staged.as_file())?;
            mapping_destination(&target_path, path, owned)?;
            if assets_index.map_virtual {
                crate::util::publish::persist(staged, &destination)?;
            } else {
                match crate::util::publish::persist_noclobber(staged, &destination) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
        if assets_index.map_virtual {
            // Leftover cleanup never fails the launch. walkdir does not follow
            // links, so it lists a linked file or folder without entering it,
            // and remove_file removes a link itself, never what it points to.
            for path in existing_files {
                let Ok(relative) = path.strip_prefix(&target_path) else {
                    continue;
                };
                if relative.as_os_str().is_empty() || path.is_dir() {
                    continue;
                }
                match linked_folder_below(&target_path, relative) {
                    Ok(None) => (),
                    Ok(Some(link)) => {
                        tracing::warn!(
                            "Keeping leftover virtual asset {}: {} is a link",
                            path.display(),
                            link.display()
                        );
                        continue;
                    }
                    Err(error) => {
                        tracing::warn!(
                            "Keeping leftover virtual asset {}: {error}",
                            path.display()
                        );
                        continue;
                    }
                }
                if let Err(error) = std::fs::remove_file(&path) {
                    tracing::warn!(
                        "Failed to remove leftover virtual asset {}: {error}",
                        path.display()
                    );
                }
            }
        }
        Ok(())
    })
    .await?
}

fn asset_matches(path: &std::path::Path, size: u64, expected: &str) -> anyhow::Result<bool> {
    use sha1::{Digest, Sha1};
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if file.metadata()?.len() != size {
        return Ok(false);
    }
    let mut hash = Sha1::new();
    std::io::copy(&mut file, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()).eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod task18_tests {
    use super::*;
    use sha1::{Digest, Sha1};
    fn boundary_fixture(
        root: &std::path::Path,
        key: &str,
    ) -> (carbon_rt_path::RuntimePath, AssetsIndex) {
        let runtime = carbon_rt_path::RuntimePath::new(root.into());
        let bytes = b"verified asset";
        let hash = format!("{:x}", Sha1::digest(bytes));
        let (_, source) =
            asset_object_location(&runtime.get_assets().get_objects_path(), &hash).unwrap();
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(source, bytes).unwrap();
        let index = serde_json::from_value(
            serde_json::json!({"virtual":true,"objects":{key:{"hash":hash,"size":bytes.len()}}}),
        )
        .unwrap();
        (runtime, index)
    }
    #[tokio::test]
    async fn task18_asset_object_destinations_reject_escape_before_overwrite() {
        let root = tempfile::tempdir().unwrap();
        let runtime = carbon_rt_path::RuntimePath::new(root.path().into());
        let outside = runtime.get_assets().get_virtual_path().join("outside.txt");
        std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
        for key in [
            "../outside.txt",
            outside.to_str().unwrap(),
            "C:/outside.txt",
            "\\\\host\\outside.txt",
            "nested/../../outside.txt",
        ] {
            std::fs::write(&outside, b"owned outside bytes").unwrap();
            let (runtime, index) = boundary_fixture(root.path(), key);
            let result = reconstruct_mappings(
                index,
                runtime.get_assets(),
                root.path().join("resources"),
                "legacy",
            )
            .await;
            assert_eq!(
                std::fs::read(&outside).unwrap(),
                b"owned outside bytes",
                "key {key} escaped its mapping"
            );
            assert!(result.is_err(), "malformed asset identity accepted: {key}");
        }
    }
    #[tokio::test]
    async fn task18_asset_index_destinations_reject_escape() {
        let root = tempfile::tempdir().unwrap();
        for id in [
            "../outside",
            root.path().to_str().unwrap(),
            "C:/outside",
            "\\\\host\\outside",
            "nested/index",
            "",
        ] {
            let (runtime, index) = boundary_fixture(root.path(), "nested/good.txt");
            let result = reconstruct_mappings(
                index,
                runtime.get_assets(),
                root.path().join("resources"),
                id,
            )
            .await;
            assert!(result.is_err(), "malformed index identity accepted: {id}");
            assert!(
                publish_index(&runtime.get_assets().get_indexes_path(), id, b"index", 5)
                    .await
                    .is_err()
            );
        }
    }
    #[tokio::test]
    async fn task18_asset_index_publication_healthy_and_link_controls() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("indexes");
        publish_index(&cache, "1.7.10", b"index", 5).await.unwrap();
        let modified = std::fs::metadata(cache.join("1.7.10.json"))
            .unwrap()
            .modified()
            .unwrap();
        publish_index(&cache, "1.7.10", b"index", 5).await.unwrap();
        assert_eq!(
            std::fs::metadata(cache.join("1.7.10.json"))
                .unwrap()
                .modified()
                .unwrap(),
            modified
        );
        #[cfg(unix)]
        {
            // A link at the index file is replaced, not written through.
            let outside = root.path().join("outside.txt");
            std::fs::write(&outside, b"outside bytes").unwrap();
            std::os::unix::fs::symlink(&outside, cache.join("linked.json")).unwrap();
            publish_index(&cache, "linked", b"index", 5).await.unwrap();
            assert_eq!(std::fs::read(&outside).unwrap(), b"outside bytes");
            let published = std::fs::symlink_metadata(cache.join("linked.json")).unwrap();
            assert!(published.file_type().is_file());
            assert_eq!(std::fs::read(cache.join("linked.json")).unwrap(), b"index");
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(published.permissions().mode() & 0o777, 0o644);
            // A linked indexes folder is followed.
            let alias = root.path().join("alias");
            std::os::unix::fs::symlink(&cache, &alias).unwrap();
            publish_index(&alias, "new", b"index", 5).await.unwrap();
            assert_eq!(std::fs::read(cache.join("new.json")).unwrap(), b"index");
            // No staging file is left behind.
            assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 3);
        }
    }
    #[tokio::test]
    async fn task18_asset_index_save_failure_names_the_os_reason() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("indexes");
        // A non-empty folder at the index's name cannot be replaced by a file.
        let blocked = cache.join("1.7.10.json");
        std::fs::create_dir_all(blocked.join("inside")).unwrap();
        let error = publish_index(&cache, "1.7.10", b"index", 5)
            .await
            .unwrap_err();
        // The instance tile shows only the first cause, so it carries both
        // what failed and why.
        assert_eq!(error.chain().count(), 1);
        let message = error.to_string();
        let prefix = format!("Failed to save the asset index {}: ", blocked.display());
        assert!(message.starts_with(&prefix), "{message}");
        assert!(message.len() > prefix.len(), "{message}");
        assert!(blocked.join("inside").is_dir());
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn task18_asset_linked_ancestor_preserves_outside_file() {
        let root = tempfile::tempdir().unwrap();
        let (runtime, index) = boundary_fixture(root.path(), "linked/outside.txt");
        let target = runtime.get_assets().get_virtual_path().join("legacy");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("outside.txt"), b"outside bytes").unwrap();
        std::os::unix::fs::symlink(&outside, target.join("linked")).unwrap();
        let result = reconstruct_mappings(
            index,
            runtime.get_assets(),
            root.path().join("resources"),
            "legacy",
        )
        .await;
        assert_eq!(
            std::fs::read(outside.join("outside.txt")).unwrap(),
            b"outside bytes"
        );
        assert!(result.is_err());
    }
    async fn run_case(virtual_mapping: bool, initial: Option<&[u8]>) {
        let root = tempfile::tempdir().unwrap();
        let runtime = carbon_rt_path::RuntimePath::new(root.path().into());
        let assets = runtime.get_assets();
        let good = b"complete fixture asset bytes";
        let hash = format!("{:x}", Sha1::digest(good));
        let (_, source) = asset_object_location(&assets.get_objects_path(), &hash).unwrap();
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, good).unwrap();
        // Objects are readable by others, as carbon_net creates them under
        // the usual umask; set it here so the test does not depend on it.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        let resources = root.path().join("resources");
        let target = if virtual_mapping {
            assets.get_virtual_path().join("legacy")
        } else {
            resources.clone()
        };
        std::fs::create_dir_all(&target).unwrap();
        let destination = target.join("test.txt");
        if let Some(initial) = initial {
            std::fs::write(&destination, initial).unwrap();
        }
        std::fs::write(target.join("leftover.txt"), b"leftover").unwrap();
        for _ in 0..2 {
            let index: AssetsIndex = serde_json::from_value(serde_json::json!({"virtual": virtual_mapping, "map_to_resources": true, "objects": {"test.txt": {"hash": hash, "size": good.len()}}})).unwrap();
            reconstruct_mappings(index, runtime.get_assets(), resources.clone(), "legacy")
                .await
                .unwrap();
            let expected = if virtual_mapping {
                good.as_slice()
            } else {
                initial.unwrap_or(good)
            };
            assert_eq!(std::fs::read(&destination).unwrap(), expected);
            // A mapping file written here keeps the object's mode, as in
            // 2.0.40, instead of tempfile's owner-only 0600.
            #[cfg(unix)]
            if virtual_mapping || initial.is_none() {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&destination)
                    .unwrap()
                    .permissions()
                    .mode();
                assert_eq!(mode & 0o777, 0o644);
            }
        }
        assert_eq!(target.join("leftover.txt").exists(), !virtual_mapping);
    }
    #[tokio::test]
    async fn task18_virtual_assets_repair_partial_and_same_size() {
        for bytes in [vec![], vec![0; 5], vec![0; 28]] {
            run_case(true, Some(&bytes)).await;
        }
    }
    #[tokio::test]
    async fn task18_virtual_assets_preserve_resources_and_absent_controls() {
        run_case(false, Some(b"user edited resource")).await;
        run_case(false, None).await;
        run_case(true, None).await;
        run_case(true, Some(b"complete fixture asset bytes")).await;
    }
    #[tokio::test]
    async fn task18_assets_failed_source_preserves_previous_and_modern_skips_mapping() {
        let root = tempfile::tempdir().unwrap();
        let runtime = carbon_rt_path::RuntimePath::new(root.path().into());
        let good = b"verified";
        let hash = format!("{:x}", Sha1::digest(good));
        let (_, source) =
            asset_object_location(&runtime.get_assets().get_objects_path(), &hash).unwrap();
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, b"invalid!").unwrap();
        let target = runtime
            .get_assets()
            .get_virtual_path()
            .join("legacy/test.txt");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, b"previous").unwrap();
        let index = |mapping| {
            serde_json::from_value::<AssetsIndex>(serde_json::json!({"virtual":mapping,"map_to_resources":false,"objects":{"test.txt":{"hash":hash,"size":good.len()}}})).unwrap()
        };
        assert!(
            reconstruct_mappings(
                index(true),
                runtime.get_assets(),
                root.path().join("resources"),
                "legacy"
            )
            .await
            .is_err()
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"previous");
        assert_eq!(
            std::fs::read_dir(target.parent().unwrap()).unwrap().count(),
            1,
            "private failed staging cleaned without replacing target"
        );
        reconstruct_mappings(
            index(false),
            runtime.get_assets(),
            root.path().join("resources"),
            "modern",
        )
        .await
        .unwrap();
        assert!(
            !runtime
                .get_assets()
                .get_virtual_path()
                .join("modern")
                .exists()
        );
    }
    #[tokio::test]
    async fn task18_assets_concurrent_repair_and_healthy_timestamp() {
        let root = tempfile::tempdir().unwrap();
        let runtime = carbon_rt_path::RuntimePath::new(root.path().into());
        let bytes = b"complete shared mapping";
        let hash = format!("{:x}", Sha1::digest(bytes));
        let (_, source) =
            asset_object_location(&runtime.get_assets().get_objects_path(), &hash).unwrap();
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(source, bytes).unwrap();
        let index = || {
            serde_json::from_value::<AssetsIndex>(serde_json::json!({"virtual":true,"objects":{"nested/file":{"hash":hash,"size":bytes.len()}}})).unwrap()
        };
        let (a, b) = tokio::join!(
            reconstruct_mappings(
                index(),
                runtime.get_assets(),
                root.path().join("unused"),
                "legacy"
            ),
            reconstruct_mappings(
                index(),
                runtime.get_assets(),
                root.path().join("unused"),
                "legacy"
            )
        );
        a.unwrap();
        b.unwrap();
        let path = runtime
            .get_assets()
            .get_virtual_path()
            .join("legacy/nested/file");
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        reconstruct_mappings(
            index(),
            runtime.get_assets(),
            root.path().join("unused"),
            "legacy",
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(path).unwrap().modified().unwrap(),
            modified
        );
    }

    /// An index served over HTTP with a fresh metadata database, so tests can
    /// run the launch path (`get_meta`, `reconstruct_assets`, `get_assets_dir`).
    struct ServedIndex {
        _server: mockito::ServerGuard,
        _db_dir: tempfile::TempDir,
        db: Db,
        client: reqwest_middleware::ClientWithMiddleware,
        index: AssetIndex,
    }

    async fn served_index(id: &str, body: &[u8]) -> ServedIndex {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/index.json")
            .with_body(body)
            .create_async()
            .await;
        let db_dir = tempfile::tempdir().unwrap();
        let path = db_dir.path().join("meta.db");
        {
            let mut connection = rusqlite::Connection::open(&path).unwrap();
            carbon_repos::get_migrations()
                .0
                .to_latest(&mut connection)
                .unwrap();
        }
        let db = Db::open(&path, 1, true).unwrap();
        let client = reqwest_middleware::ClientBuilder::new(
            reqwest::Client::builder().no_proxy().build().unwrap(),
        )
        .build();
        let index = AssetIndex {
            id: id.into(),
            sha1: format!("{:x}", Sha1::digest(body)),
            size: body.len() as u32,
            total_size: 0,
            url: format!("{}/index.json", server.url()),
        };
        ServedIndex {
            _server: server,
            _db_dir: db_dir,
            db,
            client,
            index,
        }
    }

    impl ServedIndex {
        async fn reconstruct(&self, runtime: &carbon_rt_path::RuntimePath, resources: &Path) {
            reconstruct_assets(
                &self.db,
                self.client.clone(),
                &self.index,
                runtime.get_assets(),
                resources.to_path_buf(),
            )
            .await
            .unwrap();
        }

        async fn assets_dir(
            &self,
            runtime: &carbon_rt_path::RuntimePath,
            resources: &Path,
        ) -> AssetsDir {
            get_assets_dir(
                &self.db,
                self.client.clone(),
                &self.index,
                runtime.get_assets(),
                resources.to_path_buf(),
            )
            .await
            .unwrap()
        }
    }

    /// Writes `bytes` as an asset object and returns its hash.
    fn asset_object(runtime: &carbon_rt_path::RuntimePath, bytes: &[u8]) -> String {
        let hash = format!("{:x}", Sha1::digest(bytes));
        let (_, source) =
            asset_object_location(&runtime.get_assets().get_objects_path(), &hash).unwrap();
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(source, bytes).unwrap();
        hash
    }

    /// Play on a 1.7.10-style (virtual) and a modern version when the user has
    /// linked `linked` (relative to the data folder) to a folder elsewhere, as
    /// people do to share assets with another launcher or move them to another
    /// drive. 2.0.40 followed such links; every Play must keep working and the
    /// files must land in the link target.
    async fn assert_launches_through_link(link_dir: fn(&Path, &Path), linked: &str) {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let shared = root.path().join("shared");
        std::fs::create_dir_all(&shared).unwrap();
        // Native separators: mklink reads a '/' in its arguments as a switch.
        let link = linked
            .split('/')
            .fold(data.clone(), |path, part| path.join(part));
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        link_dir(&link, &shared);
        let runtime = carbon_rt_path::RuntimePath::new(data.clone());
        let assets = runtime.get_assets();
        let resources = data.join("instances/old/instance/resources");
        let sound = b"legacy sound bytes";
        let hash = asset_object(&runtime, sound);
        let legacy = serde_json::to_vec(&serde_json::json!({
            "virtual": true,
            "objects": {"minecraft/sounds/click.ogg": {"hash": hash, "size": sound.len()}}
        }))
        .unwrap();
        let modern = serde_json::to_vec(&serde_json::json!({
            "objects": {"minecraft/sounds/click.ogg": {"hash": hash, "size": sound.len()}}
        }))
        .unwrap();

        let served = served_index("legacy", &legacy).await;
        served.reconstruct(&runtime, &resources).await;
        let dir = served.assets_dir(&runtime, &resources).await;
        assert!(matches!(dir, AssetsDir::Virtual(_)), "{linked}");
        assert_eq!(dir.to_path_buf(), assets.get_virtual_path().join("legacy"));
        let served = served_index("17", &modern).await;
        served.reconstruct(&runtime, &resources).await;
        let dir = served.assets_dir(&runtime, &resources).await;
        assert!(matches!(dir, AssetsDir::Index(_)), "{linked}");

        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the user's link at {linked} was replaced"
        );
        for (path, bytes) in [
            (assets.get_indexes_path().join("legacy.json"), &legacy[..]),
            (assets.get_indexes_path().join("17.json"), &modern[..]),
            (
                assets
                    .get_virtual_path()
                    .join("legacy/minecraft/sounds/click.ogg"),
                &sound[..],
            ),
        ] {
            assert_eq!(std::fs::read(&path).unwrap(), bytes, "{}", path.display());
            if let Ok(below) = path.strip_prefix(&link) {
                assert_eq!(
                    std::fs::read(shared.join(below)).unwrap(),
                    bytes,
                    "{} did not land in the link target",
                    path.display()
                );
            }
        }
    }

    #[cfg(unix)]
    fn symlink_dir(link: &Path, target: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn task18_symlinked_asset_folders_keep_every_version_launching() {
        for linked in ["assets", "assets/indexes", "assets/virtual"] {
            assert_launches_through_link(symlink_dir, linked).await;
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn task18_symlinked_instances_folder_maps_pre_1_6_resources() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let elsewhere = root.path().join("elsewhere");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, data.join("instances")).unwrap();
        let runtime = carbon_rt_path::RuntimePath::new(data.clone());
        let bytes = b"pre-1.6 click";
        let hash = asset_object(&runtime, bytes);
        let index: AssetsIndex = serde_json::from_value(serde_json::json!({
            "map_to_resources": true,
            "objects": {"sound3/random/click.ogg": {"hash": hash, "size": bytes.len()}}
        }))
        .unwrap();
        reconstruct_mappings(
            index,
            runtime.get_assets(),
            data.join("instances/old/instance/resources"),
            "pre-1.6",
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read(elsewhere.join("old/instance/resources/sound3/random/click.ogg"))
                .unwrap(),
            bytes
        );
    }

    /// The resources mapping only adds missing files, so a folder the user
    /// linked inside `resources` is followed, as in 2.0.40, and their files in
    /// it are never replaced.
    #[cfg(unix)]
    #[tokio::test]
    async fn task18_linked_folder_inside_resources_gets_missing_files_only() {
        let root = tempfile::tempdir().unwrap();
        let runtime = carbon_rt_path::RuntimePath::new(root.path().join("data"));
        let resources = root.path().join("data/instances/old/instance/resources");
        let music = root.path().join("my-music");
        std::fs::create_dir_all(&resources).unwrap();
        std::fs::create_dir_all(&music).unwrap();
        std::fs::write(music.join("calm1.ogg"), b"user replacement").unwrap();
        std::os::unix::fs::symlink(&music, resources.join("music")).unwrap();
        let bytes = b"default music";
        let hash = asset_object(&runtime, bytes);
        let index: AssetsIndex = serde_json::from_value(serde_json::json!({
            "map_to_resources": true,
            "objects": {
                "music/calm1.ogg": {"hash": hash, "size": bytes.len()},
                "music/calm2.ogg": {"hash": hash, "size": bytes.len()}
            }
        }))
        .unwrap();
        reconstruct_mappings(index, runtime.get_assets(), resources.clone(), "pre-1.6")
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(music.join("calm1.ogg")).unwrap(),
            b"user replacement"
        );
        assert_eq!(std::fs::read(music.join("calm2.ogg")).unwrap(), bytes);
        assert!(
            std::fs::symlink_metadata(resources.join("music"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    /// A link in place of a mapped file (or a leftover) is replaced or removed
    /// as a link: its target is never written or deleted.
    #[cfg(unix)]
    #[tokio::test]
    async fn task18_virtual_file_links_are_replaced_not_written_through() {
        let root = tempfile::tempdir().unwrap();
        let (runtime, index) = boundary_fixture(root.path(), "test.txt");
        let target = runtime.get_assets().get_virtual_path().join("legacy");
        std::fs::create_dir_all(&target).unwrap();
        let outside = root.path().join("outside.txt");
        std::fs::write(&outside, b"outside bytes").unwrap();
        std::os::unix::fs::symlink(&outside, target.join("test.txt")).unwrap();
        let kept = root.path().join("kept.txt");
        std::fs::write(&kept, b"kept bytes").unwrap();
        std::os::unix::fs::symlink(&kept, target.join("leftover.txt")).unwrap();

        reconstruct_mappings(
            index.clone(),
            runtime.get_assets(),
            root.path().join("resources"),
            "legacy",
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&outside).unwrap(), b"outside bytes");
        let mapped = std::fs::symlink_metadata(target.join("test.txt")).unwrap();
        assert!(mapped.file_type().is_file());
        assert_eq!(
            std::fs::read(target.join("test.txt")).unwrap(),
            b"verified asset"
        );
        assert!(std::fs::symlink_metadata(target.join("leftover.txt")).is_err());
        assert_eq!(std::fs::read(&kept).unwrap(), b"kept bytes");

        // A link to a file that already has the right bytes is left alone.
        let good = root.path().join("good.txt");
        std::fs::write(&good, b"verified asset").unwrap();
        std::fs::remove_file(target.join("test.txt")).unwrap();
        std::os::unix::fs::symlink(&good, target.join("test.txt")).unwrap();
        reconstruct_mappings(
            index,
            runtime.get_assets(),
            root.path().join("resources"),
            "legacy",
        )
        .await
        .unwrap();
        assert!(
            std::fs::symlink_metadata(target.join("test.txt"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    /// The one refusal left says which link is in the way and how to clear it.
    #[cfg(unix)]
    #[tokio::test]
    async fn task18_linked_folder_below_virtual_mapping_names_the_way_out() {
        let root = tempfile::tempdir().unwrap();
        let (runtime, index) = boundary_fixture(root.path(), "linked/outside.txt");
        let target = runtime.get_assets().get_virtual_path().join("legacy");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, target.join("linked")).unwrap();
        let error = reconstruct_mappings(
            index,
            runtime.get_assets(),
            root.path().join("resources"),
            "legacy",
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(
            error.contains(&target.join("linked").display().to_string()),
            "{error}"
        );
        assert!(error.contains("Remove that link"), "{error}");
        assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
    }

    /// Leftover cleanup in the virtual mapping logs and keeps what it cannot
    /// classify instead of failing the launch: a linked folder no index key
    /// passes through stays, and a leftover with a non-UTF-8 name is removed
    /// as 2.0.40 did.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn task18_virtual_leftover_cleanup_never_fails_the_launch() {
        use std::os::unix::ffi::OsStrExt;
        let root = tempfile::tempdir().unwrap();
        let (runtime, index) = boundary_fixture(root.path(), "test.txt");
        let target = runtime.get_assets().get_virtual_path().join("legacy");
        std::fs::create_dir_all(&target).unwrap();
        let odd = target.join(std::ffi::OsStr::from_bytes(b"leftover-\xff.ogg"));
        std::fs::write(&odd, b"stale").unwrap();
        let shared = root.path().join("shared");
        std::fs::create_dir(&shared).unwrap();
        std::fs::write(shared.join("keep.ogg"), b"user bytes").unwrap();
        std::os::unix::fs::symlink(&shared, target.join("shared")).unwrap();
        reconstruct_mappings(
            index,
            runtime.get_assets(),
            root.path().join("resources"),
            "legacy",
        )
        .await
        .unwrap();
        assert!(std::fs::symlink_metadata(&odd).is_err());
        assert_eq!(
            std::fs::read(target.join("test.txt")).unwrap(),
            b"verified asset"
        );
        assert!(
            std::fs::symlink_metadata(target.join("shared"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            std::fs::read(shared.join("keep.ogg")).unwrap(),
            b"user bytes"
        );
    }

    #[cfg(windows)]
    fn junction(link: &Path, target: &Path) {
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed for {}", link.display());
        assert!(
            std::fs::symlink_metadata(link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn task18_windows_junctioned_asset_folders_keep_every_version_launching() {
        for linked in ["assets", "assets/indexes", "assets/virtual"] {
            assert_launches_through_link(junction, linked).await;
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn task18_windows_junction_below_virtual_mapping_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let (runtime, index) = boundary_fixture(root.path(), "linked/outside.txt");
        let target = runtime.get_assets().get_virtual_path().join("legacy");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("outside.txt"), b"outside bytes").unwrap();
        junction(&target.join("linked"), &outside);
        let error = reconstruct_mappings(
            index,
            runtime.get_assets(),
            root.path().join("resources"),
            "legacy",
        )
        .await
        .unwrap_err()
        .to_string();
        assert_eq!(
            std::fs::read(outside.join("outside.txt")).unwrap(),
            b"outside bytes"
        );
        assert!(error.contains("symbolic link or junction"), "{error}");
    }

    /// `compact /exe` (CompactGUI) marks files with the reparse-point
    /// attribute without making them links. Returns false when this volume
    /// cannot WOF-compress `path`.
    #[cfg(windows)]
    fn wof_compress(path: &Path) -> bool {
        use std::os::windows::fs::MetadataExt;
        let compressed = std::process::Command::new("compact")
            .arg("/c")
            .arg("/exe:xpress4k")
            .arg(path)
            .stdout(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        compressed
            && std::fs::symlink_metadata(path)
                .is_ok_and(|metadata| metadata.file_attributes() & 0x400 != 0)
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn task18_windows_wof_compressed_assets_keep_launching() {
        let root = tempfile::tempdir().unwrap();
        let runtime = carbon_rt_path::RuntimePath::new(root.path().join("data"));
        let assets = runtime.get_assets();
        let resources = root.path().join("data").join("resources");
        // Large and repetitive, so WOF actually compresses them.
        let lang = "menu.play=Play\n".repeat(8 * 1024).into_bytes();
        let hash = asset_object(&runtime, &lang);
        let body = serde_json::to_vec(&serde_json::json!({
            "virtual": true,
            "padding": "x".repeat(64 * 1024),
            "objects": {"minecraft/lang/en_US.lang": {"hash": hash, "size": lang.len()}}
        }))
        .unwrap();
        let served = served_index("legacy", &body).await;
        served.reconstruct(&runtime, &resources).await;
        let index_file = assets.get_indexes_path().join("legacy.json");
        // Native separators: compact reads a '/' in its arguments as a switch.
        let mapped = ["legacy", "minecraft", "lang", "en_US.lang"]
            .iter()
            .fold(assets.get_virtual_path(), |path, part| path.join(part));
        if !(wof_compress(&index_file) && wof_compress(&mapped)) {
            eprintln!("WOF compression is not available on this volume; nothing to check");
            return;
        }
        served.reconstruct(&runtime, &resources).await;
        assert!(matches!(
            served.assets_dir(&runtime, &resources).await,
            AssetsDir::Virtual(_)
        ));
        assert_eq!(std::fs::read(index_file).unwrap(), body);
        assert_eq!(std::fs::read(mapped).unwrap(), lang);
    }
}
