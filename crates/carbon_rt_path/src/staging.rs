//! Where work is built before a rename publishes it.
//!
//! A new instance is built in full somewhere private and then renamed into
//! `instances`, so the library never holds half of one. A rename cannot leave
//! its drive, and the folder it publishes into can be on another drive than
//! the data folder's `temp`: people replace `instances` with a link or a mount
//! to keep their instances elsewhere. So the place to build depends on where
//! the work ends up, and this module is the one place that decides it:
//!
//! - [`RuntimePath::staging_for`] gives the place to build for a destination:
//!   `<data>/temp` when a rename from there reaches the destination, and
//!   otherwise a hidden [`STAGING_FOLDER`] inside the destination itself,
//!   which is on the destination's drive whatever that drive is.
//! - [`RuntimePath::cleanup_staging`] empties every such place at start-up,
//!   when whatever is in them was left by a session that ended mid-way.
//!
//! Both go by [`PublishInto`], the list of destinations. Code that builds
//! something a rename will publish asks `staging_for` and never chooses a
//! folder itself; publishing into a folder not listed yet takes a new
//! `PublishInto` entry and nothing else. [`RuntimePath::get_temp`] stays for
//! scratch work that is read or copied from, never renamed into place.

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{RuntimePath, TempPath};

/// The staging folder inside a destination on another drive than
/// `<data>/temp`. The instance scan passes over it: it is not an instance it
/// knows and has no `instance.json`, as what is staged sits one level below.
/// No instance can take the name either, as instance folders never start
/// with a dot.
pub const STAGING_FOLDER: &str = ".gdl_staging";

/// The folders of the data folder that finished work is renamed into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishInto {
    /// `<data>/instances`: a created, imported or duplicated instance.
    Instances,
}

impl PublishInto {
    /// Every destination, for [`RuntimePath::cleanup_staging`].
    pub const ALL: &'static [PublishInto] = &[PublishInto::Instances];

    fn folder(self, runtime: &RuntimePath) -> PathBuf {
        match self {
            PublishInto::Instances => runtime.get_instances().to_path(),
        }
    }
}

impl RuntimePath {
    /// The place to build what a rename will publish into `into`: the data
    /// folder's `temp`, or the destination's own [`STAGING_FOLDER`] when the
    /// destination is on another drive. Entries made there are removed when
    /// dropped, as everywhere, and by [`RuntimePath::cleanup_staging`].
    ///
    /// Asked each time, not remembered: a drive can be mounted over the
    /// destination while the launcher runs.
    pub async fn staging_for(&self, into: PublishInto) -> TempPath {
        let temp = self.get_temp();
        let staging = into.folder(self).join(STAGING_FOLDER);
        if on_another_drive(&temp, &staging).await {
            hide(&staging);
            TempPath(staging)
        } else {
            // Made for the question alone. Only an empty folder goes.
            let _ = tokio::fs::remove_dir(&staging).await;
            temp
        }
    }

    /// Empties `temp` and the staging folder of every destination, and
    /// removes the staging folders. For start-up, before anything builds.
    pub async fn cleanup_staging(&self) {
        self.get_temp().cleanup_all().await;
        for into in PublishInto::ALL {
            let staging = into.folder(self).join(STAGING_FOLDER);
            // A link is never ours: staging folders are made as plain folders.
            let plain_folder = tokio::fs::symlink_metadata(&staging)
                .await
                .is_ok_and(|metadata| metadata.is_dir());
            if plain_folder {
                TempPath(staging.clone()).cleanup_all().await;
                if let Err(error) = tokio::fs::remove_dir(&staging).await {
                    tracing::warn!(
                        "Could not remove the staging folder {}: {error}",
                        staging.display()
                    );
                }
            }
        }
    }
}

/// Whether a rename out of `temp` is refused at `staging` for being on
/// another drive, found by renaming an empty folder there: the same call the
/// publish makes, so it is right for links, mounts, junctions and drives
/// alike. Anything else that goes wrong (no destination yet, or one that
/// takes nothing) answers no, and the publish reports it as it always did.
async fn on_another_drive(temp: &TempPath, staging: &Path) -> bool {
    let Ok(probe) = temp.maketmpdir().await else {
        return false;
    };
    match tokio::fs::create_dir(staging).await {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(_) => return false,
    }
    let Some(name) = probe.file_name() else {
        return false;
    };
    let landed = staging.join(name);
    match tokio::fs::rename(&*probe, &landed).await {
        Ok(()) => {
            let _ = tokio::fs::remove_dir(&landed).await;
            false
        }
        Err(error) => crosses_devices(&error),
    }
}

/// EXDEV on unix, ERROR_NOT_SAME_DEVICE on Windows.
pub fn crosses_devices(error: &io::Error) -> bool {
    const CODE: i32 = if cfg!(windows) { 17 } else { 18 };
    error.kind() == io::ErrorKind::CrossesDevices || error.raw_os_error() == Some(CODE)
}

/// The dot hides the folder everywhere but on Windows, which goes by an
/// attribute. Best effort: a folder that stays visible still works.
#[cfg(windows)]
fn hide(folder: &Path) {
    use std::os::windows::ffi::OsStrExt;

    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetFileAttributesW(path: *const u16, attributes: u32) -> i32;
    }

    let path: Vec<u16> = folder.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `path` is a NUL-terminated UTF-16 string that outlives the call.
    unsafe { SetFileAttributesW(path.as_ptr(), FILE_ATTRIBUTE_HIDDEN) };
}

#[cfg(not(windows))]
fn hide(_: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_folder(name: &str) -> RuntimePath {
        let path = std::env::temp_dir().join(format!("staging-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join("temp")).unwrap();
        RuntimePath::new(path)
    }

    fn names_in(folder: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Removes a folder made outside the data folder when the test ends,
    /// passed or not.
    struct Removed(PathBuf);
    impl Drop for Removed {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A folder on another filesystem than the test's data folder, standing
    /// in for another drive, or `None` where this machine has none to offer.
    #[cfg(target_os = "linux")]
    fn other_drive(name: &str, data: &RuntimePath) -> Option<Removed> {
        use std::os::unix::fs::MetadataExt;
        let shm = Path::new("/dev/shm");
        let here = std::fs::metadata(data.get_temp().to_path()).ok()?.dev();
        if std::fs::metadata(shm).ok()?.dev() == here {
            return None;
        }
        let folder = shm.join(format!("gdl-staging-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).ok()?;
        Some(Removed(folder))
    }

    #[tokio::test]
    async fn work_for_a_destination_on_the_data_drive_is_built_in_temp() {
        let data = data_folder("same-drive");
        let instances = data.get_instances().to_path();
        std::fs::create_dir_all(instances.join("kept")).unwrap();

        let staging = data.staging_for(PublishInto::Instances).await;

        assert_eq!(staging.to_path(), data.get_temp().to_path());
        assert_eq!(names_in(&instances), ["kept"], "nothing is left there");
        assert!(names_in(&data.get_temp().to_path()).is_empty());
    }

    #[tokio::test]
    async fn a_destination_that_does_not_exist_yet_is_not_created_by_asking() {
        let data = data_folder("no-destination");

        let staging = data.staging_for(PublishInto::Instances).await;

        assert_eq!(staging.to_path(), data.get_temp().to_path());
        assert!(!data.get_instances().to_path().exists());
    }

    #[tokio::test]
    async fn asking_never_removes_work_staged_in_the_destination() {
        let data = data_folder("staged-work");
        let staged = data
            .get_instances()
            .to_path()
            .join(STAGING_FOLDER)
            .join("an-instance-being-built");
        std::fs::create_dir_all(&staged).unwrap();

        data.staging_for(PublishInto::Instances).await;

        assert!(staged.is_dir());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn work_for_a_destination_on_another_drive_is_built_on_that_drive() {
        let data = data_folder("other-drive");
        let Some(drive) = other_drive("build", &data) else {
            eprintln!("skipped: /dev/shm is not another filesystem here");
            return;
        };
        let instances = data.get_instances().to_path();
        std::os::unix::fs::symlink(&drive.0, &instances).unwrap();
        let from_temp = data.get_temp().maketmpdir().await.unwrap();
        assert!(
            std::fs::rename(&*from_temp, instances.join("from-temp")).is_err(),
            "the rename this is all about is refused from temp"
        );

        let staging = data.staging_for(PublishInto::Instances).await;
        assert_eq!(staging.to_path(), instances.join(STAGING_FOLDER));
        let built = staging.maketmpdir().await.unwrap();
        std::fs::write(built.join("instance.json"), b"{}").unwrap();
        std::fs::rename(&*built, instances.join("New instance")).unwrap();

        assert!(instances.join("New instance/instance.json").is_file());
        assert_eq!(names_in(&instances), [STAGING_FOLDER, "New instance"]);
    }

    #[tokio::test]
    async fn start_up_empties_temp_and_removes_every_staging_folder() {
        let data = data_folder("start-up");
        let instances = data.get_instances().to_path();
        let temp = data.get_temp().to_path();
        std::fs::create_dir_all(instances.join("kept/instance")).unwrap();
        std::fs::create_dir_all(instances.join(STAGING_FOLDER).join("left/instance")).unwrap();
        std::fs::write(instances.join(STAGING_FOLDER).join("left/a.jar"), b"jar").unwrap();
        std::fs::create_dir_all(temp.join("left")).unwrap();
        std::fs::write(temp.join("left.zip"), b"zip").unwrap();

        data.cleanup_staging().await;

        assert!(names_in(&temp).is_empty());
        assert_eq!(names_in(&instances), ["kept"]);
        assert!(instances.join("kept/instance").is_dir());
    }

    /// A link named like the staging folder is someone else's: what it leads
    /// to is not ours to empty.
    #[cfg(unix)]
    #[tokio::test]
    async fn start_up_leaves_a_link_named_like_the_staging_folder_alone() {
        let data = data_folder("start-up-link");
        let instances = data.get_instances().to_path();
        let elsewhere = data.to_path_buf().join("elsewhere");
        std::fs::create_dir_all(elsewhere.join("theirs")).unwrap();
        std::fs::create_dir_all(&instances).unwrap();
        std::os::unix::fs::symlink(&elsewhere, instances.join(STAGING_FOLDER)).unwrap();

        data.cleanup_staging().await;

        assert!(elsewhere.join("theirs").is_dir());
    }
}
