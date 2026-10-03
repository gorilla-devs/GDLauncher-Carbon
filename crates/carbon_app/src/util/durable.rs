//! Flushes files and folders to their drive on every volume a data folder
//! can live on.
//!
//! - Apple platforms: std's `File::sync_all` is `fcntl(F_FULLFSYNC)` with no
//!   fallback, and the macOS SMB and WebDAV clients refuse that command with
//!   ENOTSUP, so every sync in a data folder on a network share failed
//!   (golang/go#64215, JDK-8232861). [`sync_file`] falls back to a plain
//!   `fsync` there, as SQLite, libuv and Go do.
//! - Folder syncs make a new name in a folder durable, and some filesystems
//!   do not do them: VirtualBox shared folders answer EINVAL, network and
//!   FUSE mounts ENOTSUP, ENOTTY or ENOSYS. [`sync_dir`] takes such an answer
//!   as done, as SQLite ignores folder sync errors; any other error still
//!   fails. Windows cannot open a folder for a sync, so there it does
//!   nothing, as before.
//! - [`sync_file_plain`] and [`sync_dir_plain`] skip F_FULLFSYNC's flush of
//!   the whole drive cache on Apple platforms and use a plain `fsync`. That
//!   flush only guards against a power cut, so files that only have to
//!   outlast the launcher, not the computer, need not pay for it. Windows
//!   and Linux have no lighter flush in std that still reaches the drive, so
//!   there they flush as [`sync_file`] does (FlushFileBuffers, `fsync`), at
//!   the same cost as a plain `sync_all`.
//! - [`sync_tokio_file`] and [`sync_dir_async`] do the same for async code,
//!   on the blocking pool as tokio's own `sync_all` does.
//! - [`write_file_atomic`] saves the data folder's own files (`instance.json`,
//!   `server.json`) through [`sync_tokio_file`].

use std::fs::File;
use std::io;
use std::path::Path;

/// Flushes `file`'s data and metadata to its drive, through the drive's own
/// write cache on Apple platforms (F_FULLFSYNC) when the volume supports it.
pub fn sync_file(file: &File) -> io::Result<()> {
    #[cfg(all(test, unix))]
    if test_hooks::file_synced(file) {
        return full_or_plain(
            || Err(io::Error::from_raw_os_error(libc::ENOTSUP)),
            || plain_fsync(file),
        );
    }
    if cfg!(target_vendor = "apple") {
        full_or_plain(|| file.sync_all(), || plain_fsync(file))
    } else {
        file.sync_all()
    }
}

/// [`sync_file`] with a plain `fsync` on Apple platforms: the data reaches
/// the drive, which may still hold it in its cache through a power cut.
pub fn sync_file_plain(file: &File) -> io::Result<()> {
    #[cfg(all(test, unix))]
    test_hooks::file_synced(file);
    if cfg!(target_vendor = "apple") {
        plain_fsync(file)
    } else {
        file.sync_all()
    }
}

/// [`sync_file`] for a tokio file, once the writes it still has in flight
/// are done: an error one of them met is the answer.
pub async fn sync_tokio_file(file: &mut tokio::fs::File) -> io::Result<()> {
    use tokio::io::AsyncWriteExt;
    file.flush().await?;
    let file = file.try_clone().await?.into_std().await;
    tokio::task::spawn_blocking(move || sync_file(&file))
        .await
        .map_err(io::Error::other)?
}

/// [`sync_dir`] for async code.
pub async fn sync_dir_async(dir: &Path) -> io::Result<()> {
    if cfg!(not(unix)) {
        return Ok(());
    }
    let dir = dir.to_owned();
    tokio::task::spawn_blocking(move || sync_dir(&dir))
        .await
        .map_err(io::Error::other)?
}

/// Makes the entries of `dir` durable with [`sync_file`]. A volume that does
/// not sync folders leaves them to the system.
pub fn sync_dir(dir: &Path) -> io::Result<()> {
    sync_dir_with(dir, sync_file)
}

/// [`sync_dir`] with [`sync_file_plain`].
pub fn sync_dir_plain(dir: &Path) -> io::Result<()> {
    sync_dir_with(dir, sync_file_plain)
}

#[cfg(unix)]
fn sync_dir_with(dir: &Path, sync: fn(&File) -> io::Result<()>) -> io::Result<()> {
    let folder = File::open(dir)?;
    #[cfg(test)]
    let result = match test_hooks::folder_synced(dir) {
        Some(fault) => Err(fault),
        None => sync(&folder),
    };
    #[cfg(not(test))]
    let result = sync(&folder);
    match result {
        Err(error) if folder_sync_unsupported(&error) => {
            tracing::debug!(
                "{} does not support folder syncs ({error}); its entries are left to the system",
                dir.display()
            );
            Ok(())
        }
        result => result,
    }
}

#[cfg(not(unix))]
fn sync_dir_with(_dir: &Path, _sync: fn(&File) -> io::Result<()>) -> io::Result<()> {
    Ok(())
}

/// Tries the drive-cache flush and falls back to a plain `fsync` when the
/// volume does not support it. Any other error is the answer.
fn full_or_plain(
    full: impl FnOnce() -> io::Result<()>,
    plain: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    match full() {
        Err(error) if unsupported(&error) => {
            tracing::debug!("The drive refused a full flush ({error}); using a plain fsync");
            plain()
        }
        result => result,
    }
}

/// The volume says it does not do this kind of sync, rather than that the
/// data did not reach the drive.
fn unsupported(error: &io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error().is_some_and(|code| {
            [
                libc::EINVAL,
                libc::ENOTSUP,
                libc::EOPNOTSUPP,
                libc::ENOTTY,
                libc::ENOSYS,
            ]
            .contains(&code)
        })
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

/// [`unsupported`], or a folder descriptor the system will not sync (EBADF
/// on some systems for a folder opened read-only).
#[cfg(unix)]
fn folder_sync_unsupported(error: &io::Error) -> bool {
    unsupported(error) || error.raw_os_error() == Some(libc::EBADF)
}

#[cfg(unix)]
fn plain_fsync(file: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    loop {
        // SAFETY: `file` owns the descriptor and outlives the call.
        if unsafe { libc::fsync(file.as_raw_fd()) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(not(unix))]
fn plain_fsync(file: &File) -> io::Result<()> {
    file.sync_all()
}

/// Lets tests see which folders and files were synced here, on whichever
/// thread, and make folder syncs fail.
#[cfg(test)]
pub(crate) mod test_hooks {
    // Folder syncs only happen on unix, and files are told apart by inode.
    #![cfg_attr(not(unix), allow(dead_code))]
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, MutexGuard, PoisonError};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn lock<T>(hooks: &'static Mutex<T>) -> MutexGuard<'static, T> {
        hooks.lock().unwrap_or_else(PoisonError::into_inner)
    }

    struct FolderHook {
        root: PathBuf,
        synced: Vec<PathBuf>,
        fault: i32,
    }

    static FOLDERS: Mutex<Vec<(usize, FolderHook)>> = Mutex::new(Vec::new());

    /// Until dropped, every folder sync at or below a root fails with an
    /// error instead of syncing, and is noted. Folders elsewhere, such as
    /// those of tests running alongside, are left alone.
    pub(crate) struct FolderSyncs(usize);

    impl FolderSyncs {
        /// Every folder sync at or below `root` fails with `errno`.
        pub(crate) fn failing_with(root: &Path, errno: i32) -> Self {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            lock(&FOLDERS).push((
                id,
                FolderHook {
                    root: root.to_owned(),
                    synced: Vec::new(),
                    fault: errno,
                },
            ));
            Self(id)
        }
        pub(crate) fn synced(&self) -> Vec<PathBuf> {
            lock(&FOLDERS)
                .iter()
                .find(|(id, _)| *id == self.0)
                .map(|(_, hook)| hook.synced.clone())
                .unwrap_or_default()
        }
    }

    impl Drop for FolderSyncs {
        fn drop(&mut self) {
            lock(&FOLDERS).retain(|(id, _)| *id != self.0);
        }
    }

    /// Notes a folder sync, and returns the error a test asked for.
    pub(super) fn folder_synced(dir: &Path) -> Option<std::io::Error> {
        let mut fault = None;
        for (_, hook) in lock(&FOLDERS)
            .iter_mut()
            .filter(|(_, hook)| dir.starts_with(&hook.root))
        {
            hook.synced.push(dir.to_owned());
            fault.get_or_insert(hook.fault);
        }
        fault.map(std::io::Error::from_raw_os_error)
    }

    /// Files synced while a [`FileSyncs`] lives, by device, inode and birth
    /// time: a file a test running alongside synced and then deleted frees
    /// its inode for the next new file, which was never synced.
    type Synced = std::collections::HashSet<(u64, u64, Option<std::time::SystemTime>)>;

    #[cfg(unix)]
    fn key(metadata: &std::fs::Metadata) -> (u64, u64, Option<std::time::SystemTime>) {
        use std::os::unix::fs::MetadataExt;
        (metadata.dev(), metadata.ino(), metadata.created().ok())
    }

    static FILES: Mutex<Vec<(usize, Synced)>> = Mutex::new(Vec::new());

    /// Until dropped, notes every file synced through this module, on any
    /// thread, and stands in for a drive that refuses F_FULLFSYNC as the
    /// macOS SMB client does: [`super::sync_file`] gets ENOTSUP for the full
    /// flush on every platform and falls back to a plain `fsync`. Tests
    /// running alongside get that plain `fsync` too, which syncs their files
    /// all the same.
    #[cfg(unix)]
    pub(crate) struct FileSyncs(usize);

    #[cfg(unix)]
    impl FileSyncs {
        pub(crate) fn refusing_full_flush() -> Self {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            lock(&FILES).push((id, Synced::new()));
            Self(id)
        }
        /// Whether the file now at `path` was synced through this module.
        pub(crate) fn synced(&self, path: &Path) -> bool {
            let key = key(&std::fs::metadata(path).unwrap());
            lock(&FILES)
                .iter()
                .find(|(id, _)| *id == self.0)
                .is_some_and(|(_, synced)| synced.contains(&key))
        }
    }

    #[cfg(unix)]
    impl Drop for FileSyncs {
        fn drop(&mut self) {
            lock(&FILES).retain(|(id, _)| *id != self.0);
        }
    }

    /// Notes a file sync. True while a test stands in for a drive that
    /// refuses a full flush.
    #[cfg(unix)]
    pub(super) fn file_synced(file: &std::fs::File) -> bool {
        let mut files = lock(&FILES);
        if files.is_empty() {
            return false;
        }
        if let Ok(metadata) = file.metadata() {
            for (_, synced) in files.iter_mut() {
                synced.insert(key(&metadata));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn os_error(code: i32) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(code))
    }

    /// The macOS SMB client refuses F_FULLFSYNC with ENOTSUP: the plain
    /// fsync it still supports is used instead, and its answer counts.
    #[cfg(unix)]
    #[test]
    fn a_refused_full_flush_falls_back_to_a_plain_fsync() {
        for refusal in [libc::ENOTSUP, libc::EOPNOTSUPP, libc::ENOTTY, libc::EINVAL] {
            let plain = Cell::new(false);
            full_or_plain(
                || os_error(refusal),
                || {
                    plain.set(true);
                    Ok(())
                },
            )
            .unwrap();
            assert!(plain.get(), "errno {refusal}");
        }
        let error = full_or_plain(|| os_error(libc::ENOTSUP), || os_error(libc::EIO)).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EIO));
    }

    /// A real write error is not hidden behind a plain fsync that might
    /// report success.
    #[cfg(unix)]
    #[test]
    fn a_failed_full_flush_is_not_retried() {
        for failure in [libc::EIO, libc::ENOSPC, libc::EDQUOT] {
            let error = full_or_plain(
                || os_error(failure),
                || panic!("errno {failure} must not fall back"),
            )
            .unwrap_err();
            assert_eq!(error.raw_os_error(), Some(failure));
        }
        full_or_plain(|| Ok(()), || panic!("a supported flush is enough")).unwrap();
    }

    /// VirtualBox shared folders answer a folder sync with EINVAL, network
    /// and FUSE mounts with ENOTSUP or ENOTTY: the folder's entries are left
    /// to the system. A real I/O error still fails.
    #[cfg(unix)]
    #[test]
    fn a_folder_sync_the_volume_does_not_support_is_done() {
        let dir = tempfile::tempdir().unwrap();
        for refusal in [
            libc::EINVAL,
            libc::ENOTSUP,
            libc::EOPNOTSUPP,
            libc::ENOTTY,
            libc::ENOSYS,
            libc::EBADF,
        ] {
            let syncs = test_hooks::FolderSyncs::failing_with(dir.path(), refusal);
            sync_dir(dir.path()).unwrap();
            sync_dir_plain(dir.path()).unwrap();
            assert_eq!(syncs.synced().len(), 2, "errno {refusal}");
        }
        let _syncs = test_hooks::FolderSyncs::failing_with(dir.path(), libc::EIO);
        assert_eq!(
            sync_dir(dir.path()).unwrap_err().raw_os_error(),
            Some(libc::EIO)
        );
        assert_eq!(
            sync_dir_plain(dir.path()).unwrap_err().raw_os_error(),
            Some(libc::EIO)
        );
    }

    /// Async code gets the same answers: a folder the volume does not sync
    /// is done, a real error fails, and a tokio file is synced here once
    /// its pending writes are done, falling back to a plain fsync where the
    /// drive refuses a full flush.
    #[cfg(unix)]
    #[tokio::test]
    async fn async_callers_sync_the_same_way() {
        use tokio::io::AsyncWriteExt;
        let dir = tempfile::tempdir().unwrap();
        {
            let syncs = test_hooks::FolderSyncs::failing_with(dir.path(), libc::EINVAL);
            sync_dir_async(dir.path()).await.unwrap();
            assert_eq!(syncs.synced(), [dir.path()]);
        }
        {
            let _syncs = test_hooks::FolderSyncs::failing_with(dir.path(), libc::EIO);
            assert_eq!(
                sync_dir_async(dir.path()).await.unwrap_err().raw_os_error(),
                Some(libc::EIO)
            );
        }
        sync_dir_async(dir.path()).await.unwrap();

        let files = test_hooks::FileSyncs::refusing_full_flush();
        let path = dir.path().join("file");
        let mut file = tokio::fs::File::create(&path).await.unwrap();
        file.write_all(b"data").await.unwrap();
        sync_tokio_file(&mut file).await.unwrap();
        assert!(files.synced(&path));
        file.write_all(b" more").await.unwrap();
        sync_tokio_file(&mut file).await.unwrap();
        drop(file);
        assert_eq!(std::fs::read(&path).unwrap(), b"data more");
    }

    /// A file saved in the data folder on a drive that refuses a full flush
    /// replaces the old one, flushed here, and leaves nothing in temp.
    #[cfg(unix)]
    #[tokio::test]
    async fn an_atomic_write_lands_where_the_drive_refuses_a_full_flush() {
        let dir = tempfile::tempdir().unwrap();
        let temp = carbon_rt_path::TempPath::new(dir.path().join("temp"));
        let path = dir.path().join("instance.json");
        std::fs::write(&path, b"old").unwrap();
        let files = test_hooks::FileSyncs::refusing_full_flush();
        write_file_atomic(&temp, &path, b"new").await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(files.synced(&path));
        assert_eq!(std::fs::read_dir(temp.to_path()).unwrap().count(), 0);
    }

    #[test]
    fn real_files_and_folders_sync() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file");
        std::fs::write(&path, b"data").unwrap();
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        sync_file(&file).unwrap();
        sync_file_plain(&file).unwrap();
        plain_fsync(&file).unwrap();
        sync_dir(dir.path()).unwrap();
        sync_dir_plain(dir.path()).unwrap();
        // A folder that cannot be opened is still an error.
        #[cfg(unix)]
        assert!(sync_dir(&dir.path().join("missing")).is_err());
    }
}
