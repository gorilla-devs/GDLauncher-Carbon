//! Moves a deleted server's folder to the Recycle Bin, as the "Deletion
//! through Recycle Bin" setting asks, so its world can be taken back out.

use anyhow::anyhow;
use std::path::{Path, PathBuf};

/// Moves `root` to the Recycle Bin with everything in it, its `server.json`
/// too: a folder put back from there is listed again on the next start. A
/// folder already gone is fine. The error tells the user nothing was deleted
/// and how to delete the server without the Recycle Bin.
pub(super) async fn move_to_recycle_bin(root: PathBuf) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || {
        if std::fs::symlink_metadata(&root)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            return Ok(());
        }
        send_to_bin(&root).map_err(|error| {
            anyhow!(
                "Nothing was deleted, because {} could not be moved to the Recycle Bin: {error}. To delete the server for good instead, turn off \"Deletion through Recycle Bin\" in the General settings and delete it again.",
                root.display()
            )
        })
    })
    .await?
}

#[cfg(not(test))]
fn send_to_bin(root: &Path) -> anyhow::Result<()> {
    Ok(trash::delete(root)?)
}

/// Where tests find what was deleted: a folder beside `servers`, never the
/// Recycle Bin of whoever runs them.
#[cfg(test)]
pub(super) fn bin_under_test(root: &Path) -> PathBuf {
    root.parent()
        .and_then(Path::parent)
        .expect("a server folder is inside the data folder")
        .join("recycle-bin-under-test")
}

#[cfg(test)]
fn send_to_bin(root: &Path) -> anyhow::Result<()> {
    let bin = bin_under_test(root);
    std::fs::create_dir_all(&bin)?;
    let name = root.file_name().expect("a server folder has a name");
    Ok(std::fs::rename(root, bin.join(name))?)
}
