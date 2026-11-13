//! Moves what the user deletes to the Recycle Bin, as the "Deletion through
//! Recycle Bin" setting asks, so it can be taken back out.

use anyhow::anyhow;
use std::path::{Path, PathBuf};

/// How a user's Delete takes a file or folder away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Removal {
    /// Erased.
    Permanent,
    /// Moved to the Recycle Bin.
    RecycleBin,
}

impl Removal {
    /// What the "Deletion through Recycle Bin" setting asks for.
    pub(crate) fn from_setting(deletion_through_recycle_bin: bool) -> Self {
        if deletion_through_recycle_bin {
            Self::RecycleBin
        } else {
            Self::Permanent
        }
    }
}

/// Tells the user the Recycle Bin refused `path`, that it is still there, and
/// how to delete it without the Recycle Bin.
fn refusal(path: &Path, error: &anyhow::Error) -> anyhow::Error {
    anyhow!(
        "Nothing was deleted, because {} could not be moved to the Recycle Bin: {error}. To delete it for good instead, turn off \"Deletion through Recycle Bin\" in the General settings and delete it again.",
        path.display()
    )
}

fn is_gone(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
}

/// Moves `path` to the Recycle Bin with everything in it. One already gone
/// is fine.
pub(crate) async fn move_to_recycle_bin(path: PathBuf) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || {
        if is_gone(&path) {
            return Ok(());
        }
        send_to_bin(std::slice::from_ref(&path)).map_err(|error| refusal(&path, &error))
    })
    .await?
}

/// Moves every one of `paths` to the Recycle Bin in one request, which is one
/// Finder operation on macOS instead of one each. When the request fails some
/// of them may have been moved and others not: the result has, in the order
/// of `paths`, the error for each one still in its place.
pub(crate) async fn move_all_to_recycle_bin(paths: Vec<PathBuf>) -> Vec<anyhow::Result<()>> {
    let count = paths.len();
    let moved = tokio::task::spawn_blocking(move || {
        let present: Vec<PathBuf> = paths
            .iter()
            .filter(|path| !is_gone(path))
            .cloned()
            .collect();
        if present.is_empty() {
            return paths.iter().map(|_| Ok(())).collect();
        }
        match send_to_bin(&present) {
            Ok(()) => paths.iter().map(|_| Ok(())).collect::<Vec<_>>(),
            Err(error) => paths
                .iter()
                .map(|path| {
                    if is_gone(path) {
                        Ok(())
                    } else {
                        Err(refusal(path, &error))
                    }
                })
                .collect(),
        }
    })
    .await;
    match moved {
        Ok(results) => results,
        Err(error) => (0..count)
            .map(|_| Err(anyhow!("Nothing was deleted: {error}")))
            .collect(),
    }
}

#[cfg(not(test))]
fn send_to_bin(paths: &[PathBuf]) -> anyhow::Result<()> {
    Ok(trash::delete_all(paths)?)
}

/// Where tests find what was deleted: a folder two levels above it (beside
/// `servers` for a server, beside `mods` or `saves` for an addon), never the
/// Recycle Bin of whoever runs them. A file in that place makes the Recycle
/// Bin refuse.
#[cfg(test)]
pub(crate) fn bin_under_test(path: &Path) -> PathBuf {
    path.parent()
        .and_then(Path::parent)
        .expect("what tests delete is two folders deep")
        .join("recycle-bin-under-test")
}

#[cfg(test)]
fn send_to_bin(paths: &[PathBuf]) -> anyhow::Result<()> {
    for path in paths {
        let bin = bin_under_test(path);
        std::fs::create_dir_all(&bin)?;
        let name = path.file_name().expect("a deleted entry has a name");
        std::fs::rename(path, bin.join(name))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn moves_one_entry_and_is_fine_with_one_already_gone() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mods").join("a.jar");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"a").unwrap();

        move_to_recycle_bin(file.clone()).await.unwrap();
        assert!(!file.exists());
        assert_eq!(
            std::fs::read(bin_under_test(&file).join("a.jar")).unwrap(),
            b"a"
        );
        move_to_recycle_bin(file).await.unwrap();
    }

    #[tokio::test]
    async fn a_refusal_says_nothing_was_deleted_and_how_to_delete_for_good() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mods").join("a.jar");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"a").unwrap();
        std::fs::write(bin_under_test(&file), b"").unwrap();

        let error = move_to_recycle_bin(file.clone())
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("Nothing was deleted"), "{error}");
        assert!(error.contains("a.jar"), "{error}");
        assert!(error.contains("turn off"), "{error}");
        assert!(file.exists());
    }

    /// A request that fails part way reports each entry by where it is now.
    #[tokio::test]
    async fn a_request_that_fails_part_way_reports_each_entry_by_itself() {
        let dir = tempfile::tempdir().unwrap();
        let moved = dir.path().join("mods").join("a.jar");
        let gone = dir.path().join("mods").join("gone.jar");
        // Two levels below another folder, whose Recycle Bin refuses.
        let refused = dir.path().join("other").join("mods").join("b.jar");
        for file in [&moved, &refused] {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, b"x").unwrap();
        }
        std::fs::write(bin_under_test(&refused), b"").unwrap();

        let results =
            move_all_to_recycle_bin(vec![moved.clone(), gone.clone(), refused.clone()]).await;

        assert!(results[0].is_ok() && !moved.exists());
        assert!(results[1].is_ok());
        let error = results[2].as_ref().unwrap_err().to_string();
        assert!(error.contains("b.jar"), "{error}");
        assert!(refused.exists());

        let all = move_all_to_recycle_bin(vec![gone]).await;
        assert!(all.iter().all(Result::is_ok));
    }
}
