use std::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::Deref,
    path::{Path, PathBuf},
    sync::atomic::{self, AtomicUsize},
};

use anyhow::Context;

#[derive(Clone)]
pub struct RuntimePath(PathBuf);

pub struct RootPath(PathBuf);

impl RootPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }
}

#[derive(Clone)]
pub struct LibrariesPath(PathBuf);

impl LibrariesPath {
    pub fn get_mc_client(&self, id: &str) -> PathBuf {
        self.0
            .join("net/minecraft/client")
            .join(id)
            .join(format!("{}.jar", id))
    }
    pub fn get_library_path(&self, library_path: String) -> PathBuf {
        self.0.join(library_path)
    }

    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }
}

pub struct AssetsPath(PathBuf);

impl AssetsPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_indexes_path(&self) -> PathBuf {
        self.0.join("indexes")
    }

    pub fn get_virtual_path(&self) -> PathBuf {
        self.0.join("virtual")
    }

    pub fn get_objects_path(&self) -> PathBuf {
        self.0.join("objects")
    }
}

pub struct NativesPath(PathBuf);

impl NativesPath {
    pub fn get_versioned(&self, version: &str) -> PathBuf {
        self.0.clone().join(version)
    }
}

#[derive(Debug)]
pub struct ManagedJavasPath(PathBuf);

impl ManagedJavasPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }
}

pub struct LoggingConfigsPath(PathBuf);

impl LoggingConfigsPath {
    pub fn get_client_path(&self, id: &str) -> PathBuf {
        self.0.clone().join(&id)
    }
}

pub struct ServersPath(PathBuf);

impl ServersPath {
    pub fn subpath() -> ServersPath {
        Self(PathBuf::from("servers"))
    }

    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_server_path(&self, server_shortpath: &str) -> ServerPath {
        ServerPath(self.0.join(server_shortpath))
    }
}

#[derive(Debug, Clone)]
pub struct ServerPath(PathBuf);

impl ServerPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn get_root(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_data_path(&self) -> PathBuf {
        self.0.join("server")
    }

    pub fn get_logs_path(&self) -> PathBuf {
        self.get_data_path().join("logs")
    }

    pub fn get_world_path(&self) -> PathBuf {
        self.get_data_path().join("world")
    }

    pub fn get_plugins_path(&self) -> PathBuf {
        self.get_data_path().join("plugins")
    }

    pub fn get_server_properties_path(&self) -> PathBuf {
        self.get_data_path().join("server.properties")
    }

    pub fn get_server_jar_path(&self) -> PathBuf {
        self.get_data_path().join("server.jar")
    }

    pub fn get_eula_path(&self) -> PathBuf {
        self.get_data_path().join("eula.txt")
    }

    pub fn get_mods_path(&self) -> PathBuf {
        self.get_data_path().join("mods")
    }

    pub fn get_datapacks_path(&self) -> PathBuf {
        self.get_data_path().join("world").join("datapacks")
    }

    pub fn get_whitelist_path(&self) -> PathBuf {
        self.get_data_path().join("whitelist.json")
    }

    pub fn get_ops_path(&self) -> PathBuf {
        self.get_data_path().join("ops.json")
    }

    pub fn get_banned_players_path(&self) -> PathBuf {
        self.get_data_path().join("banned-players.json")
    }

    pub fn get_banned_ips_path(&self) -> PathBuf {
        self.get_data_path().join("banned-ips.json")
    }

    pub fn get_modloader_config_path(&self) -> PathBuf {
        self.0.join("modloader_config.json")
    }
}

// TODO: WIP
pub struct InstancesPath(PathBuf);

impl InstancesPath {
    pub fn subpath() -> InstancesPath {
        Self(PathBuf::from("instances"))
    }

    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_instance_path(&self, instance_shortpath: &str) -> InstancePath {
        InstancePath(self.0.join(instance_shortpath))
    }
}

#[derive(Debug, Clone)]
pub struct InstancePath(PathBuf);

impl InstancePath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn get_root(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_data_path(&self) -> PathBuf {
        self.0.join("instance")
    }

    pub fn get_resources_path(&self) -> PathBuf {
        self.get_data_path().join("resources")
    }

    pub fn get_mods_path(&self) -> PathBuf {
        self.get_data_path().join("mods")
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.get_data_path().join("config")
    }

    pub fn get_resourcepacks_path(&self) -> PathBuf {
        self.get_data_path().join("resourcepacks")
    }

    pub fn get_texturepacks_path(&self) -> PathBuf {
        self.get_data_path().join("texturepacks")
    }

    pub fn get_shaderpacks_path(&self) -> PathBuf {
        self.get_data_path().join("shaderpacks")
    }

    pub fn get_saves_path(&self) -> PathBuf {
        self.get_data_path().join("saves")
    }

    pub fn get_datapacks_path(&self) -> PathBuf {
        self.get_data_path().join("datapacks")
    }

    pub fn get_logs_path(&self) -> PathBuf {
        self.get_data_path().join("logs")
    }

    pub fn get_gdl_logs_path(&self) -> PathBuf {
        self.get_root().join("logs")
    }

    pub fn get_plugins_path(&self) -> PathBuf {
        self.get_data_path().join("plugins")
    }

    pub fn get_crash_reports_path(&self) -> PathBuf {
        self.get_data_path().join("crash-reports")
    }

    pub fn get_screenshots_path(&self) -> PathBuf {
        self.get_data_path().join("screenshots")
    }

    pub fn get_options_file_path(&self) -> PathBuf {
        self.get_data_path().join("options.txt")
    }
}

#[derive(Debug)]
pub struct TempPath(PathBuf);

impl TempPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub async fn maketmp<T: tempentry::TempEntryType>(&self) -> anyhow::Result<TempEntry<T>> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time is somehow pre-epoch")
            .as_millis() as usize;

        // Bounded, so a temp folder that cannot hold entries fails instead of spinning.
        for _ in 0..1000 {
            static LAST_COUNT: AtomicUsize = AtomicUsize::new(0);
            let i = LAST_COUNT.fetch_add(1, atomic::Ordering::Relaxed);
            let path = self.to_path().join(format!("{time}-{i}"));
            if path.exists() {
                continue;
            }
            let path_copy = path.clone();
            match tokio::task::spawn_blocking(move || T::create(&path_copy))
                .await
                .with_context(|| format!("Creating temporary entry {}", path.display()))?
            {
                Ok(()) => return Ok(TempEntry(path, PhantomData)),
                // Only a name some other entry took is a collision. A temp
                // folder that is a file or a dangling link reports
                // AlreadyExists for every name.
                Err(error)
                    if error.kind() == std::io::ErrorKind::AlreadyExists
                        && path.symlink_metadata().is_ok() =>
                {
                    continue;
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("Creating temporary entry {}", path.display()));
                }
            }
        }
        anyhow::bail!(
            "No free temporary entry name in {}",
            self.to_path().display()
        )
    }

    pub async fn maketmpdir(&self) -> anyhow::Result<TempEntry<tempentry::Folder>> {
        self.maketmp().await
    }

    pub async fn maketmpfile(&self) -> anyhow::Result<TempEntry<tempentry::File>> {
        self.maketmp().await
    }

    /// Clean up all temp files/folders. Should be called on startup.
    /// Runs the (potentially slow) tree walk on a blocking thread so it
    /// doesn't stall the tokio runtime if temp accumulated GBs across
    /// interrupted modpack installs.
    pub async fn cleanup_all(&self) {
        let root = self.0.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let Ok(read_dir) = std::fs::read_dir(&root) else {
                return;
            };

            for entry in read_dir.filter_map(|e| e.ok()) {
                let path = entry.path();
                let res = if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
                if let Err(e) = res {
                    tracing::warn!("Failed to clean up temp entry {}: {}", path.display(), e);
                }
            }
        })
        .await;
    }
}

pub struct TempEntry<T: tempentry::TempEntryType>(PathBuf, PhantomData<T>);

pub mod tempentry {
    use std::{io, path::Path};

    pub trait TempEntryType {
        // tokio::fs is just implemented as a spawn_blocking wrapper for the most part.
        fn create(path: &Path) -> io::Result<()>;
        fn drop_for(path: &Path);
    }

    pub struct Folder;
    pub struct File;

    /// Names a temp folder that can't hold entries, which otherwise only shows up as
    /// "File exists" or "Not a directory" for an entry inside it. Keeps the error kind.
    fn explain(entry: &Path, error: io::Error) -> io::Error {
        match entry.parent() {
            Some(root) if root.symlink_metadata().is_ok() && !root.is_dir() => io::Error::new(
                error.kind(),
                format!(
                    "GDLauncher's temporary folder {} is not a usable folder: it is a file, or \
                     a link to a place that is not available. Remove it or reconnect the drive \
                     it points to, then try again ({error})",
                    root.display()
                ),
            ),
            _ => error,
        }
    }

    impl TempEntryType for Folder {
        fn create(path: &Path) -> io::Result<()> {
            std::fs::create_dir_all(path).map_err(|error| explain(path, error))
        }

        fn drop_for(path: &Path) {
            let _ = std::fs::remove_dir_all(path);
        }
    }

    impl TempEntryType for File {
        fn create(path: &Path) -> io::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| explain(path, error))?;
            }

            // files will be created on write
            Ok(())
        }

        fn drop_for(path: &Path) {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl<T: tempentry::TempEntryType> TempEntry<T> {
    /// Extract the contained path without deleting the tmpdir.
    pub fn into_path(self) -> PathBuf {
        let v = ManuallyDrop::new(self);

        // SAFETY: v is not dropped so v.0 can be extracted safely.
        let path = unsafe { std::ptr::read(&v.0 as *const PathBuf) };

        path
    }

    pub async fn try_rename_or_move(self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let dest = path.as_ref();

        if tokio::fs::rename(&*self, dest).await.is_ok() {
            return Ok(());
        }

        // rename failed (typically EXDEV: the temp source and the destination live on different
        // filesystems). Stage the copy as a sibling temp file in the destination directory and
        // rename it into place, so the publish stays atomic — a crash mid-copy cannot leave a
        // partially written destination. (Only files reach here; tokio::fs::copy is file-only.)
        let staging = match dest.file_name() {
            Some(name) => {
                let mut staged = name.to_os_string();
                staged.push(".__gdl_xdev~");
                dest.with_file_name(staged)
            }
            None => anyhow::bail!("destination path has no file name: {}", dest.display()),
        };

        tokio::fs::copy(&*self, &staging).await.with_context(|| {
            format!(
                "failed to copy {} to {}",
                (&*self).display(),
                staging.display(),
            )
        })?;
        tokio::fs::rename(&staging, dest).await.with_context(|| {
            format!("failed to move {} to {}", staging.display(), dest.display())
        })?;

        if self.is_dir() {
            tokio::fs::remove_dir_all(&*self).await?;
        } else {
            tokio::fs::remove_file(&*self).await?;
        }

        Ok(())
    }
}

impl<T: tempentry::TempEntryType> Deref for TempEntry<T> {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: tempentry::TempEntryType> Drop for TempEntry<T> {
    fn drop(&mut self) {
        T::drop_for(&*self)
    }
}

pub struct DownloadPath(PathBuf);

impl DownloadPath {
    pub fn to_pathbuf(&self) -> PathBuf {
        self.0.clone()
    }
}

impl RuntimePath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn get_root(&self) -> RootPath {
        RootPath(self.0.clone())
    }

    pub fn get_libraries(&self) -> LibrariesPath {
        LibrariesPath(self.0.join("libraries"))
    }

    pub fn get_assets(&self) -> AssetsPath {
        AssetsPath(self.0.join("assets"))
    }

    pub fn get_natives(&self) -> NativesPath {
        NativesPath(self.0.join("natives"))
    }

    pub fn get_managed_javas(&self) -> ManagedJavasPath {
        ManagedJavasPath(self.0.join("managed_javas"))
    }

    pub fn get_instances(&self) -> InstancesPath {
        InstancesPath(self.0.join("instances"))
    }

    pub fn get_servers(&self) -> ServersPath {
        ServersPath(self.0.join("servers"))
    }

    pub fn get_logging_configs(&self) -> LoggingConfigsPath {
        LoggingConfigsPath(self.0.join("logging_configs"))
    }

    pub fn get_temp(&self) -> TempPath {
        TempPath(self.0.join("temp"))
    }

    pub fn get_download(&self) -> DownloadPath {
        DownloadPath(self.0.join("download"))
    }
}

impl Deref for RuntimePath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// How many files [`copy_dir_filter`] copies at once. Many small files copy much faster
/// side by side on Windows (per-file create and antivirus cost), and a small fixed bound
/// keeps open files well under low per-process limits such as macOS's 256.
const COPY_CONCURRENCY: usize = 16;

/// Recursively copy `from` to `to`, overwriting files. The filter sees paths relative to
/// `from` and never the root itself; rejected directories prune their entire subtree.
/// Traversal errors in included subtrees are reported.
///
/// The whole source is listed before anything is written, so the copy cannot walk into
/// its own output. When `to` lies inside `from` (for example GDLauncher's data folder
/// sits inside the game folder being imported), the top-level source folder holding `to`
/// is skipped. Files are copied [`COPY_CONCURRENCY`] at a time; after a failure no new
/// copy starts, and the copies already running finish before the error is returned, so
/// nothing keeps writing into `to` once this returns.
pub async fn copy_dir_filter<F>(from: &Path, to: &Path, filter: F) -> anyhow::Result<()>
where
    F: for<'a> Fn(&'a Path) -> bool,
{
    let holder = destination_holder(from, to)?;
    if let Some(holder) = &holder {
        tracing::info!(
            "Skipping {} while copying {}: it holds the destination {}",
            from.join(holder).display(),
            from.display(),
            to.display()
        );
    }

    let mut directories = Vec::new();
    let mut files = Vec::new();
    {
        // Encountered directory links are not traversed (the root is followed, as walkdir
        // does). Pathname file copying below dereferences regular-file links; this is not
        // an outside-source containment policy.
        let entries = walkdir::WalkDir::new(from)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                if entry.depth() == 0 {
                    return true;
                }
                match entry.path().strip_prefix(from) {
                    Ok(relative) => holder.as_deref() != Some(relative) && filter(relative),
                    Err(_) => true,
                }
            });
        for entry in entries {
            let entry =
                entry.with_context(|| format!("Traversing included source {}", from.display()))?;
            let source = entry.path().to_path_buf();
            if entry.depth() == 0 {
                if source.is_dir() {
                    directories.push((source, to.to_path_buf()));
                } else {
                    files.push((source, to.to_path_buf()));
                }
                continue;
            }
            let relative = source.strip_prefix(from).with_context(|| {
                format!("Source {} outside {}", source.display(), from.display())
            })?;
            let destination = to.join(relative);
            if entry.file_type().is_dir() {
                directories.push((source, destination));
            } else {
                files.push((source, destination));
            }
        }
    }

    // In walk order, so every parent exists before its children and files.
    tokio::task::spawn_blocking(move || {
        for (source, destination) in directories {
            std::fs::create_dir_all(&destination).with_context(|| {
                format!(
                    "Copying directory {} to {}",
                    source.display(),
                    destination.display()
                )
            })?;
        }
        anyhow::Ok(())
    })
    .await
    .with_context(|| format!("Creating the folders of {}", to.display()))??;

    run_bounded(
        files,
        COPY_CONCURRENCY,
        |(source, destination)| async move {
            tokio::fs::copy(&source, &destination)
                .await
                .with_context(|| {
                    format!("Copying {} to {}", source.display(), destination.display())
                })
                .map(drop)
        },
    )
    .await
}

/// The top-level entry of `from` whose subtree holds `to`, if any. A destination that
/// is the source, or a source inside the destination, is refused.
fn destination_holder(from: &Path, to: &Path) -> anyhow::Result<Option<PathBuf>> {
    // When the source can't be resolved the walk reports why.
    let Ok(source) = std::fs::canonicalize(from) else {
        return Ok(None);
    };
    let Some(destination) = canonical_destination(to) else {
        return Ok(None);
    };
    anyhow::ensure!(
        !source.starts_with(&destination),
        "Can't copy {} into {}: the folder being copied is inside the destination",
        from.display(),
        to.display()
    );
    Ok(destination
        .strip_prefix(&source)
        .ok()
        .and_then(|relative| relative.components().next())
        .map(|first| PathBuf::from(first.as_os_str())))
}

/// `to` with its existing part resolved, so it compares with a canonical source path
/// even before the destination is created.
fn canonical_destination(to: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(to).ok()?;
    let mut missing = Vec::new();
    let mut current = absolute.as_path();
    loop {
        match std::fs::canonicalize(current) {
            Ok(mut resolved) => {
                resolved.extend(missing.iter().rev());
                return Some(resolved);
            }
            Err(_) => {
                missing.push(current.file_name()?);
                current = current.parent()?;
            }
        }
    }
}

/// Runs `operation` over `jobs`, at most `limit` at a time. After the first failure no
/// new job starts; the running ones are awaited, then that first failure is returned.
async fn run_bounded<T, Op, Fut>(
    jobs: impl IntoIterator<Item = T>,
    limit: usize,
    mut operation: Op,
) -> anyhow::Result<()>
where
    Op: FnMut(T) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    use futures::StreamExt;

    let mut pending = jobs.into_iter();
    let mut running = futures::stream::FuturesUnordered::new();
    let mut failure = None;
    loop {
        while failure.is_none() && running.len() < limit.max(1) {
            let Some(job) = pending.next() else { break };
            running.push(operation(job));
        }
        match running.next().await {
            Some(Ok(())) => {}
            Some(Err(error)) => match failure {
                None => failure = Some(error),
                Some(_) => tracing::debug!("Another copy failed while stopping: {error:#}"),
            },
            None => break,
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(test)]
mod task06_tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("task06-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
    #[tokio::test]
    async fn task06_temp_inner_error() {
        let root = fixture("temp");
        let file = root.join("file");
        std::fs::write(&file, b"test").unwrap();
        let result = TempPath::new(file.clone()).maketmpdir().await;
        assert!(result.is_err(), "filesystem creation error must propagate");
        let error = result.err().unwrap();
        assert!(error.downcast_ref::<std::io::Error>().is_some());
        let ok = TempPath::new(root.clone()).maketmpdir().await.unwrap();
        assert!(ok.is_dir());
    }
    #[tokio::test]
    async fn a_temp_folder_that_is_a_file_fails_instead_of_spinning() {
        let root = fixture("file-root");
        let file = root.join("file");
        std::fs::write(&file, b"test").unwrap();
        let temp = TempPath::new(file);
        for result in [
            tokio::time::timeout(std::time::Duration::from_secs(10), temp.maketmpfile())
                .await
                .map(|entry| entry.map(|_| ())),
            tokio::time::timeout(std::time::Duration::from_secs(10), temp.maketmpdir())
                .await
                .map(|entry| entry.map(|_| ())),
        ] {
            let result = result.expect("temp creation must not spin");
            assert!(result.is_err(), "a file can't hold temp entries");
        }
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn task06_included_copy_error_and_excluded_control() {
        use std::os::unix::fs::PermissionsExt;
        let root = fixture("copy");
        let source = root.join("source");
        let world = source.join("saves/world");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"world").unwrap();
        std::fs::write(source.join("options.txt"), b"options").unwrap();
        copy_dir_filter(&source, &root.join("readable"), |_| true)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(root.join("readable/saves/world/level.dat")).unwrap(),
            b"world"
        );
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
        }
        let _restore = Restore(world.clone());
        std::fs::set_permissions(&world, std::fs::Permissions::from_mode(0)).unwrap();
        assert!(
            std::fs::read_dir(&world).is_err(),
            "fixture requires unprivileged permissions; do not claim root coverage"
        );
        let result = copy_dir_filter(&source, &root.join("denied"), |_| true).await;
        assert!(
            result.is_err(),
            "included traversal failure cannot become successful copy"
        );
        assert!(format!("{:#}", result.unwrap_err()).contains("world"));
        copy_dir_filter(&source, &root.join("excluded"), |p| p != Path::new("saves"))
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(root.join("excluded/options.txt")).unwrap(),
            b"options"
        );
        assert!(!root.join("excluded/saves").exists());
    }
}

#[cfg(test)]
mod copy_and_temp_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn fresh(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("rt-path-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[tokio::test]
    async fn bounded_runs_up_to_the_limit_at_once() {
        let running = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let done = AtomicUsize::new(0);
        run_bounded(0..40, 4, |_| {
            let (running, peak, done) = (&running, &peak, &done);
            async move {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                for _ in 0..3 {
                    tokio::task::yield_now().await;
                }
                running.fetch_sub(1, Ordering::SeqCst);
                done.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();
        assert_eq!(peak.load(Ordering::SeqCst), 4);
        assert_eq!(done.load(Ordering::SeqCst), 40);
    }

    #[tokio::test]
    async fn bounded_stops_starting_and_waits_for_running_jobs_after_a_failure() {
        let started = AtomicUsize::new(0);
        let finished = AtomicUsize::new(0);
        let result = run_bounded(0..100, 4, |job| {
            let (started, finished) = (&started, &finished);
            async move {
                started.fetch_add(1, Ordering::SeqCst);
                // Later failures must not replace the first one.
                let delay = if job == 5 { 1 } else { 4 };
                for _ in 0..delay {
                    tokio::task::yield_now().await;
                }
                finished.fetch_add(1, Ordering::SeqCst);
                match job {
                    5 => anyhow::bail!("job 5 failed"),
                    6 => anyhow::bail!("job 6 failed"),
                    _ => Ok(()),
                }
            }
        })
        .await;
        assert_eq!(result.unwrap_err().to_string(), "job 5 failed");
        let started = started.load(Ordering::SeqCst);
        assert!(started < 100, "no job may start after the failure");
        assert_eq!(
            finished.load(Ordering::SeqCst),
            started,
            "every job that started finished before the error was returned"
        );
    }

    /// Each FIFO blocks its copy until a writer opens it. The writer opens them in
    /// reverse walk order, which only gets anywhere when the copies run side by side.
    #[cfg(unix)]
    #[tokio::test]
    async fn copies_files_side_by_side_and_waits_for_them_on_failure() {
        let root = fresh("fifo");
        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        for name in ["a", "b", "c", "d"] {
            let status = std::process::Command::new("mkfifo")
                .arg(source.join(name))
                .status()
                .unwrap();
            assert!(status.success());
        }
        let order: Vec<PathBuf> = walkdir::WalkDir::new(&source)
            .min_depth(1)
            .into_iter()
            .map(|entry| entry.unwrap().into_path())
            .collect();
        let writer_order = order.clone();
        let writer = std::thread::spawn(move || {
            for fifo in writer_order.iter().rev() {
                drop(std::fs::OpenOptions::new().write(true).open(fifo).unwrap());
            }
        });
        let copy = tokio::time::timeout(
            Duration::from_secs(20),
            copy_dir_filter(&source, &root.join("destination"), |_| true),
        )
        .await;
        if copy.is_err() {
            // Unblock the copy stuck on the first FIFO so the runtime can shut down.
            std::thread::spawn(move || {
                for fifo in &order {
                    let _ = std::fs::OpenOptions::new().write(true).open(fifo);
                }
            });
            panic!("the copies ran one at a time");
        }
        writer.join().unwrap();
        let error = format!("{:#}", copy.unwrap().unwrap_err());
        assert!(error.contains("source"), "{error}");
    }

    #[tokio::test]
    async fn copies_a_tree_through_the_filter() {
        let root = fresh("tree");
        let source = root.join("source");
        for dir in 0..5 {
            let folder = source.join(format!("config/mod{dir}/nested"));
            std::fs::create_dir_all(&folder).unwrap();
            for file in 0..30 {
                std::fs::write(folder.join(format!("{file}.toml")), format!("{dir}-{file}"))
                    .unwrap();
            }
        }
        std::fs::create_dir_all(source.join("logs/old")).unwrap();
        std::fs::write(source.join("logs/old/latest.log"), b"log").unwrap();
        std::fs::create_dir_all(source.join("empty")).unwrap();
        std::fs::write(source.join("options.txt"), b"options").unwrap();
        let destination = root.join("destination");
        copy_dir_filter(&source, &destination, |path| path != Path::new("logs"))
            .await
            .unwrap();
        for dir in 0..5 {
            for file in 0..30 {
                assert_eq!(
                    std::fs::read_to_string(
                        destination.join(format!("config/mod{dir}/nested/{file}.toml"))
                    )
                    .unwrap(),
                    format!("{dir}-{file}")
                );
            }
        }
        assert!(destination.join("empty").is_dir());
        assert_eq!(
            std::fs::read(destination.join("options.txt")).unwrap(),
            b"options"
        );
        assert!(!destination.join("logs").exists());
    }

    /// The legacy importer rejects the path equal to its `background` setting, which
    /// is the root when the setting is empty.
    #[tokio::test]
    async fn the_filter_never_rejects_the_root() {
        let root = fresh("root-filter");
        let source = root.join("source");
        std::fs::create_dir_all(source.join("mods")).unwrap();
        std::fs::write(source.join("mods/a.jar"), b"jar").unwrap();
        let destination = root.join("destination");
        copy_dir_filter(&source, &destination, |path| path != Path::new(""))
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(destination.join("mods/a.jar")).unwrap(),
            b"jar"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_linked_source_folder_is_copied() {
        let root = fresh("root-link");
        let source = root.join("source");
        std::fs::create_dir_all(source.join("saves")).unwrap();
        std::fs::write(source.join("saves/level.dat"), b"world").unwrap();
        let link = root.join("link");
        std::os::unix::fs::symlink(&source, &link).unwrap();
        let destination = root.join("destination");
        copy_dir_filter(&link, &destination, |_| true)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(destination.join("saves/level.dat")).unwrap(),
            b"world"
        );
    }

    #[tokio::test]
    async fn skips_the_source_folder_that_holds_the_destination() {
        let root = fresh("holder");
        let source = root.join("minecraft");
        std::fs::create_dir_all(source.join("saves/world")).unwrap();
        std::fs::write(source.join("saves/world/level.dat"), b"world").unwrap();
        std::fs::write(source.join("options.txt"), b"options").unwrap();
        let data = source.join("gdl");
        std::fs::create_dir_all(data.join("instances/other/instance")).unwrap();
        std::fs::write(data.join("instances/other/instance/options.txt"), b"other").unwrap();
        let existing = data.join("temp/1-0/instance");
        std::fs::create_dir_all(&existing).unwrap();
        // Not created yet: only part of the destination can be resolved.
        let missing = data.join("temp/1-1/instance");
        for destination in [existing, missing] {
            tokio::time::timeout(
                Duration::from_secs(20),
                copy_dir_filter(&source, &destination, |_| true),
            )
            .await
            .expect("the copy must not walk into its own output")
            .unwrap();
            assert_eq!(
                std::fs::read(destination.join("options.txt")).unwrap(),
                b"options"
            );
            assert_eq!(
                std::fs::read(destination.join("saves/world/level.dat")).unwrap(),
                b"world"
            );
            assert!(!destination.join("gdl").exists());
        }
    }

    #[tokio::test]
    async fn refuses_to_copy_a_folder_onto_itself() {
        let root = fresh("self");
        let source = root.join("source");
        std::fs::create_dir_all(source.join("saves")).unwrap();
        std::fs::write(source.join("saves/level.dat"), b"world").unwrap();
        assert!(copy_dir_filter(&source, &source, |_| true).await.is_err());
        assert!(
            copy_dir_filter(&source.join("saves"), &source, |_| true)
                .await
                .is_err()
        );
        assert_eq!(
            std::fs::read(source.join("saves/level.dat")).unwrap(),
            b"world"
        );
    }

    async fn expect_prompt_error<T: tempentry::TempEntryType>(temp: &TempPath) -> String {
        let result = tokio::time::timeout(Duration::from_secs(20), temp.maketmp::<T>())
            .await
            .expect("creating a temporary entry must not retry forever");
        format!(
            "{:#}",
            result.err().expect("a broken temp folder is an error")
        )
    }

    #[tokio::test]
    async fn a_temp_folder_that_is_a_file_fails_promptly() {
        let root = fresh("temp-file");
        let temp = root.join("temp");
        std::fs::write(&temp, b"not a folder").unwrap();
        let temp = TempPath::new(temp);
        let error = expect_prompt_error::<tempentry::File>(&temp).await;
        assert!(error.contains("not a usable folder"), "{error}");
        let error = expect_prompt_error::<tempentry::Folder>(&temp).await;
        assert!(error.contains("not a usable folder"), "{error}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_temp_folder_linked_to_a_missing_place_fails_promptly() {
        let root = fresh("temp-link");
        let temp = root.join("temp");
        std::os::unix::fs::symlink(root.join("unmounted"), &temp).unwrap();
        let temp = TempPath::new(temp);
        let error = expect_prompt_error::<tempentry::File>(&temp).await;
        assert!(error.contains("not a usable folder"), "{error}");
        let error = expect_prompt_error::<tempentry::Folder>(&temp).await;
        assert!(error.contains("not a usable folder"), "{error}");
    }

    #[tokio::test]
    async fn temp_entries_get_distinct_names() {
        let root = fresh("temp-ok");
        let temp = TempPath::new(root.join("temp"));
        let first = temp.maketmpdir().await.unwrap();
        let second = temp.maketmpdir().await.unwrap();
        assert!(first.is_dir() && second.is_dir());
        assert_ne!(&*first, &*second);
        let file = temp.maketmpfile().await.unwrap();
        assert!(file.parent().unwrap().is_dir());
    }
}
