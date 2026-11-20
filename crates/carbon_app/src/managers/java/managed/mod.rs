use self::{azul_zulu::AzulZulu, mojang::MojangRuntime};
use super::java_checker::{JavaChecker, RealJavaChecker};
use super::resolve::JavaRequirement;
use super::updates::{get_pinned_managed_javas, set_pinned_managed_javas};
use super::{build_version::JavaBuildVersion, scan_and_sync::upsert_java_component_to_db};
use crate::{
    api::keys::java::{GET_MANAGED_JAVA_INSTALLS, GET_SETUP_MANAGED_JAVA_PROGRESS},
    domain::java::{JavaArch, JavaComponentType, JavaOs, JavaVendor, JavaVersion},
};
use anyhow::Context;
use carbon_repos::db_exec::Db;
use carbon_rt_path::{ManagedJavasPath, TempPath};
use std::{collections::HashMap, path::Path, sync::Arc, time::Instant};
use strum::IntoEnumIterator;
use tokio::sync::{
    Mutex,
    watch::{self, Sender},
};

pub mod azul_zulu;
pub mod mojang;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Step {
    #[default]
    Idle,
    Downloading(u64, u64),
    Extracting(u64, u64),
    Done,
}

/// An install that is running right now.
#[derive(Debug, Clone)]
pub struct InstallProgress {
    pub step: Step,
    pub vendor: JavaVendor,
    pub major: u8,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ManagedJavaBuild {
    /// Azul `package_uuid`, or the sha1 of Mojang's component manifest.
    pub id: String,
    pub name: String,
    pub major: u8,
    pub version: JavaBuildVersion,
    pub source: BuildSource,
    pub size_bytes: Option<u64>,
    /// Azul `build_date`, ISO 8601.
    pub release_date: Option<String>,
    pub lts: bool,
}

#[derive(Debug, Clone)]
pub enum BuildSource {
    AzulZip {
        download_url: String,
        sha256: Option<String>,
    },
    MojangManifest {
        component: String,
        manifest_url: String,
        manifest_sha1: String,
    },
}

impl ManagedJavaBuild {
    pub fn java_version(&self) -> JavaVersion {
        let components = self.version.components();
        let component = |i: usize| components.get(i).copied().unwrap_or(0);

        JavaVersion {
            major: u16::from(self.major),
            minor: component(1) as u16,
            patch: component(2).to_string(),
            update_number: None,
            prerelease: None,
            build_metadata: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManagedJavaMajor {
    pub major: u8,
    pub lts: bool,
    /// Newest first.
    pub builds: Vec<ManagedJavaBuild>,
}

#[derive(Debug, Clone, Default)]
pub struct ManagedJavaCatalog {
    /// Newest first.
    pub majors: Vec<ManagedJavaMajor>,
}

impl ManagedJavaCatalog {
    pub fn from_builds(builds: Vec<ManagedJavaBuild>) -> Self {
        let mut majors: Vec<ManagedJavaMajor> = Vec::new();

        for build in builds {
            match majors.iter_mut().find(|m| m.major == build.major) {
                Some(major) => {
                    major.lts |= build.lts;
                    major.builds.push(build);
                }
                None => majors.push(ManagedJavaMajor {
                    major: build.major,
                    lts: build.lts,
                    builds: vec![build],
                }),
            }
        }

        for major in &mut majors {
            major.builds.sort_by(|a, b| b.version.cmp(&a.version));
        }
        majors.sort_by(|a, b| b.major.cmp(&a.major));

        Self { majors }
    }

    pub fn latest_for_major(&self, major: u8) -> Option<&ManagedJavaBuild> {
        self.majors
            .iter()
            .find(|m| m.major == major)
            .and_then(|m| m.builds.first())
    }

    pub fn find(&self, id: &str) -> Option<&ManagedJavaBuild> {
        self.builds().find(|b| b.id == id)
    }

    /// Newest first across all majors.
    pub fn builds(&self) -> impl Iterator<Item = &ManagedJavaBuild> {
        self.majors.iter().flat_map(|m| m.builds.iter())
    }
}

/// The first build, newest first, that meets the requirement.
pub fn pick_auto_install_build<'a>(
    catalog: &'a ManagedJavaCatalog,
    req: &JavaRequirement,
) -> Option<&'a ManagedJavaBuild> {
    catalog
        .builds()
        .find(|build| req.accepts(build.major, &build.version))
}

/// Another install of the same build is running.
#[derive(Debug, thiserror::Error)]
#[error("This Java is already being installed")]
pub struct AlreadyInstalling {
    pub build_id: String,
}

/// How long another install may go without progress before a launch stops
/// waiting for it.
const INSTALL_STALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
const INSTALL_WAIT_POLL: std::time::Duration = std::time::Duration::from_millis(250);

type InstallsInProgress = Arc<Mutex<HashMap<String, InstallProgress>>>;

/// Marks a build as installing for as long as it lives. Dropping it, as when
/// the request running the install goes away part-way, frees the build, so it
/// never looks like it is still installing.
pub(crate) struct InstallSlot {
    installs: InstallsInProgress,
    /// `None` once freed.
    build_id: Option<String>,
    /// Told when the slot frees, so the list of running installs refreshes.
    app: Option<crate::App>,
}

impl std::fmt::Debug for InstallSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstallSlot")
            .field("build_id", &self.build_id)
            .finish_non_exhaustive()
    }
}

impl InstallSlot {
    fn notifying(mut self, app: crate::App) -> Self {
        self.app = Some(app);
        self
    }

    /// Frees the build now.
    async fn release(mut self) {
        if let Some(build_id) = self.build_id.take() {
            self.installs.lock().await.remove(&build_id);
            if let Some(app) = &self.app {
                app.invalidate(GET_MANAGED_JAVA_INSTALLS, None);
            }
        }
    }
}

impl Drop for InstallSlot {
    fn drop(&mut self) {
        let Some(build_id) = self.build_id.take() else {
            return;
        };
        tracing::debug!("install of java build {build_id} ended early, freeing it");

        // Free it at once when nothing holds the map, otherwise as soon as it
        // is free.
        match self.installs.try_lock() {
            Ok(mut installs) => {
                installs.remove(&build_id);
                if let Some(app) = &self.app {
                    app.invalidate(GET_MANAGED_JAVA_INSTALLS, None);
                }
            }
            Err(_) => {
                let installs = Arc::clone(&self.installs);
                let app = self.app.take();
                if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                    runtime.spawn(async move {
                        installs.lock().await.remove(&build_id);
                        if let Some(app) = app {
                            app.invalidate(GET_MANAGED_JAVA_INSTALLS, None);
                        }
                    });
                }
            }
        }
    }
}

/// Whether installing `build` should pin it: it is not the newest build of its
/// major, so auto-update must leave it alone.
pub(crate) fn should_pin(catalog: &ManagedJavaCatalog, build: &ManagedJavaBuild) -> bool {
    catalog
        .latest_for_major(build.major)
        .is_some_and(|latest| latest.id != build.id)
}

/// How long a fetched catalog is reused before its distribution is asked
/// again.
pub(crate) const CATALOG_TTL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// How long after a background refresh of the catalogs another may start,
/// so an offline launcher doesn't retry on every refresh of a list.
const BACKGROUND_REFRESH_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(5 * 60);

type CachedCatalog = Option<(Instant, ManagedJavaCatalog)>;

/// What the catalogs tell their age and the refresh cooldown by: the time
/// now, which tests move ahead to age them. Going back from now instead
/// can't work everywhere, since a Windows `Instant` can't be older than the
/// computer's start.
#[derive(Debug, Clone, Default)]
pub(crate) struct Clock {
    #[cfg(test)]
    ahead: Arc<std::sync::Mutex<std::time::Duration>>,
}

impl Clock {
    pub(crate) fn now(&self) -> Instant {
        let now = Instant::now();
        #[cfg(test)]
        let now = now
            + *self
                .ahead
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        now
    }

    /// How long ago `then` was.
    fn since(&self, then: Instant) -> std::time::Duration {
        self.now().saturating_duration_since(then)
    }

    /// For tests: moves every clock sharing this one ahead by `by`.
    #[cfg(test)]
    pub(crate) fn advance(&self, by: std::time::Duration) {
        *self
            .ahead
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) += by;
    }
}

/// A distribution's last catalog. One fetch runs at a time, and it never
/// holds up a reader of the last catalog. A failed fetch isn't kept, so the
/// next try fetches again.
#[derive(Debug, Default)]
pub(crate) struct CatalogCache {
    last: std::sync::RwLock<CachedCatalog>,
    /// Held for as long as a fetch runs.
    fetching: Mutex<()>,
    clock: Clock,
}

impl CatalogCache {
    /// Tells the catalog's age by `clock`.
    pub(crate) fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    /// The last catalog fetched, however old, without waiting.
    pub(crate) fn last(&self) -> Option<ManagedJavaCatalog> {
        self.read().as_ref().map(|(_, catalog)| catalog.clone())
    }

    /// Whether it was never fetched or is older than [`CATALOG_TTL`].
    pub(crate) fn is_stale(&self) -> bool {
        self.fresh().is_none()
    }

    fn fresh(&self) -> Option<ManagedJavaCatalog> {
        self.read()
            .as_ref()
            .filter(|(fetched_at, _)| self.clock.since(*fetched_at) < CATALOG_TTL)
            .map(|(_, catalog)| catalog.clone())
    }

    /// The catalog, from `fetch` when the last one is missing or stale.
    pub(crate) async fn get_or_fetch<F, Fut>(&self, fetch: F) -> anyhow::Result<ManagedJavaCatalog>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<ManagedJavaCatalog>>,
    {
        if let Some(catalog) = self.fresh() {
            return Ok(catalog);
        }

        let _fetching = self.fetching.lock().await;
        // Another caller may have fetched it while this one waited.
        if let Some(catalog) = self.fresh() {
            return Ok(catalog);
        }

        let catalog = fetch().await?;
        *self.write() = Some((self.clock.now(), catalog.clone()));

        Ok(catalog)
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, CachedCatalog> {
        self.last
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, CachedCatalog> {
        self.last
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// For tests: `catalog`, as if fetched now.
    #[cfg(test)]
    pub(crate) fn set_for_test(&self, catalog: ManagedJavaCatalog) {
        *self.write() = Some((self.clock.now(), catalog));
    }

    /// For tests: a fetch that never ends, until the guard is dropped.
    #[cfg(test)]
    pub(crate) async fn hold_fetch_for_test(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.fetching.lock().await
    }
}

#[derive(Debug, Default)]
struct BackgroundRefresh {
    running: bool,
    last_ended: Option<Instant>,
}

/// The catalogs' background refresh, claimed until dropped.
pub(crate) struct RefreshClaim {
    refresh: Arc<std::sync::Mutex<BackgroundRefresh>>,
    clock: Clock,
}

impl Drop for RefreshClaim {
    fn drop(&mut self) {
        let mut refresh = self
            .refresh
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        refresh.running = false;
        refresh.last_ended = Some(self.clock.now());
    }
}

/// Probes the installed binary and records it as a managed Java.
pub(crate) async fn register_managed_java<G: JavaChecker + Send + Sync>(
    bin: &Path,
    java_checker: &G,
    db: &Db,
) -> anyhow::Result<String> {
    let java_component = java_checker
        .get_bin_info(bin, JavaComponentType::Managed)
        .await
        .with_context(|| format!("Could not get bin info for main binary: {bin:?}"))?;

    upsert_java_component_to_db(db, java_component).await
}

#[async_trait::async_trait]
pub trait Managed {
    /// Builds for the current OS and arch only, cached for 6 hours.
    async fn catalog(&self) -> anyhow::Result<ManagedJavaCatalog>;

    async fn setup<G: JavaChecker + Send + Sync>(
        &self,
        build: &ManagedJavaBuild,
        tmp_path: TempPath,
        base_managed_java_path: ManagedJavasPath,
        java_checker: &G,
        db: &Db,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<String>;
}

#[derive(Debug)]
pub struct ManagedService {
    azul_zulu: AzulZulu,
    mojang: MojangRuntime,
    /// Running installs, keyed by build id. Entries are removed when the install ends.
    pub setup_progress: InstallsInProgress,
    background_refresh: Arc<std::sync::Mutex<BackgroundRefresh>>,
    /// Shared by both catalogs and the background refresh.
    clock: Clock,
}

impl Default for ManagedService {
    fn default() -> Self {
        Self::from_runtimes(AzulZulu::default(), MojangRuntime::default())
    }
}

impl ManagedService {
    pub fn new() -> Self {
        Self::default()
    }

    /// For tests that need a catalog served by a mock.
    #[cfg(test)]
    pub fn with_runtimes(azul_zulu: AzulZulu, mojang: MojangRuntime) -> Self {
        Self::from_runtimes(azul_zulu, mojang)
    }

    fn from_runtimes(mut azul_zulu: AzulZulu, mut mojang: MojangRuntime) -> Self {
        let clock = Clock::default();
        azul_zulu.catalog.set_clock(clock.clone());
        mojang.catalog.set_clock(clock.clone());

        Self {
            azul_zulu,
            mojang,
            setup_progress: Default::default(),
            background_refresh: Default::default(),
            clock,
        }
    }

    /// For tests: ages both catalogs and the refresh cooldown by `by`.
    #[cfg(test)]
    pub fn advance_clock_for_test(&self, by: std::time::Duration) {
        self.clock.advance(by);
    }

    pub fn get_all_os(&self) -> Vec<JavaOs> {
        JavaOs::iter().collect()
    }

    pub fn get_all_archs(&self) -> Vec<JavaArch> {
        JavaArch::iter().collect()
    }

    pub fn get_all_vendors(&self) -> Vec<JavaVendor> {
        JavaVendor::iter().collect()
    }

    pub async fn get_catalog(&self, vendor: JavaVendor) -> anyhow::Result<ManagedJavaCatalog> {
        match vendor {
            JavaVendor::Azul => self.azul_zulu.catalog().await,
            JavaVendor::Mojang => self.mojang.catalog().await,
        }
    }

    /// The last catalog fetched for `vendor`, however old, without fetching
    /// one or waiting for one being fetched: for what must not wait on the
    /// network.
    pub fn cached_catalog(&self, vendor: JavaVendor) -> Option<ManagedJavaCatalog> {
        match vendor {
            JavaVendor::Azul => self.azul_zulu.cached_catalog(),
            JavaVendor::Mojang => self.mojang.cached_catalog(),
        }
    }

    /// The catalog of every distribution that can be fetched right now, all
    /// fetched at once. One that fails is logged and left out.
    pub async fn get_catalogs(&self) -> HashMap<JavaVendor, ManagedJavaCatalog> {
        let fetches = JavaVendor::iter().map(|vendor| async move {
            match self.get_catalog(vendor).await {
                Ok(catalog) => Some((vendor, catalog)),
                Err(e) => {
                    tracing::warn!("could not fetch the {vendor:?} java catalog: {e:#}");
                    None
                }
            }
        });

        futures::future::join_all(fetches)
            .await
            .into_iter()
            .flatten()
            .collect()
    }

    /// The last catalog of every distribution fetched, however old, without
    /// fetching or waiting.
    pub fn cached_catalogs(&self) -> HashMap<JavaVendor, ManagedJavaCatalog> {
        JavaVendor::iter()
            .filter_map(|vendor| Some((vendor, self.cached_catalog(vendor)?)))
            .collect()
    }

    fn catalogs_stale(&self) -> bool {
        !self.stale_vendors().is_empty()
    }

    /// The distributions whose catalog was never fetched or is stale, so
    /// asking for it fetches it.
    pub(crate) fn stale_vendors(&self) -> Vec<JavaVendor> {
        JavaVendor::iter()
            .filter(|vendor| match vendor {
                JavaVendor::Azul => self.azul_zulu.catalog_is_stale(),
                JavaVendor::Mojang => self.mojang.catalog_is_stale(),
            })
            .collect()
    }

    /// Claims the catalogs' background refresh when one was never fetched or
    /// is stale, no refresh runs, and the last ended at least
    /// [`BACKGROUND_REFRESH_COOLDOWN`] ago.
    pub(crate) fn claim_stale_refresh(&self) -> Option<RefreshClaim> {
        if !self.catalogs_stale() {
            return None;
        }

        let mut refresh = self
            .background_refresh
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cooling = refresh
            .last_ended
            .is_some_and(|ended| self.clock.since(ended) < BACKGROUND_REFRESH_COOLDOWN);
        if refresh.running || cooling {
            return None;
        }
        refresh.running = true;

        Some(RefreshClaim {
            refresh: Arc::clone(&self.background_refresh),
            clock: self.clock.clone(),
        })
    }

    /// For tests: serves `catalog` as the Azul catalog for the cache's lifetime.
    #[cfg(test)]
    pub async fn cache_azul_catalog_for_test(&self, catalog: ManagedJavaCatalog) {
        self.azul_zulu.cache_catalog_for_test(catalog).await;
    }

    /// For tests: serves `catalog` as the Mojang catalog for the cache's lifetime.
    #[cfg(test)]
    pub async fn cache_mojang_catalog_for_test(&self, catalog: ManagedJavaCatalog) {
        self.mojang.cache_catalog_for_test(catalog).await;
    }

    /// For tests: an Azul catalog fetch that never ends, until the returned
    /// guard is dropped.
    #[cfg(test)]
    pub async fn hold_azul_fetch_for_test(&self) -> impl Sized + '_ {
        self.azul_zulu.hold_fetch_for_test().await
    }

    /// Marks the build as installing until the returned slot is released or
    /// dropped. Fails when the same build is already installing, so a manual
    /// install cannot race a background update.
    pub(crate) async fn register_install(
        &self,
        vendor: JavaVendor,
        build: &ManagedJavaBuild,
    ) -> anyhow::Result<InstallSlot> {
        let mut progress = self.setup_progress.lock().await;

        if progress.contains_key(&build.id) {
            return Err(AlreadyInstalling {
                build_id: build.id.clone(),
            }
            .into());
        }

        progress.insert(
            build.id.clone(),
            InstallProgress {
                step: Step::Idle,
                vendor,
                major: build.major,
                version: build.version.to_string(),
            },
        );

        Ok(InstallSlot {
            installs: Arc::clone(&self.setup_progress),
            build_id: Some(build.id.clone()),
            app: None,
        })
    }

    /// Waits until no install of `build_id` is running, passing its progress
    /// on to `progress`. Fails when the install makes no progress for two
    /// minutes.
    pub async fn wait_for_install(
        &self,
        build_id: &str,
        progress: Option<&watch::Sender<Step>>,
    ) -> anyhow::Result<()> {
        self.wait_for_install_within(build_id, progress, INSTALL_STALL_TIMEOUT, INSTALL_WAIT_POLL)
            .await
    }

    async fn wait_for_install_within(
        &self,
        build_id: &str,
        progress: Option<&watch::Sender<Step>>,
        stall_timeout: std::time::Duration,
        poll: std::time::Duration,
    ) -> anyhow::Result<()> {
        let mut last_step = None;
        let mut last_change = std::time::Instant::now();

        loop {
            let step = self
                .setup_progress
                .lock()
                .await
                .get(build_id)
                .map(|install| install.step.clone());
            let Some(step) = step else {
                return Ok(());
            };

            if last_step.as_ref() != Some(&step) {
                if let Some(progress) = progress {
                    let _ = progress.send(step.clone());
                }
                last_step = Some(step);
                last_change = std::time::Instant::now();
            } else if last_change.elapsed() >= stall_timeout {
                anyhow::bail!("Another install of this Java stopped making progress");
            }

            tokio::time::sleep(poll).await;
        }
    }

    pub async fn setup_managed(
        &self,
        vendor: JavaVendor,
        id: String,
        app: crate::App,
        progress_sender: Option<watch::Sender<Step>>,
    ) -> anyhow::Result<String> {
        let java_id = {
            {
                let catalog = self.get_catalog(vendor).await?;
                let build = catalog
                    .find(&id)
                    .ok_or_else(|| anyhow::anyhow!("No version for id: {}", id))?;

                let tmp_path = app.settings_manager().runtime_path.get_temp();
                let base_managed_java_path =
                    app.settings_manager().runtime_path.get_managed_javas();
                let db = app.db.clone();

                let (sender, mut recv) = tokio::sync::watch::channel(Step::Idle);

                let slot = self
                    .register_install(vendor, build)
                    .await?
                    .notifying(app.clone());
                app.invalidate(GET_MANAGED_JAVA_INSTALLS, None);

                let progress_ref = Arc::clone(&self.setup_progress);
                let progress_app = app.clone();
                let build_id = build.id.clone();

                tokio::spawn(async move {
                    while recv.changed().await.is_ok() {
                        let step = recv.borrow().clone();
                        {
                            let mut progress_ref = progress_ref.lock().await;
                            // Gone means the install already ended.
                            let Some(entry) = progress_ref.get_mut(&build_id) else {
                                break;
                            };
                            entry.step = step.clone();
                        }
                        if let Some(progress_sender) = &progress_sender {
                            let _ = progress_sender.send(step);
                        }
                        progress_app.invalidate(
                            GET_SETUP_MANAGED_JAVA_PROGRESS,
                            Some(serde_json::json!(build_id)),
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                });

                let result = match vendor {
                    JavaVendor::Azul => {
                        self.azul_zulu
                            .setup(
                                build,
                                tmp_path,
                                base_managed_java_path,
                                &RealJavaChecker,
                                &db,
                                sender,
                            )
                            .await
                    }
                    JavaVendor::Mojang => {
                        self.mojang
                            .setup(
                                build,
                                tmp_path,
                                base_managed_java_path,
                                &RealJavaChecker,
                                &db,
                                sender,
                            )
                            .await
                    }
                };

                slot.release().await;

                if let Ok(new_java_id) = &result
                    && should_pin(&catalog, build)
                {
                    match get_pinned_managed_javas(&db).await {
                        Ok(mut pinned) => {
                            pinned.insert(new_java_id.clone());
                            if let Err(e) = set_pinned_managed_javas(&db, &pinned).await {
                                tracing::warn!("could not pin managed java {new_java_id}: {e:#}");
                            }
                        }
                        Err(e) => tracing::warn!(
                            "could not read pinned managed javas, not pinning {new_java_id}: {e:#}"
                        ),
                    }
                }
                app.invalidate(
                    GET_SETUP_MANAGED_JAVA_PROGRESS,
                    Some(serde_json::json!(build.id)),
                );

                result?
            }
        };

        Ok(java_id)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn build(id: &str, version: &[u32]) -> ManagedJavaBuild {
        ManagedJavaBuild {
            id: id.to_string(),
            name: format!("{id}.zip"),
            major: version[0] as u8,
            version: JavaBuildVersion::from_components(version),
            source: BuildSource::AzulZip {
                download_url: String::new(),
                sha256: None,
            },
            size_bytes: None,
            release_date: None,
            lts: false,
        }
    }

    /// Two servers that each answer 500, but only once both have a request.
    async fn serve_500_once_both_are_asked() -> (String, String) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let both = Arc::new(tokio::sync::Barrier::new(2));
        let mut urls = Vec::new();
        for _ in 0..2 {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            urls.push(format!("http://{}/", listener.local_addr().unwrap()));
            let both = Arc::clone(&both);
            tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 4096];
                let _ = socket.read(&mut request).await;
                both.wait().await;
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\
                          connection: close\r\n\r\n",
                    )
                    .await;
            });
        }

        (urls.remove(0), urls.remove(0))
    }

    #[tokio::test]
    async fn the_distributions_catalogs_are_fetched_together() {
        let (Ok(os), Ok(arch)) = (JavaOs::get_current_os(), JavaArch::get_current_arch()) else {
            return;
        };
        // Mojang has nothing to fetch for this platform.
        if mojang::mojang_platform_key(&os, &arch).is_none() {
            return;
        }
        let (azul, mojang) = serve_500_once_both_are_asked().await;
        let service = ManagedService::with_runtimes(
            AzulZulu::with_base_url(azul),
            MojangRuntime::with_base_url(mojang),
        );

        let catalogs =
            tokio::time::timeout(std::time::Duration::from_secs(5), service.get_catalogs())
                .await
                .expect("one distribution's fetch waited for the other's");

        assert!(catalogs.is_empty());
    }

    #[tokio::test]
    async fn a_stale_catalog_is_refreshed_once_per_cooldown() {
        let service = ManagedService::new();

        // Never fetched.
        let claim = service.claim_stale_refresh().expect("nothing fetched yet");
        assert!(service.claim_stale_refresh().is_none(), "one at a time");
        drop(claim);
        assert!(
            service.claim_stale_refresh().is_none(),
            "an offline launcher would retry on every refresh of a list"
        );

        service.advance_clock_for_test(BACKGROUND_REFRESH_COOLDOWN);
        assert!(
            service.claim_stale_refresh().is_some(),
            "after the cooldown"
        );
    }

    #[tokio::test]
    async fn fresh_catalogs_are_not_refreshed() {
        let service = ManagedService::new();
        service
            .cache_azul_catalog_for_test(ManagedJavaCatalog::default())
            .await;
        service
            .cache_mojang_catalog_for_test(ManagedJavaCatalog::default())
            .await;
        assert!(service.claim_stale_refresh().is_none());
        assert!(service.stale_vendors().is_empty());

        service.advance_clock_for_test(CATALOG_TTL + std::time::Duration::from_secs(1));
        assert_eq!(
            service.stale_vendors(),
            [JavaVendor::Azul, JavaVendor::Mojang]
        );
        assert!(service.claim_stale_refresh().is_some());
        // The stale ones still read.
        assert!(service.cached_catalogs().contains_key(&JavaVendor::Azul));
    }

    #[tokio::test]
    async fn the_same_build_cannot_be_installed_twice_at_once() {
        let service = ManagedService::new();
        let b = build("17-21", &[17, 0, 21, 1]);

        let _installing = service
            .register_install(JavaVendor::Azul, &b)
            .await
            .unwrap();
        let err = service
            .register_install(JavaVendor::Azul, &b)
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "This Java is already being installed");
        assert_eq!(
            err.downcast_ref::<AlreadyInstalling>()
                .map(|e| e.build_id.as_str()),
            Some("17-21")
        );

        service
            .register_install(JavaVendor::Azul, &build("21", &[21, 0, 13, 1]))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn waiting_for_an_install_ends_when_it_does_and_passes_progress_on() {
        let service = Arc::new(ManagedService::new());
        let b = build("21", &[21, 0, 13, 1]);
        let _installing = service
            .register_install(JavaVendor::Azul, &b)
            .await
            .unwrap();
        service
            .setup_progress
            .lock()
            .await
            .get_mut("21")
            .unwrap()
            .step = Step::Downloading(5, 10);

        let ender = Arc::clone(&service);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            ender.setup_progress.lock().await.remove("21");
        });

        let (tx, rx) = watch::channel(Step::Idle);
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            service.wait_for_install("21", Some(&tx)),
        )
        .await
        .expect("the wait should end with the install")
        .unwrap();

        assert!(matches!(*rx.borrow(), Step::Downloading(5, 10)));
        // Nothing running: returns at once.
        service.wait_for_install("21", None).await.unwrap();
    }

    const STALL: std::time::Duration = std::time::Duration::from_millis(300);
    const POLL: std::time::Duration = std::time::Duration::from_millis(20);

    #[tokio::test]
    async fn waiting_gives_up_when_the_install_stops_moving() {
        let service = ManagedService::new();
        let _installing = service
            .register_install(JavaVendor::Azul, &build("21", &[21, 0, 13, 1]))
            .await
            .unwrap();

        let waited = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            service.wait_for_install_within("21", None, STALL, POLL),
        )
        .await
        .expect("an install that never moves must not hold the wait");

        assert!(waited.is_err());
        // Giving up leaves the other install alone.
        assert!(service.setup_progress.lock().await.contains_key("21"));
    }

    #[tokio::test]
    async fn waiting_carries_on_while_the_install_moves() {
        let service = Arc::new(ManagedService::new());
        let installing = service
            .register_install(JavaVendor::Azul, &build("21", &[21, 0, 13, 1]))
            .await
            .unwrap();

        let mover = Arc::clone(&service);
        tokio::spawn(async move {
            // Moves for well over the stall window, then ends.
            for downloaded in 0..10 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                if let Some(install) = mover.setup_progress.lock().await.get_mut("21") {
                    install.step = Step::Downloading(downloaded, 10);
                }
            }
            drop(installing);
        });

        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            service.wait_for_install_within("21", None, STALL, POLL),
        )
        .await
        .expect("the wait should end with the install")
        .unwrap();
    }

    #[tokio::test]
    async fn a_dropped_install_frees_its_slot() {
        let service = ManagedService::new();
        let b = build("21", &[21, 0, 13, 1]);
        let installing = service
            .register_install(JavaVendor::Azul, &b)
            .await
            .unwrap();
        assert!(service.setup_progress.lock().await.contains_key("21"));

        // As when the request running `setup_managed` goes away part-way.
        drop(installing);

        assert!(!service.setup_progress.lock().await.contains_key("21"));
        let _again = service
            .register_install(JavaVendor::Azul, &b)
            .await
            .unwrap();
    }

    #[test]
    fn pick_auto_install_build_prefers_newest_compatible() {
        use crate::domain::java::SystemJavaProfileName;
        use crate::managers::java::resolve::requirement_for;

        let catalog = ManagedJavaCatalog::from_builds(vec![
            build("17-19", &[17, 0, 19]),
            build("21", &[21, 0, 13, 1]),
            build("17-21", &[17, 0, 21, 1]),
            build("8-302", &[8, 0, 302]),
            build("8-412", &[8, 0, 412]),
            build("8-292", &[8, 0, 292]),
        ]);

        let pick = |profile| pick_auto_install_build(&catalog, &requirement_for(profile));

        assert_eq!(pick(SystemJavaProfileName::Gamma).unwrap().id, "17-21");
        assert_eq!(
            pick(SystemJavaProfileName::LegacyFixed1).unwrap().id,
            "8-302"
        );
        assert_eq!(pick(SystemJavaProfileName::Legacy).unwrap().id, "8-412");
        assert!(pick(SystemJavaProfileName::Epsilon).is_none());
    }

    #[test]
    fn java_version_fills_major_minor_and_patch() {
        let version = build("x", &[17, 0, 21, 1]).java_version();
        assert_eq!(
            (version.major, version.minor, version.patch.as_str()),
            (17, 0, "21")
        );
    }
}
