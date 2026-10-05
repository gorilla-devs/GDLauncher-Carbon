use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;

#[cfg(target_os = "windows")]
pub static PATH_SEPARATOR: &str = ";";

#[cfg(not(target_os = "windows"))]
pub static PATH_SEPARATOR: &str = ":";

const JAVA_CHECK_APP: &[u8; 1013] = include_bytes!("JavaCheck.class");
pub const JAVA_CHECK_APP_NAME: &str = "JavaCheck.class";
/// Prefix of the folder made for one run of the launcher when the account's
/// own folder cannot be used.
const PRIVATE_FOLDER_PREFIX: &str = "gdlauncher-java-check-";

/// Publishes `JavaCheck.class` for `java -cp <its folder>` and returns its
/// path. The folder is one only this account can change, so a probe always
/// runs the launcher's own class and no other account can block it:
///
/// - On unix the temp folder can be shared by every account (`/tmp`), so
///   each account gets `gdlauncher-<uid>` in it, made 0700. 2.0.40 wrote
///   `/tmp/JavaCheck.class` itself; that file is left alone for it.
/// - On Windows every account has its own temp folder, which holds the class
///   as in 2.0.40.
///
/// When that folder cannot be used (another account owns it, say), the
/// class goes into a private folder made for this run of the launcher, so
/// where the helper lives never makes a Java check fail.
pub async fn locate_java_check_class() -> anyhow::Result<PathBuf> {
    locate_java_check_class_in(std::env::temp_dir()).await
}

async fn locate_java_check_class_in(temp_dir: PathBuf) -> anyhow::Result<PathBuf> {
    // The blocking owner retains its lock and private file even if the caller
    // disappears. Publication can never expose an unfinished Tokio write.
    tokio::task::spawn_blocking(move || {
        // The private folder made in each temp folder during this run.
        static PUBLICATION: std::sync::Mutex<Vec<(PathBuf, PathBuf)>> =
            std::sync::Mutex::new(Vec::new());
        let mut private = PUBLICATION
            .lock()
            .map_err(|_| anyhow::anyhow!("Java checker publication panicked"))?;
        let unusable = match account_folder(&temp_dir).and_then(|dir| publish_in(&dir)) {
            Ok(path) => return Ok(path),
            Err(error) => error,
        };
        // mkdtemp gave that folder a name no other account could take first,
        // so it only has to still be there.
        if let Some((_, dir)) = private.iter().find(|(base, _)| *base == temp_dir) {
            if let Ok(path) = real_folder(dir).and_then(|()| publish_in(dir)) {
                return Ok(path);
            }
        }
        tracing::warn!("Using a private folder for the Java check helper: {unusable:#}");
        let dir = new_private_folder(&temp_dir)?;
        let path = publish_in(&dir)?;
        private.retain(|(base, _)| *base != temp_dir);
        private.push((temp_dir, dir));
        Ok(path)
    })
    .await?
}

/// The folder in `temp_dir` that holds this account's checker.
#[cfg(unix)]
fn account_folder(temp_dir: &Path) -> anyhow::Result<PathBuf> {
    use std::os::unix::fs::DirBuilderExt;
    let dir = temp_dir.join(format!("gdlauncher-{}", current_uid()));
    match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(error).with_context(|| format!("Could not create {}", dir.display()));
        }
    }
    only_this_account_can_change(&dir)?;
    Ok(dir)
}

#[cfg(not(unix))]
fn account_folder(temp_dir: &Path) -> anyhow::Result<PathBuf> {
    Ok(temp_dir.to_owned())
}

#[cfg(unix)]
fn current_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

/// `dir` is a real folder, not a link, and no other account can add,
/// replace or change the files in it.
#[cfg(unix)]
fn only_this_account_can_change(dir: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let metadata = folder_metadata(dir)?;
    anyhow::ensure!(
        metadata.uid() == current_uid(),
        "{} belongs to another account",
        dir.display()
    );
    anyhow::ensure!(
        metadata.mode() & 0o022 == 0,
        "{} can be changed by other accounts",
        dir.display()
    );
    Ok(())
}

/// `dir` is a real folder, not a link.
fn real_folder(dir: &Path) -> anyhow::Result<()> {
    folder_metadata(dir).map(drop)
}

fn folder_metadata(dir: &Path) -> anyhow::Result<std::fs::Metadata> {
    let metadata = std::fs::symlink_metadata(dir)
        .with_context(|| format!("Could not inspect {}", dir.display()))?;
    anyhow::ensure!(metadata.is_dir(), "{} is not a folder", dir.display());
    Ok(metadata)
}

/// A new folder in `temp_dir` that only this account can open.
fn new_private_folder(temp_dir: &Path) -> anyhow::Result<PathBuf> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(PRIVATE_FOLDER_PREFIX);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let dir = builder.tempdir_in(temp_dir).with_context(|| {
        format!(
            "Could not create a folder for the Java check helper in the temp folder {}. Check that it exists, is writable and has free space",
            temp_dir.display()
        )
    })?;
    Ok(dir.keep())
}

/// Writes the checker into `dir` unless an intact copy is already there.
fn publish_in(dir: &Path) -> anyhow::Result<PathBuf> {
    let path = dir.join(JAVA_CHECK_APP_NAME);
    if std::fs::read(&path).is_ok_and(|bytes| bytes == JAVA_CHECK_APP) {
        return Ok(path);
    }
    let publish = || -> std::io::Result<()> {
        let mut staged = crate::util::publish::temp_in(dir)?;
        staged.write_all(JAVA_CHECK_APP)?;
        staged.flush()?;
        crate::util::durable::sync_file(staged.as_file())?;
        crate::util::publish::persist(staged, &path)
    };
    publish().with_context(|| {
        format!(
            "Could not write the Java check helper to {}. Check that the folder is writable and its drive has free space",
            dir.display()
        )
    })?;
    Ok(path)
}

#[cfg(test)]
mod task18_tests {
    use super::*;
    #[tokio::test]
    async fn task18_java_check_repairs_stale_and_replaced_bytes() {
        let root = tempfile::tempdir().unwrap();
        let path = locate_java_check_class_in(root.path().into())
            .await
            .unwrap();
        for bytes in [vec![], vec![0; 12], vec![0; JAVA_CHECK_APP.len()]] {
            std::fs::write(&path, bytes).unwrap();
            for _ in 0..3 {
                let found = locate_java_check_class_in(root.path().into())
                    .await
                    .unwrap();
                assert_eq!(found, path);
                assert_eq!(std::fs::read(found).unwrap(), JAVA_CHECK_APP);
            }
        }
    }
    #[tokio::test]
    async fn task18_java_check_absent_healthy_and_deleted() {
        let root = tempfile::tempdir().unwrap();
        for _ in 0..2 {
            let path = locate_java_check_class_in(root.path().into())
                .await
                .unwrap();
            assert_eq!(std::fs::read(&path).unwrap(), JAVA_CHECK_APP);
            let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
            locate_java_check_class_in(root.path().into())
                .await
                .unwrap();
            assert_eq!(
                std::fs::metadata(&path).unwrap().modified().unwrap(),
                modified
            );
            std::fs::remove_file(path).unwrap();
        }
    }
}

#[cfg(all(test, unix))]
mod account_folder_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn account_folder_in(root: &Path) -> PathBuf {
        root.join(format!("gdlauncher-{}", current_uid()))
    }

    fn mode(path: &Path) -> u32 {
        std::fs::symlink_metadata(path)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777
    }

    /// The other account's checker in a shared, sticky temp folder can be
    /// neither read nor replaced. This account checks Java from its own
    /// folder instead and leaves that file exactly as it was.
    #[tokio::test]
    async fn java_check_uses_the_account_folder_beside_another_accounts_checker() {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o1777)).unwrap();
        let shared = root.path().join(JAVA_CHECK_APP_NAME);
        std::fs::write(&shared, b"another account's checker").unwrap();
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o000)).unwrap();

        let found = locate_java_check_class_in(root.path().into())
            .await
            .unwrap();

        let folder = account_folder_in(root.path());
        assert_eq!(found, folder.join(JAVA_CHECK_APP_NAME));
        assert_eq!(std::fs::read(&found).unwrap(), JAVA_CHECK_APP);
        assert_eq!(mode(&folder), 0o700);
        assert_eq!(mode(&shared), 0o000);
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            std::fs::read(&shared).unwrap(),
            b"another account's checker"
        );
        assert_eq!(
            locate_java_check_class_in(root.path().into())
                .await
                .unwrap(),
            found
        );
    }

    /// An account folder that is a file, a link or open to other accounts is
    /// never used or changed. The checker goes into a private 0700 folder of
    /// this run, which later checks reuse and which is made again if it is
    /// removed.
    #[tokio::test]
    async fn java_check_falls_back_to_a_private_folder_when_the_account_folder_is_unusable() {
        for case in ["file", "link", "writable"] {
            let root = tempfile::tempdir().unwrap();
            let folder = account_folder_in(root.path());
            let elsewhere = root.path().join("elsewhere");
            std::fs::create_dir(&elsewhere).unwrap();
            match case {
                "file" => std::fs::write(&folder, b"not a folder").unwrap(),
                "link" => std::os::unix::fs::symlink(&elsewhere, &folder).unwrap(),
                _ => {
                    std::fs::create_dir(&folder).unwrap();
                    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o777))
                        .unwrap();
                }
            }

            let found = locate_java_check_class_in(root.path().into())
                .await
                .unwrap();
            assert_eq!(std::fs::read(&found).unwrap(), JAVA_CHECK_APP, "{case}");
            let private = found.parent().unwrap().to_owned();
            assert_eq!(private.parent().unwrap(), root.path(), "{case}");
            assert!(
                private
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(PRIVATE_FOLDER_PREFIX),
                "{case}: {}",
                private.display()
            );
            assert_eq!(mode(&private), 0o700, "{case}");
            assert_eq!(
                locate_java_check_class_in(root.path().into())
                    .await
                    .unwrap(),
                found,
                "{case}: later checks reuse the private folder"
            );

            std::fs::remove_dir_all(&private).unwrap();
            let again = locate_java_check_class_in(root.path().into())
                .await
                .unwrap();
            assert_eq!(std::fs::read(&again).unwrap(), JAVA_CHECK_APP, "{case}");
            assert_ne!(again.parent().unwrap(), folder, "{case}");

            assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0, "{case}");
            match case {
                "file" => assert_eq!(std::fs::read(&folder).unwrap(), b"not a folder"),
                "link" => assert!(std::fs::symlink_metadata(&folder).unwrap().is_symlink()),
                _ => {
                    assert_eq!(mode(&folder), 0o777);
                    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
                }
            }
        }
    }
}
