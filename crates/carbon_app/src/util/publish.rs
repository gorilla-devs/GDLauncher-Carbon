//! Publishes complete private files at their final path.
//!
//! Every publication in the core goes through here, so three gaps in
//! tempfile's rename-based persist are handled in one place:
//!
//! - Windows: tempfile passes plain paths to `SetFileAttributesW` and
//!   `MoveFileExW`. The core has no `longPathAware` manifest, so those calls
//!   stop at MAX_PATH (259 characters), while std, which 2.0.40 used for
//!   these moves, adds the `\\?\` prefix itself. [`long_path`] does the same
//!   before tempfile sees a path.
//! - Volumes without a no-replace rename: on macOS `renameatx_np(RENAME_EXCL)`
//!   returns ENOTSUP on exFAT, FAT, NTFS, SMB and NFS, and tempfile only
//!   falls back on EINVAL/ENOSYS; on Linux tempfile's hard-link fallback
//!   fails with EPERM or ENOSYS where links are missing too (vboxsf, older
//!   FUSE). [`persist_noclobber`] and [`move_noclobber`] then check the
//!   destination, hard-link, and as a last resort rename.
//! - Moves across volumes (an instances or mods folder junctioned or linked
//!   to another drive): [`move_noclobber`] copies into a private file next to
//!   the destination and publishes that copy without replacing anything.
//!   Async callers use [`move_noclobber_async`] and
//!   [`persist_noclobber_async`], which keep that copy off the runtime.
//! - Windows file attributes: tempfile's persist marks the file it moves
//!   NORMAL first, and TEMPORARY again when the move fails. That suits the
//!   private files it created, but not a user's file, so [`move_noclobber`]
//!   and [`rename_noclobber`] call `MoveFileExW` themselves there, which
//!   leaves attributes alone as 2.0.40's renames did. A replacing persist
//!   refused with Access denied (a destination another program holds open
//!   with delete sharing, such as a scanner) gets std's rename, which retries
//!   with POSIX semantics.
//! - Windows folder renames: a folder can't be renamed while a file in it is
//!   open, and antivirus or the indexer opens freshly written files for a
//!   moment, so [`rename_dir`] retries that refusal briefly.
//!
//! Callers keep their original paths for records, comparisons and messages;
//! only the paths handed to tempfile change form.

use std::io;
use std::path::{Path, PathBuf};

use tempfile::{NamedTempFile, PathPersistError, TempPath};

/// Room for the name tempfile appends to a directory (`\.tmpXXXXXX`).
const TEMP_NAME_ROOM: usize = 16;

/// `path` in a form that tempfile's raw Win32 calls accept past MAX_PATH.
///
/// On Windows a path of 248 UTF-16 units or more (std's own threshold) is
/// made absolute with `GetFullPathNameW`, which turns `/` into `\`, resolves
/// `.` and `..` and strips trailing dots and spaces, and then gets the `\\?\`
/// (or `\\?\UNC\`) prefix. Shorter and already verbatim paths come back
/// unchanged, so they behave exactly as before. Other platforms always get
/// `path` back unchanged.
pub fn long_path(path: &Path) -> PathBuf {
    long_path_with_room(path, 0)
}

#[cfg(windows)]
fn long_path_with_room(path: &Path, room: usize) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if is_verbatim(&wide) || wide.len() + room < LEGACY_MAX_PATH {
        return path.to_path_buf();
    }
    // The prefix turns off all normalization, so it only ever goes on the
    // absolute form: a raw prefix would keep '/' and break every write.
    let Ok(absolute) = std::path::absolute(path) else {
        return path.to_path_buf();
    };
    let absolute: Vec<u16> = absolute.as_os_str().encode_wide().collect();
    match verbatim(&absolute) {
        Some(prefixed) => PathBuf::from(std::ffi::OsString::from_wide(&prefixed)),
        None => path.to_path_buf(),
    }
}

#[cfg(not(windows))]
fn long_path_with_room(path: &Path, _room: usize) -> PathBuf {
    path.to_path_buf()
}

/// std's threshold: below it every Win32 path API accepts a plain path.
#[cfg_attr(not(windows), allow(dead_code))]
const LEGACY_MAX_PATH: usize = 248;
const SEP: u16 = b'\\' as u16;
const QUERY: u16 = b'?' as u16;
const COLON: u16 = b':' as u16;
const DOT: u16 = b'.' as u16;

#[cfg_attr(not(windows), allow(dead_code))]
fn is_verbatim(wide: &[u16]) -> bool {
    wide.starts_with(&[SEP, SEP, QUERY, SEP]) || wide.starts_with(&[SEP, QUERY, QUERY, SEP])
}

/// The verbatim form of a normalized absolute Windows path, by std's rule:
/// `C:\x` becomes `\\?\C:\x`, `\\.\x` becomes `\\?\x`, `\\server\share\x`
/// becomes `\\?\UNC\server\share\x`, and verbatim paths stay as they are.
/// Anything else has no verbatim form.
#[cfg_attr(not(windows), allow(dead_code))]
fn verbatim(absolute: &[u16]) -> Option<Vec<u16>> {
    const VERBATIM: &[u16] = &[SEP, SEP, QUERY, SEP];
    const UNC: &[u16] = &[
        SEP,
        SEP,
        QUERY,
        SEP,
        b'U' as u16,
        b'N' as u16,
        b'C' as u16,
        SEP,
    ];
    let (prefix, rest): (&[u16], &[u16]) = match absolute {
        [_, COLON, SEP, ..] => (VERBATIM, absolute),
        [SEP, SEP, DOT, SEP, rest @ ..] => (VERBATIM, rest),
        [SEP, SEP, QUERY, SEP, ..] | [SEP, QUERY, QUERY, SEP, ..] => (&[], absolute),
        [SEP, SEP, rest @ ..] if !rest.is_empty() => (UNC, rest),
        _ => return None,
    };
    Some([prefix, rest].concat())
}

/// A private file in `dir` for [`persist`] or [`persist_noclobber`].
pub fn temp_in(dir: &Path) -> io::Result<NamedTempFile> {
    NamedTempFile::new_in(long_path_with_room(dir, TEMP_NAME_ROOM))
}

/// Publishes `file` at `destination`, atomically replacing what is there.
pub fn persist(file: NamedTempFile, destination: &Path) -> io::Result<()> {
    replace(file.into_temp_path(), destination).map_err(|(error, _)| error)
}

/// Publishes `file` at `destination` only if nothing is there yet. Otherwise
/// returns `AlreadyExists`; the private file is deleted on any failure.
pub fn persist_noclobber(file: NamedTempFile, destination: &Path) -> io::Result<()> {
    move_or_copy(file.into_temp_path(), destination)
}

/// Moves the file at `source` to `destination` only if nothing is there yet.
/// Works across volumes by copying. Any failure keeps `source` in place.
pub fn move_noclobber(source: &Path, destination: &Path) -> io::Result<()> {
    match rename_noclobber(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if crosses_devices(&error) => copy_noclobber(source, destination, error),
        Err(error) => Err(error),
    }
}

/// Renames `source` to `destination` only if nothing is there yet, on the
/// same volume only: across volumes it fails with the platform's
/// cross-device error (see [`crosses_devices`]) instead of copying. Any
/// failure keeps `source` in place, and its attributes as they were.
pub fn rename_noclobber(source: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        #[cfg(test)]
        if let Some(error) = fault::take(fault::Step::NoReplace, destination) {
            return Err(error);
        }
        win::move_no_replace(&long_path(source), &long_path(destination))
    }
    #[cfg(not(windows))]
    {
        let mut path = TempPath::from_path(long_path(source));
        path.disable_cleanup(true);
        no_replace(path, destination).map_err(|(error, _)| error)
    }
}

/// [`move_noclobber`] for async callers. A move across volumes copies and
/// syncs the whole file, so it runs on the blocking pool like every other
/// publication instead of stalling a runtime worker.
pub async fn move_noclobber_async(source: &Path, destination: &Path) -> io::Result<()> {
    let (source, destination) = (source.to_owned(), destination.to_owned());
    blocking(move || move_noclobber(&source, &destination)).await
}

/// [`persist_noclobber`] for async callers; see [`move_noclobber_async`].
pub async fn persist_noclobber_async(file: NamedTempFile, destination: &Path) -> io::Result<()> {
    let destination = destination.to_owned();
    blocking(move || persist_noclobber(file, &destination)).await
}

async fn blocking(work: impl FnOnce() -> io::Result<()> + Send + 'static) -> io::Result<()> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(io::Error::other)?
}

fn move_or_copy(path: TempPath, destination: &Path) -> io::Result<()> {
    match no_replace(path, destination) {
        Ok(()) => Ok(()),
        Err((error, path)) if crosses_devices(&error) => copy_noclobber(&path, destination, error),
        Err((error, _)) => Err(error),
    }
}

type Failed = (io::Error, TempPath);

fn replace(path: TempPath, destination: &Path) -> Result<(), Failed> {
    #[cfg(test)]
    let attempt = match fault::take(fault::Step::Replace, destination) {
        Some(error) => Err(PathPersistError { error, path }),
        None => path.persist(long_path(destination)),
    };
    #[cfg(not(test))]
    let attempt = path.persist(long_path(destination));
    match attempt {
        Ok(()) => Ok(()),
        #[cfg(windows)]
        Err(PathPersistError { error, path }) if error.raw_os_error() == Some(ACCESS_DENIED) => {
            replace_posix(path, destination, error)
        }
        Err(PathPersistError { error, path }) => Err((error, path)),
    }
}

/// ERROR_ACCESS_DENIED.
#[cfg(windows)]
const ACCESS_DENIED: i32 = 5;

/// A replacing `MoveFileExW` refused with Access denied, as it is when the
/// destination is open in another program that allows deleting it (a
/// scanner or the indexer): std's rename retries that with POSIX semantics,
/// which replaces such a file. tempfile marked the private file TEMPORARY
/// again when its move failed, so that goes first; a published record is an
/// ordinary file. Any failure here reports the original refusal.
#[cfg(windows)]
fn replace_posix(path: TempPath, destination: &Path, denied: io::Error) -> Result<(), Failed> {
    if win::set_normal(&path).is_err() {
        return Err((denied, path));
    }
    match std::fs::rename(&path, long_path(destination)) {
        Ok(()) => {
            let mut path = path;
            path.disable_cleanup(true);
            Ok(())
        }
        Err(_) => Err((denied, path)),
    }
}

/// The two Win32 calls tempfile makes for its own files, made directly so
/// that a user's file keeps its attributes.
#[cfg(windows)]
mod win {
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
        fn SetFileAttributesW(path: *const u16, attributes: u32) -> i32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`: an occupied
    /// destination fails with ERROR_ALREADY_EXISTS or ERROR_FILE_EXISTS
    /// (`AlreadyExists`), another volume with ERROR_NOT_SAME_DEVICE.
    pub(super) fn move_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
        let (source, destination) = (wide(source), wide(destination));
        // SAFETY: both are NUL-terminated UTF-16 strings that outlive the call.
        if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn set_normal(path: &Path) -> io::Result<()> {
        let path = wide(path);
        // SAFETY: `path` is a NUL-terminated UTF-16 string that outlives the call.
        if unsafe { SetFileAttributesW(path.as_ptr(), FILE_ATTRIBUTE_NORMAL) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// How long [`rename_dir`] waits before each retry of a refusal on Windows,
/// about two seconds in all.
const BUSY_RETRY_DELAYS: [std::time::Duration; 7] = {
    use std::time::Duration;
    [
        Duration::from_millis(20),
        Duration::from_millis(40),
        Duration::from_millis(80),
        Duration::from_millis(160),
        Duration::from_millis(320),
        Duration::from_millis(500),
        Duration::from_millis(800),
    ]
};

/// Renames a folder the setup owns (`.setup`, its staging or a download in
/// progress, a retired or replacement setup), as `tokio::fs::rename` does.
///
/// Windows refuses to rename a folder while a file anywhere in it is open,
/// and antivirus or the search indexer opens freshly written files for a
/// moment. A refusal there (Access denied, or a sharing violation) is tried
/// again for about two seconds, so a scan doesn't fail a setup or, before
/// its record is saved, cost a whole download. A refusal that lasts is
/// reported as it was. Other platforms rename once.
pub async fn rename_dir(from: &Path, to: &Path) -> io::Result<()> {
    let (from, to) = (from.to_owned(), to.to_owned());
    retry_while_busy(
        || {
            let (from, to) = (from.clone(), to.clone());
            async move {
                #[cfg(test)]
                if let Some(error) = fault::take(fault::Step::RenameDir, &from)
                    .or_else(|| fault::take(fault::Step::RenameDir, &to))
                {
                    return Err(error);
                }
                tokio::fs::rename(&from, &to).await
            }
        },
        held_open,
    )
    .await
}

/// Runs `attempt` again after each of [`BUSY_RETRY_DELAYS`] while it fails
/// with an error `busy` accepts, and returns the first other outcome or the
/// last refusal.
async fn retry_while_busy<F, Fut>(mut attempt: F, busy: fn(&io::Error) -> bool) -> io::Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = io::Result<()>>,
{
    for delay in BUSY_RETRY_DELAYS {
        match attempt().await {
            Err(error) if busy(&error) => {
                tracing::debug!(?error, "folder busy, renaming it again shortly");
                tokio::time::sleep(delay).await;
            }
            outcome => return outcome,
        }
    }
    attempt().await
}

/// ERROR_ACCESS_DENIED or ERROR_SHARING_VIOLATION on Windows, where a file
/// another program holds open inside the folder refuses its rename. Test
/// builds accept the same codes everywhere, so injected refusals exercise
/// the retry on every platform.
fn held_open(error: &io::Error) -> bool {
    (cfg!(windows) || cfg!(test)) && matches!(error.raw_os_error(), Some(5 | 32))
}

/// tempfile's no-replace rename (`MoveFileExW` without
/// `MOVEFILE_REPLACE_EXISTING`, `renameat2(RENAME_NOREPLACE)`, or a hard
/// link), plus the fallback for volumes that have none of them.
fn no_replace(path: TempPath, destination: &Path) -> Result<(), Failed> {
    #[cfg(test)]
    let attempt = match fault::take(fault::Step::NoReplace, destination) {
        Some(error) => Err(PathPersistError { error, path }),
        None => path.persist_noclobber(long_path(destination)),
    };
    #[cfg(not(test))]
    let attempt = path.persist_noclobber(long_path(destination));
    let PathPersistError { error, path } = match attempt {
        Ok(()) => return Ok(()),
        Err(failure) => failure,
    };
    #[cfg(unix)]
    if no_replace_unsupported(&error) {
        return link_or_rename(path, destination);
    }
    Err((error, path))
}

/// Errors that mean the volume cannot publish without replacing, rather than
/// that this publication is refused:
/// - ENOTSUP or EOPNOTSUPP (45 and 102 on Apple platforms, both 95 on
///   Linux): macOS `renameatx_np(RENAME_EXCL)` on exFAT, FAT, NTFS, SMB, NFS;
/// - EPERM, ENOSYS or EINVAL: tempfile already turned the EINVAL of an
///   unsupported `renameat2(RENAME_NOREPLACE)` into a hard link, and the
///   volume has no hard links either (vboxsf answers EPERM, FUSE without a
///   link handler ENOSYS).
///
/// [`link_or_rename`] still refuses an occupied destination first, and a
/// real permission problem fails its overwriting rename the same way, so
/// treating these as a missing capability never replaces anything.
#[cfg(unix)]
fn no_replace_unsupported(error: &io::Error) -> bool {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const CODES: &[i32] = &[95, 1, 38, 22];
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    const CODES: &[i32] = &[45, 102, 1, 78, 22];
    error
        .raw_os_error()
        .is_some_and(|code| CODES.contains(&code))
}

/// EXDEV on unix, ERROR_NOT_SAME_DEVICE on Windows.
fn crosses_devices(error: &io::Error) -> bool {
    const CODE: i32 = if cfg!(windows) { 17 } else { 18 };
    error.kind() == io::ErrorKind::CrossesDevices || error.raw_os_error() == Some(CODE)
}

/// No-replace publication on a volume that cannot rename without replacing.
/// Callers hold the instance gate (natives their cache mutex), so only
/// outside writers can race the existence check; 2.0.40's plain rename
/// accepted the same race.
#[cfg(unix)]
fn link_or_rename(path: TempPath, destination: &Path) -> Result<(), Failed> {
    if destination.symlink_metadata().is_ok() {
        return Err((io::ErrorKind::AlreadyExists.into(), path));
    }
    #[cfg(test)]
    let linked = match fault::take(fault::Step::Link, destination) {
        Some(error) => Err(error),
        None => std::fs::hard_link(&path, destination),
    };
    #[cfg(not(test))]
    let linked = std::fs::hard_link(&path, destination);
    match linked {
        Ok(()) => match remove_source(&path, destination) {
            Ok(()) => {
                let mut path = path;
                path.disable_cleanup(true);
                Ok(())
            }
            Err(error) => {
                // Keep the move all-or-nothing: withdraw the link just made.
                let _ = std::fs::remove_file(destination);
                Err((error, path))
            }
        },
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err((error, path)),
        // Most of these volumes cannot hard-link either.
        Err(_) => replace(path, destination),
    }
}

/// Moves `source` to another volume: copy into a private file next to
/// `destination`, sync it, publish it without replacing, then remove the
/// source. The source stays unless the copy was published.
fn copy_noclobber(source: &Path, destination: &Path, cross_device: io::Error) -> io::Result<()> {
    use std::io::Write;
    let metadata = std::fs::symlink_metadata(source)?;
    if !metadata.is_file() {
        return Err(cross_device);
    }
    if destination.symlink_metadata().is_ok() {
        return Err(io::ErrorKind::AlreadyExists.into());
    }
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"))?;
    let mut private = temp_in(parent)?;
    let copied = io::copy(&mut std::fs::File::open(source)?, private.as_file_mut())?;
    if copied != metadata.len() {
        return Err(io::Error::other(format!(
            "{} changed while it was copied to another drive",
            source.display()
        )));
    }
    private.flush()?;
    #[cfg(unix)]
    private.as_file().set_permissions(metadata.permissions())?;
    if let Ok(modified) = metadata.modified() {
        // A rename keeps the modification time; the copy follows suit.
        let _ = private.as_file().set_modified(modified);
    }
    // Through the helper: a macOS network share refuses std's F_FULLFSYNC.
    crate::util::durable::sync_file(private.as_file())?;
    // The copy sits next to its destination, so no further copy can help.
    no_replace(private.into_temp_path(), destination).map_err(|(error, _)| error)?;
    if let Err(error) = remove_source(source, destination) {
        // Keep the move all-or-nothing: withdraw the published copy.
        let _ = std::fs::remove_file(destination);
        return Err(error);
    }
    Ok(())
}

/// Whether `a` and `b` both exist and name one file (hard links of each
/// other, or the same path). Unix compares device and inode from a stat, so
/// nothing is opened and a FIFO cannot block; Windows needs a handle for the
/// volume serial and file index.
pub fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // `a` first: callers pass the side that is usually missing.
        let Ok(a) = std::fs::metadata(a) else {
            return false;
        };
        std::fs::metadata(b).is_ok_and(|b| a.dev() == b.dev() && a.ino() == b.ino())
    }
    #[cfg(not(unix))]
    {
        same_file::is_same_file(a, b).unwrap_or(false)
    }
}

/// Removes the source of a move whose link or copy is already published at
/// `destination`.
fn remove_source(source: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(test)]
    if let Some(error) = fault::take(fault::Step::RemoveSource, destination) {
        return Err(error);
    }
    #[cfg(not(test))]
    let _ = destination;
    std::fs::remove_file(source)
}

/// One-shot failures injected into a publication step, keyed by the
/// caller's destination path, in the style of `staging_test::io_fault`.
#[cfg(test)]
pub(crate) mod fault {
    use std::collections::{HashMap, VecDeque};
    use std::path::{Path, PathBuf};
    use std::sync::{LazyLock, Mutex};

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum Step {
        /// The platform no-replace rename.
        NoReplace,
        /// The hard link of the unix fallback.
        #[cfg(unix)]
        Link,
        /// Removing the source once its link or copy is published.
        RemoveSource,
        /// tempfile's replacing persist.
        Replace,
        /// A [`super::rename_dir`] attempt, keyed by either of its paths.
        RenameDir,
    }

    /// ERROR_SHARING_VIOLATION: another program holds a file open without
    /// delete sharing. Test builds treat it as a busy folder everywhere.
    pub(crate) const SHARING_VIOLATION: i32 = 32;

    /// ENOTSUP as the platform reports it.
    #[cfg(unix)]
    pub(crate) const ENOTSUP: i32 = if cfg!(any(target_os = "linux", target_os = "android")) {
        95
    } else {
        45
    };
    /// EXDEV, or ERROR_NOT_SAME_DEVICE on Windows.
    pub(crate) const CROSS_DEVICE: i32 = if cfg!(windows) { 17 } else { 18 };
    /// EACCES, or ERROR_SHARING_VIOLATION on Windows (another process holds
    /// the file without delete sharing).
    pub(crate) const IN_USE: i32 = if cfg!(windows) { 32 } else { 13 };

    type Faults = HashMap<(Step, PathBuf), VecDeque<i32>>;
    static FAULTS: LazyLock<Mutex<Faults>> = LazyLock::new(|| Mutex::new(HashMap::new()));

    /// The next `step` publishing at `destination` fails with OS error `code`
    /// without touching the disk. Repeated calls queue further failures.
    pub(crate) fn inject(step: Step, destination: &Path, code: i32) {
        FAULTS
            .lock()
            .unwrap()
            .entry((step, destination.to_owned()))
            .or_default()
            .push_back(code);
    }

    /// Whether an injected failure at `destination` is still waiting, i.e.
    /// the step never ran through this module.
    pub(crate) fn pending(step: Step, destination: &Path) -> bool {
        FAULTS
            .lock()
            .unwrap()
            .contains_key(&(step, destination.to_owned()))
    }

    pub(super) fn take(step: Step, destination: &Path) -> Option<std::io::Error> {
        let mut faults = FAULTS.lock().unwrap();
        let key = (step, destination.to_owned());
        let code = faults.get_mut(&key)?.pop_front();
        if faults.get(&key).is_some_and(VecDeque::is_empty) {
            faults.remove(&key);
        }
        code.map(std::io::Error::from_raw_os_error)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::fault::ENOTSUP;
    use super::fault::{CROSS_DEVICE, IN_USE, Step, inject, pending};
    use super::*;
    use std::io::Write;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn private(dir: &Path, bytes: &[u8]) -> NamedTempFile {
        let mut file = temp_in(dir).unwrap();
        file.write_all(bytes).unwrap();
        file
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".tmp"))
            .collect()
    }

    #[test]
    fn verbatim_form_follows_std_rule() {
        let cases = [
            (
                r"C:\Users\a\instances\x",
                Some(r"\\?\C:\Users\a\instances\x"),
            ),
            (r"\\server\share\x", Some(r"\\?\UNC\server\share\x")),
            (r"\\.\C:\x", Some(r"\\?\C:\x")),
            (r"\\?\C:\x", Some(r"\\?\C:\x")),
            (r"\??\C:\x", Some(r"\??\C:\x")),
            (r"relative\x", None),
            (r"\rooted\x", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                verbatim(&wide(input)),
                expected.map(wide),
                "verbatim form of {input}"
            );
        }
        assert!(is_verbatim(&wide(r"\\?\C:\x")));
        assert!(is_verbatim(&wide(r"\??\C:\x")));
        assert!(!is_verbatim(&wide(r"C:\x")));
        assert!(!is_verbatim(&wide("//?/C:/x")));
    }

    #[test]
    fn short_paths_are_unchanged() {
        let short = Path::new("instances/short/.setup/representation-transitions/key.json");
        assert_eq!(long_path(short), short);
        assert_eq!(long_path_with_room(short, TEMP_NAME_ROOM), short);
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_paths_are_unchanged() {
        let long = PathBuf::from(format!("/data/{}/../file.json", "x".repeat(300)));
        assert_eq!(long_path(&long), long);
        assert_eq!(long_path_with_room(&long, TEMP_NAME_ROOM), long);
    }

    #[cfg(windows)]
    #[test]
    fn windows_long_paths_become_normalized_verbatim_paths() {
        let deep = "d".repeat(120);
        let drive = format!(r"C:\Users\gamer\{deep}/sub\..\{deep}/.setup/file.json");
        assert_eq!(
            long_path(Path::new(&drive)),
            PathBuf::from(format!(
                r"\\?\C:\Users\gamer\{deep}\{deep}\.setup\file.json"
            ))
        );
        let unc = format!(r"\\server\share\{deep}\{deep}\file.json");
        assert_eq!(
            long_path(Path::new(&unc)),
            PathBuf::from(format!(r"\\?\UNC\server\share\{deep}\{deep}\file.json"))
        );
        let already = format!(r"\\?\C:\{deep}\{deep}\a/b");
        assert_eq!(long_path(Path::new(&already)), Path::new(&already));
        let short = r"C:\Users\gamer\x/../y.json";
        assert_eq!(long_path(Path::new(short)), Path::new(short));
        // The directory form leaves room for tempfile's own name.
        let near = format!(r"C:\{}", "n".repeat(240));
        assert_eq!(long_path(Path::new(&near)), Path::new(&near));
        assert_eq!(
            long_path_with_room(Path::new(&near), TEMP_NAME_ROOM),
            PathBuf::from(format!(r"\\?\{near}"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_publication_past_max_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mut dir = tmp.path().to_path_buf();
        while dir.as_os_str().len() < 300 {
            dir.push("x".repeat(60));
        }
        std::fs::create_dir_all(&dir).unwrap();
        let destination = dir.join("published.json");
        persist_noclobber(private(&dir, b"new"), &destination).unwrap();
        persist(private(&dir, b"replaced"), &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"replaced");
        let moved = dir.join("moved.json");
        move_noclobber(&destination, &moved).unwrap();
        assert_eq!(std::fs::read(&moved).unwrap(), b"replaced");
        assert!(leftovers(&dir).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn unsupported_no_replace_publishes_absent_and_keeps_present() {
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("record.json");
        inject(Step::NoReplace, &destination, ENOTSUP);
        persist_noclobber(private(tmp.path(), b"first"), &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"first");
        inject(Step::NoReplace, &destination, ENOTSUP);
        let error = persist_noclobber(private(tmp.path(), b"second"), &destination).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&destination).unwrap(), b"first");
        assert!(leftovers(tmp.path()).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn unsupported_no_replace_without_hard_links_renames() {
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("record.json");
        inject(Step::NoReplace, &destination, ENOTSUP);
        inject(Step::Link, &destination, 1);
        persist_noclobber(private(tmp.path(), b"renamed"), &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"renamed");
        assert!(leftovers(tmp.path()).is_empty());

        let source = tmp.path().join("staged.jar");
        let moved = tmp.path().join("mods.jar");
        std::fs::write(&source, b"jar").unwrap();
        inject(Step::NoReplace, &moved, ENOTSUP);
        inject(Step::Link, &moved, 1);
        move_noclobber(&source, &moved).unwrap();
        assert_eq!(std::fs::read(&moved).unwrap(), b"jar");
        assert!(!source.exists());
    }

    #[cfg(unix)]
    #[test]
    fn unsupported_no_replace_moves_without_replacing() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("staged.jar");
        let destination = tmp.path().join("mod.jar");
        std::fs::write(&source, b"new").unwrap();
        std::fs::write(&destination, b"user").unwrap();
        inject(Step::NoReplace, &destination, ENOTSUP);
        let error = move_noclobber(&source, &destination).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&destination).unwrap(), b"user");
        assert_eq!(std::fs::read(&source).unwrap(), b"new");

        std::fs::remove_file(&destination).unwrap();
        inject(Step::NoReplace, &destination, ENOTSUP);
        move_noclobber(&source, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"new");
        assert!(!source.exists());
    }

    /// Linux volumes with neither RENAME_NOREPLACE nor hard links (vboxsf,
    /// older FUSE, some network mounts): tempfile reports its hard link's
    /// EPERM or ENOSYS. 2.0.40's plain rename worked there, so publication
    /// does too, and still never replaces.
    #[cfg(unix)]
    #[test]
    fn volumes_without_links_still_publish_without_replacing() {
        let enosys = if cfg!(any(target_os = "linux", target_os = "android")) {
            38
        } else {
            78
        };
        for code in [1, enosys, 22] {
            let tmp = tempfile::tempdir().unwrap();
            let destination = tmp.path().join("record.json");
            inject(Step::NoReplace, &destination, code);
            inject(Step::Link, &destination, code);
            persist_noclobber(private(tmp.path(), b"first"), &destination).unwrap();
            assert_eq!(std::fs::read(&destination).unwrap(), b"first");
            assert!(!pending(Step::Link, &destination), "code {code}");

            inject(Step::NoReplace, &destination, code);
            let error =
                persist_noclobber(private(tmp.path(), b"second"), &destination).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::AlreadyExists, "code {code}");
            assert_eq!(std::fs::read(&destination).unwrap(), b"first");
            assert!(leftovers(tmp.path()).is_empty());

            let source = tmp.path().join("staged.jar");
            let moved = tmp.path().join("mods.jar");
            std::fs::write(&source, b"jar").unwrap();
            inject(Step::NoReplace, &moved, code);
            inject(Step::Link, &moved, code);
            move_noclobber(&source, &moved).unwrap();
            assert_eq!(std::fs::read(&moved).unwrap(), b"jar");
            assert!(!source.exists());
        }
    }

    /// Treating EPERM as a missing capability does not hide a real
    /// permission problem: the last-resort rename reports it.
    #[cfg(unix)]
    #[test]
    fn real_permission_failures_are_still_reported() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("locked");
        std::fs::create_dir(&dir).unwrap();
        let file = private(&dir, b"x");
        let destination = dir.join("record.json");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        inject(Step::NoReplace, &destination, 1);
        let result = persist_noclobber(file, &destination);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        assert!(!destination.exists());
    }

    #[test]
    fn other_no_replace_failures_are_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("record.json");
        inject(Step::NoReplace, &destination, 5);
        let error = persist_noclobber(private(tmp.path(), b"x"), &destination).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(5));
        assert!(!destination.exists());
        assert!(leftovers(tmp.path()).is_empty());
    }

    #[test]
    fn cross_device_move_copies_and_never_replaces() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        let mods = tmp.path().join("mods");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::create_dir_all(&mods).unwrap();
        let source = staging.join("artifact");
        let destination = mods.join("mod.jar");
        std::fs::write(&source, b"artifact").unwrap();

        // Occupied destination: nothing is replaced and the source stays.
        std::fs::write(&destination, b"user").unwrap();
        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        let error = move_noclobber(&source, &destination).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&destination).unwrap(), b"user");
        assert_eq!(std::fs::read(&source).unwrap(), b"artifact");
        std::fs::remove_file(&destination).unwrap();

        // A failed publication of the copy keeps the source.
        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        inject(Step::NoReplace, &destination, 5);
        assert_eq!(
            move_noclobber(&source, &destination)
                .unwrap_err()
                .raw_os_error(),
            Some(5)
        );
        assert!(!destination.exists());
        assert_eq!(std::fs::read(&source).unwrap(), b"artifact");
        assert!(leftovers(&mods).is_empty());

        // The source goes only after the copy is published.
        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        move_noclobber(&source, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"artifact");
        assert!(!source.exists());
        assert!(leftovers(&mods).is_empty());
    }

    /// A move onto a macOS network share, which refuses std's F_FULLFSYNC,
    /// still lands: the copy is synced through `util::durable`, which falls
    /// back to the plain fsync the share supports. The test stands in for
    /// that share on every platform.
    #[cfg(unix)]
    #[test]
    fn cross_device_move_lands_on_a_drive_that_refuses_a_full_flush() {
        let syncs = crate::util::durable::test_hooks::FileSyncs::refusing_full_flush();
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("artifact");
        let destination = tmp.path().join("mod.jar");
        std::fs::write(&source, b"artifact").unwrap();
        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        move_noclobber(&source, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"artifact");
        assert!(!source.exists());
        assert!(
            syncs.synced(&destination),
            "the copy is synced through the helper"
        );
        assert!(leftovers(tmp.path()).is_empty());
    }

    /// The datapack copy is written in the instance root and published into
    /// a world folder that may sit on another drive.
    #[test]
    fn cross_device_persist_publishes_once_and_leaves_nothing_behind() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let datapacks = tmp.path().join("datapacks");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&datapacks).unwrap();
        let destination = datapacks.join("pack.zip");

        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        persist_noclobber(private(&root, b"pack"), &destination).unwrap();
        assert!(!pending(Step::NoReplace, &destination));
        assert_eq!(std::fs::read(&destination).unwrap(), b"pack");
        assert!(leftovers(&root).is_empty());
        assert!(leftovers(&datapacks).is_empty());

        // Occupied destination: refused, nothing replaced, nothing left over.
        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        let error = persist_noclobber(private(&root, b"other"), &destination).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&destination).unwrap(), b"pack");
        assert!(leftovers(&root).is_empty());
        assert!(leftovers(&datapacks).is_empty());
    }

    /// A move stays all-or-nothing: when the source cannot be removed after
    /// its copy is published (on Windows another process may hold it without
    /// delete sharing), the copy is withdrawn and the source stays.
    #[test]
    fn cross_device_copy_is_withdrawn_when_the_source_cannot_be_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        let mods = tmp.path().join("mods");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::create_dir_all(&mods).unwrap();
        let source = staging.join("previous-addon");
        let destination = mods.join("mod.jar");
        std::fs::write(&source, b"original").unwrap();

        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        inject(Step::RemoveSource, &destination, IN_USE);
        let error = move_noclobber(&source, &destination).unwrap_err();
        assert!(!pending(Step::RemoveSource, &destination));
        assert_eq!(error.raw_os_error(), Some(IN_USE));
        assert!(!destination.exists());
        assert_eq!(std::fs::read(&source).unwrap(), b"original");
        assert!(leftovers(&mods).is_empty());
        assert!(leftovers(&staging).is_empty());

        // The private file of a persist is withdrawn the same way, then dropped.
        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        inject(Step::RemoveSource, &destination, IN_USE);
        let error = persist_noclobber(private(&staging, b"new"), &destination).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(IN_USE));
        assert!(!destination.exists());
        assert!(leftovers(&mods).is_empty());
        assert!(leftovers(&staging).is_empty());
    }

    /// The hard-link fallback withdraws its link the same way.
    #[cfg(unix)]
    #[test]
    fn hard_link_is_withdrawn_when_the_source_cannot_be_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("staged.jar");
        let destination = tmp.path().join("mod.jar");
        std::fs::write(&source, b"staged").unwrap();

        inject(Step::NoReplace, &destination, ENOTSUP);
        inject(Step::RemoveSource, &destination, IN_USE);
        let error = move_noclobber(&source, &destination).unwrap_err();
        assert!(!pending(Step::RemoveSource, &destination));
        assert_eq!(error.raw_os_error(), Some(IN_USE));
        assert!(!destination.exists());
        assert_eq!(std::fs::read(&source).unwrap(), b"staged");

        inject(Step::NoReplace, &destination, ENOTSUP);
        inject(Step::RemoveSource, &destination, IN_USE);
        let error = persist_noclobber(private(tmp.path(), b"new"), &destination).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(IN_USE));
        assert!(!destination.exists());
        assert!(leftovers(tmp.path()).is_empty());
    }

    /// Async callers get the same move, run on the blocking pool.
    #[tokio::test]
    async fn async_wrappers_publish_across_volumes() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("artifact");
        let destination = tmp.path().join("mod.jar");
        std::fs::write(&source, b"artifact").unwrap();
        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        move_noclobber_async(&source, &destination).await.unwrap();
        assert!(!pending(Step::NoReplace, &destination));
        assert_eq!(std::fs::read(&destination).unwrap(), b"artifact");
        assert!(!source.exists());

        let record = tmp.path().join("record.json");
        inject(Step::NoReplace, &record, CROSS_DEVICE);
        persist_noclobber_async(private(tmp.path(), b"record"), &record)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&record).unwrap(), b"record");
        let error = persist_noclobber_async(private(tmp.path(), b"late"), &record)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&record).unwrap(), b"record");
        assert!(leftovers(tmp.path()).is_empty());
    }

    /// The modpack apply moves a staged file only within its volume: across
    /// volumes it copies itself, with its own checks, and keeps the original.
    #[test]
    fn rename_noclobber_never_copies_and_never_replaces() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("staged.jar");
        let destination = tmp.path().join("mod.jar");
        std::fs::write(&source, b"staged").unwrap();

        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        let error = rename_noclobber(&source, &destination).unwrap_err();
        assert!(crosses_devices(&error), "{error}");
        assert_eq!(std::fs::read(&source).unwrap(), b"staged");
        assert!(!destination.exists());

        std::fs::write(&destination, b"user").unwrap();
        let error = rename_noclobber(&source, &destination).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&destination).unwrap(), b"user");
        assert_eq!(std::fs::read(&source).unwrap(), b"staged");

        std::fs::remove_file(&destination).unwrap();
        rename_noclobber(&source, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"staged");
        assert!(!source.exists());
        assert!(leftovers(tmp.path()).is_empty());
    }

    /// A folder that antivirus holds busy for a moment is renamed once it
    /// lets go; one held for longer than about two seconds fails as before,
    /// and any other failure fails at once.
    #[tokio::test(start_paused = true)]
    async fn rename_dir_waits_out_a_busy_folder() {
        use super::fault::SHARING_VIOLATION;
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("preparing-1");
        let to = tmp.path().join("staging");
        std::fs::create_dir_all(from.join("mods")).unwrap();

        for _ in 0..3 {
            inject(Step::RenameDir, &from, SHARING_VIOLATION);
        }
        rename_dir(&from, &to).await.unwrap();
        assert!(!pending(Step::RenameDir, &from));
        assert!(to.join("mods").is_dir());
        assert!(!from.exists());

        // Busy for every attempt: the last refusal is reported, the
        // folder stays where it was, and the wait is bounded.
        for _ in 0..=BUSY_RETRY_DELAYS.len() + 1 {
            inject(Step::RenameDir, &to, 5);
        }
        let started = tokio::time::Instant::now();
        let error = rename_dir(&to, &from).await.unwrap_err();
        assert_eq!(
            started.elapsed(),
            BUSY_RETRY_DELAYS.iter().sum::<std::time::Duration>()
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(3));
        assert_eq!(error.raw_os_error(), Some(5));
        assert!(to.is_dir());
        // One refusal more than the attempts made is still waiting.
        assert!(pending(Step::RenameDir, &to));
        assert!(super::fault::take(Step::RenameDir, &to).is_some());
        assert!(!pending(Step::RenameDir, &to));

        let started = tokio::time::Instant::now();
        let missing = tmp.path().join("missing");
        let error = rename_dir(&missing, &from).await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert_eq!(started.elapsed(), std::time::Duration::ZERO);
    }

    /// A replacing persist refused with Access denied: Windows retries it
    /// through std's rename, which can replace a file another program holds
    /// open with delete sharing, and publishes an ordinary file. Elsewhere
    /// the refusal is reported and the private file is gone.
    #[test]
    fn a_denied_replacing_persist() {
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("record.json");
        std::fs::write(&destination, b"old").unwrap();
        inject(Step::Replace, &destination, 5);
        let result = persist(private(tmp.path(), b"new"), &destination);
        assert!(!pending(Step::Replace, &destination));
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            result.unwrap();
            assert_eq!(std::fs::read(&destination).unwrap(), b"new");
            let attributes = std::fs::metadata(&destination).unwrap().file_attributes();
            assert_eq!(
                attributes & 0x100,
                0,
                "published TEMPORARY: {attributes:#x}"
            );
        }
        #[cfg(not(windows))]
        {
            assert_eq!(result.unwrap_err().raw_os_error(), Some(5));
            assert_eq!(std::fs::read(&destination).unwrap(), b"old");
        }
        assert!(leftovers(tmp.path()).is_empty());
    }

    /// Moving a user's file keeps its attributes, and a move that fails
    /// leaves them as they were instead of marking the file TEMPORARY.
    #[cfg(windows)]
    #[test]
    // Clearing the flag again lets the temporary folder go; on Windows that
    // touches nothing but the read-only attribute.
    #[allow(clippy::permissions_set_readonly_false)]
    fn windows_moves_keep_a_user_files_attributes() {
        use std::os::windows::fs::MetadataExt;
        const READONLY: u32 = 0x1;
        const TEMPORARY: u32 = 0x100;
        let attributes = |path: &Path| std::fs::metadata(path).unwrap().file_attributes();
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("iris.jar.disabled");
        let destination = tmp.path().join("iris.jar");
        std::fs::write(&source, b"mod").unwrap();
        let mut permissions = std::fs::metadata(&source).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&source, permissions).unwrap();
        let before = attributes(&source);
        assert_ne!(before & READONLY, 0);

        std::fs::write(&destination, b"other").unwrap();
        let error = move_noclobber(&source, &destination).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(attributes(&source), before);
        assert_eq!(attributes(&source) & TEMPORARY, 0);

        std::fs::remove_file(&destination).unwrap();
        move_noclobber(&source, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"mod");
        assert_eq!(attributes(&destination), before);

        let mut permissions = std::fs::metadata(&destination).unwrap().permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(&destination, permissions).unwrap();
    }

    #[test]
    fn cross_device_move_of_a_directory_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("world");
        std::fs::create_dir(&source).unwrap();
        let destination = tmp.path().join("copy");
        inject(Step::NoReplace, &destination, CROSS_DEVICE);
        let error = move_noclobber(&source, &destination).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(CROSS_DEVICE));
        assert!(source.is_dir());
        assert!(!destination.exists());
    }
}
