//! Known-vulnerability advisories for Java builds, and the matching of
//! installed Javas against them.

use super::{build_version::JavaBuildVersion, instance_java::java_usage, shown_catalogs};
use crate::domain::java::JavaArch;
use crate::{
    App,
    api::keys::java::{GET_JAVA_ADVISORY_MATCHES, GET_MANAGED_CATALOG},
    managers::java::JavaManager,
};
use carbon_repos::{
    db_exec::Db,
    dbtypes::DbDateTime,
    repos::{frontend_preference, java::JavaRow},
};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvisoryFeed {
    pub schema: u32,
    pub advisories: Vec<JavaAdvisory>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaAdvisory {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub url: String,
    pub affected: Vec<AffectedRange>,
    #[serde(default)]
    pub arch: Vec<String>,
    #[serde(default)]
    pub cpu_features: Vec<String>,
    #[serde(default)]
    pub mitigation_jvm_args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedRange {
    pub major: u32,
    pub introduced: Option<String>,
    pub fixed: Option<String>,
}

pub trait HostCpu: Send + Sync {
    fn has_feature(&self, name: &str) -> bool;
}

pub struct RealHostCpu;

impl HostCpu for RealHostCpu {
    fn has_feature(&self, name: &str) -> bool {
        match name {
            "avx512vbmi" | "avx512f" | "avx512bw" | "bmi2" => detect_x86_feature(name),
            // Unrecognised names count as present, which errs toward warning.
            _ => true,
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn detect_x86_feature(name: &str) -> bool {
    match name {
        "avx512vbmi" => std::arch::is_x86_feature_detected!("avx512vbmi"),
        "avx512f" => std::arch::is_x86_feature_detected!("avx512f"),
        "avx512bw" => std::arch::is_x86_feature_detected!("avx512bw"),
        "bmi2" => std::arch::is_x86_feature_detected!("bmi2"),
        _ => true,
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn detect_x86_feature(_name: &str) -> bool {
    false
}

#[derive(Debug, Clone)]
pub struct AdvisoryMatch {
    pub java_id: String,
    pub advisory: JavaAdvisory,
    pub fixed: Option<JavaBuildVersion>,
}

/// An advisory match, with what decides whether to warn about it.
#[derive(Debug, Clone)]
pub struct AdvisoryMatchStatus {
    pub matched: AdvisoryMatch,
    /// Auto-update replaces the Java with a fixed build by itself.
    pub auto_fixable: bool,
    /// An instance uses the Java, directly or through Automatic, as the
    /// overview's "Used by" counts it.
    pub in_use: bool,
}

/// Whether `advisory` affects a Java of this version and arch on this host.
/// An unparsable bound makes a range unusable; it is skipped rather than
/// guessed at, so a bad feed entry never causes a false warning.
fn affects(
    advisory: &JavaAdvisory,
    version: &JavaBuildVersion,
    arch: JavaArch,
    cpu: &dyn HostCpu,
) -> bool {
    affecting_range(advisory, version, arch, cpu).is_some()
}

/// The fixed version of the first range that affects the Java; the outer
/// `Option` is `None` when nothing affects it.
fn affecting_range(
    advisory: &JavaAdvisory,
    version: &JavaBuildVersion,
    arch: JavaArch,
    cpu: &dyn HostCpu,
) -> Option<Option<JavaBuildVersion>> {
    if !advisory.arch.is_empty()
        && !advisory
            .arch
            .iter()
            .any(|a| JavaArch::try_from(a.as_str()).is_ok_and(|a| a == arch))
    {
        return None;
    }

    if !advisory.cpu_features.iter().all(|f| cpu.has_feature(f)) {
        return None;
    }

    for range in advisory
        .affected
        .iter()
        .filter(|r| r.major == version.major())
    {
        let introduced = match range.introduced.as_deref().map(JavaBuildVersion::parse) {
            Some(None) => continue,
            Some(Some(v)) => Some(v),
            None => None,
        };
        let fixed = match range.fixed.as_deref().map(JavaBuildVersion::parse) {
            Some(None) => continue,
            Some(Some(v)) => Some(v),
            None => None,
        };

        let after_start = introduced.as_ref().is_none_or(|i| *i <= *version);
        let before_fix = fixed.as_ref().is_none_or(|f| *version < *f);

        if after_start && before_fix {
            return Some(fixed);
        }
    }

    None
}

/// Returns every (java, advisory) pair where the Java is affected on this host.
/// Invalid rows and rows whose version or arch cannot be parsed are skipped.
pub fn match_advisories(
    javas: &[JavaRow],
    advisories: &[JavaAdvisory],
    cpu: &dyn HostCpu,
) -> Vec<AdvisoryMatch> {
    let mut out = Vec::new();

    for java in javas.iter().filter(|j| j.is_valid) {
        let Some(version) = JavaBuildVersion::parse(&java.full_version) else {
            continue;
        };
        let Ok(arch) = JavaArch::try_from(java.arch.as_str()) else {
            continue;
        };

        for advisory in advisories {
            if let Some(fixed) = affecting_range(advisory, &version, arch, cpu) {
                out.push(AdvisoryMatch {
                    java_id: java.id.clone(),
                    advisory: advisory.clone(),
                    fixed,
                });
            }
        }
    }

    out
}

/// Names of the advisories that affect a build, for the catalog.
pub fn known_issues_for_build(
    version: &JavaBuildVersion,
    arch: &str,
    advisories: &[JavaAdvisory],
    cpu: &dyn HostCpu,
) -> Vec<String> {
    let Ok(arch) = JavaArch::try_from(arch) else {
        return Vec::new();
    };

    advisories
        .iter()
        .filter(|a| affects(a, version, arch, cpu))
        .map(|a| a.name.clone())
        .collect()
}

/// JVM args that mitigate the advisories affecting a Java, deduplicated with
/// first-seen order kept. An unparsable version or arch yields nothing, so a
/// mitigation is never applied on a guess.
pub fn mitigation_args(
    version: &str,
    arch: &str,
    advisories: &[JavaAdvisory],
    cpu: &dyn HostCpu,
) -> Vec<String> {
    let version_str = version;
    let (Some(version), Ok(arch)) = (JavaBuildVersion::parse(version), JavaArch::try_from(arch))
    else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    let mut args = Vec::new();

    for advisory in advisories
        .iter()
        .filter(|a| affects(a, &version, arch, cpu))
    {
        ids.push(advisory.id.as_str());
        for arg in &advisory.mitigation_jvm_args {
            if seen.insert(arg.clone()) {
                args.push(arg.clone());
            }
        }
    }

    if !args.is_empty() {
        tracing::info!("Applying Java advisory mitigations {ids:?} for {version_str}");
    }

    args
}

/// Mitigation args for the installed Java at `java_path`, from the advisories
/// fetched this session. Any lookup failure yields no args: a mitigation must
/// never get in the way of starting the game.
pub async fn mitigation_args_for_path(app: &App, java_path: &std::path::Path) -> Vec<String> {
    let manager = app.java_manager();
    let advisories = manager.advisories.read().await;
    if advisories.is_empty() {
        return Vec::new();
    }

    let Ok(javas) = carbon_repos::repos::java::get_all_java(&app.db).await else {
        return Vec::new();
    };
    let Some(java) = javas
        .iter()
        .find(|j| std::path::Path::new(&j.path) == java_path)
    else {
        return Vec::new();
    };

    mitigation_args(&java.full_version, &java.arch, &advisories, &RealHostCpu)
}

const JAVA_ADVISORY_DISMISSED_KEY: &str = "java_advisory_dismissed";

/// Dismissed `"<advisoryId>|<javaId>"` keys. Missing or unparsable reads as empty.
pub async fn get_dismissed(db: &Db) -> anyhow::Result<HashSet<String>> {
    let pref = frontend_preference::get_preference(db, JAVA_ADVISORY_DISMISSED_KEY).await?;

    Ok(pref
        .and_then(|p| serde_json::from_str::<HashSet<String>>(&p.value).ok())
        .unwrap_or_default())
}

/// Merges `keys` into the dismissed set.
pub async fn add_dismissed(db: &Db, keys: Vec<String>) -> anyhow::Result<()> {
    let mut all = get_dismissed(db).await?;
    all.extend(keys);

    let mut sorted: Vec<&String> = all.iter().collect();
    sorted.sort();

    frontend_preference::upsert_preference(
        db,
        JAVA_ADVISORY_DISMISSED_KEY,
        &serde_json::to_string(&sorted)?,
        DbDateTime(chrono::Utc::now().fixed_offset()),
    )
    .await?;

    Ok(())
}

const MAX_FEED_BYTES: usize = 256 * 1024;

/// Fetches the feed. Anything wrong with it (status, size, JSON, schema) is
/// logged at debug level and reads as `None`. Advisories are remote only, so
/// the request skips the app's disk cache, which would also read the whole
/// body before the size cap below could stop it.
pub async fn fetch_feed(
    client: &reqwest_middleware::ClientWithMiddleware,
    url: &str,
) -> Option<AdvisoryFeed> {
    let request = client.get(url).header("avoid-caching", "");
    let mut resp = match request.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("could not fetch java advisories: {e}");
            return None;
        }
    };

    if !resp.status().is_success() {
        tracing::debug!("java advisories answered {}", resp.status());
        return None;
    }

    if resp
        .content_length()
        .is_some_and(|len| len > MAX_FEED_BYTES as u64)
    {
        tracing::debug!("java advisories feed is too large");
        return None;
    }

    let mut body = Vec::new();
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                body.extend_from_slice(&chunk);
                if body.len() > MAX_FEED_BYTES {
                    tracing::debug!("java advisories feed is too large");
                    return None;
                }
            }
            Ok(None) => break,
            Err(e) => {
                tracing::debug!("could not read java advisories: {e}");
                return None;
            }
        }
    }

    match serde_json::from_slice::<AdvisoryFeed>(&body) {
        Ok(feed) if feed.schema == 1 => Some(feed),
        Ok(feed) => {
            tracing::debug!("ignoring java advisories schema {}", feed.schema);
            None
        }
        Err(e) => {
            tracing::debug!("could not parse java advisories: {e}");
            None
        }
    }
}

/// Every known Java's advisory matches on this host.
pub async fn advisory_match_statuses(app: &App) -> anyhow::Result<Vec<AdvisoryMatchStatus>> {
    let manager = app.java_manager();
    let advisories = manager.advisories.read().await.clone();
    if advisories.is_empty() {
        return Ok(Vec::new());
    }

    let rows = carbon_repos::repos::java::get_all_java(&app.db).await?;
    let matches = match_advisories(&rows, &advisories, &RealHostCpu);
    if matches.is_empty() {
        return Ok(Vec::new());
    }

    let catalogs = shown_catalogs(app);
    let fixes = matches
        .iter()
        .map(|m| (m.java_id.clone(), m.fixed.clone()))
        .collect::<Vec<_>>();
    let fixable = manager.auto_fixable_ids(&catalogs, &fixes).await?;
    let usage = java_usage(app).await?;

    Ok(matches
        .into_iter()
        .map(|matched| AdvisoryMatchStatus {
            auto_fixable: fixable.contains(&matched.java_id),
            in_use: usage
                .get(&matched.java_id)
                .is_some_and(|users| !users.is_empty()),
            matched,
        })
        .collect())
}

fn feed_url() -> String {
    #[cfg(debug_assertions)]
    if let Ok(url) = std::env::var("GDL_ADVISORIES_URL") {
        if !url.is_empty() {
            return url;
        }
    }

    match advisories_url(env!("META_BASE_URL")) {
        Ok(url) => url.to_string(),
        Err(e) => {
            tracing::warn!("the java advisories url is invalid: {e:#}");
            String::new()
        }
    }
}

/// The advisory feed under the meta base URL, with or without its trailing
/// slash: joining onto `…/meta` would otherwise replace `meta`.
fn advisories_url(meta_base_url: &str) -> anyhow::Result<reqwest::Url> {
    let mut base = reqwest::Url::parse(meta_base_url)?;
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }

    Ok(base.join("java/v1/advisories.json")?)
}

impl JavaManager {
    /// Replaces the stored list from the remote feed. On any failure the list
    /// stays as it was.
    pub async fn refresh_advisories(&self, app: &App) {
        self.refresh_advisories_from(app, &feed_url()).await;
    }

    pub(crate) async fn refresh_advisories_from(&self, app: &App, url: &str) {
        let Some(feed) = fetch_feed(&app.reqwest_client, url).await else {
            return;
        };

        *self.advisories.write().await = feed.advisories;
        app.invalidate(GET_JAVA_ADVISORY_MATCHES, None);
        app.invalidate(GET_MANAGED_CATALOG, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("testdata/java4shell.json");

    struct FakeCpu {
        vbmi: bool,
    }

    impl HostCpu for FakeCpu {
        fn has_feature(&self, name: &str) -> bool {
            match name {
                "avx512vbmi" => self.vbmi,
                _ => true,
            }
        }
    }

    fn feed() -> AdvisoryFeed {
        serde_json::from_str(FIXTURE).unwrap()
    }

    fn row(version: &str, arch: &str, valid: bool) -> JavaRow {
        JavaRow {
            id: format!("java-{version}-{arch}"),
            path: "/opt/java".into(),
            major: JavaBuildVersion::parse(version).unwrap().major() as i32,
            full_version: version.into(),
            r#type: "local".into(),
            os: "linux".into(),
            arch: arch.into(),
            vendor: "Azul".into(),
            is_valid: valid,
        }
    }

    fn run(r: JavaRow, vbmi: bool) -> Vec<AdvisoryMatch> {
        match_advisories(&[r], &feed().advisories, &FakeCpu { vbmi })
    }

    #[test]
    fn fixture_parses_and_contains_java4shell() {
        let feed = feed();
        assert_eq!(feed.schema, 1);
        let a = &feed.advisories[0];
        assert_eq!(a.id, "CVE-2026-114938");
        assert_eq!(a.name, "Java4Shell");
        assert_eq!(a.affected.len(), 4);
        assert_eq!(a.arch, vec!["x86_64"]);
        assert_eq!(a.cpu_features, vec!["avx512vbmi"]);
        assert_eq!(a.mitigation_jvm_args, vec!["-XX:-UseBASE64Intrinsics"]);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{ "schema": 1, "extra": true, "advisories": [ {
            "id": "X", "name": "N", "severity": "low", "url": "u", "note": "hi",
            "affected": [ { "major": 8, "label": "y" } ] } ] }"#;
        let feed: AdvisoryFeed = serde_json::from_str(json).unwrap();
        assert_eq!(feed.advisories.len(), 1);
        assert!(feed.advisories[0].arch.is_empty());
    }

    #[test]
    fn affected_build_matches_with_fixed_version() {
        let m = run(row("17.0.20.1", "x86_64", true), true);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].fixed.as_ref().unwrap().to_string(), "17.0.21.1");
        assert_eq!(m[0].advisory.id, "CVE-2026-114938");
    }

    #[test]
    fn fixed_build_does_not_match() {
        assert!(run(row("17.0.21.1", "x86_64", true), true).is_empty());
    }

    #[test]
    fn before_introduced_does_not_match() {
        assert!(run(row("17.0.5", "x86_64", true), true).is_empty());
    }

    #[test]
    fn open_start_range_matches() {
        assert_eq!(run(row("21.0.13", "x86_64", true), true).len(), 1);
        assert_eq!(run(row("21.0.13.0.3", "x86_64", true), true).len(), 1);
    }

    #[test]
    fn unlisted_major_does_not_match() {
        assert!(run(row("11.0.30", "x86_64", true), true).is_empty());
    }

    #[test]
    fn arm_java_does_not_match() {
        assert!(run(row("17.0.20.1", "arm64", true), true).is_empty());
        assert!(run(row("17.0.20.1", "aarch64", true), true).is_empty());
    }

    #[test]
    fn arch_aliases_are_normalised() {
        assert_eq!(run(row("17.0.20.1", "amd64", true), true).len(), 1);
        assert_eq!(run(row("17.0.20.1", "x64", true), true).len(), 1);
    }

    #[test]
    fn host_without_feature_does_not_match() {
        assert!(run(row("17.0.20.1", "x86_64", true), false).is_empty());
    }

    #[test]
    fn invalid_row_is_skipped() {
        assert!(run(row("17.0.20.1", "x86_64", false), true).is_empty());
    }

    #[test]
    fn range_without_fixed_is_unfixed() {
        let json = r#"{ "schema": 1, "advisories": [ {
            "id": "X", "name": "N", "severity": "low", "url": "u",
            "affected": [ { "major": 22 } ] } ] }"#;
        let feed: AdvisoryFeed = serde_json::from_str(json).unwrap();
        let m = match_advisories(
            &[row("22.0.1", "arm64", true)],
            &feed.advisories,
            &FakeCpu { vbmi: false },
        );
        assert_eq!(m.len(), 1);
        assert!(m[0].fixed.is_none());
    }

    #[test]
    fn unknown_cpu_feature_counts_as_present() {
        assert!(RealHostCpu.has_feature("some-new-feature"));
    }

    async fn serve(body: &str, status: usize) -> (mockito::ServerGuard, mockito::Mock, String) {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/advisories.json")
            .with_status(status)
            .with_body(body)
            .create_async()
            .await;
        let url = format!("{}/advisories.json", server.url());
        (server, mock, url)
    }

    fn client() -> reqwest_middleware::ClientWithMiddleware {
        reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build()
    }

    #[tokio::test]
    async fn fetch_feed_accepts_valid_feed() {
        let (_s, _m, url) = serve(FIXTURE, 200).await;
        let feed = fetch_feed(&client(), &url).await.unwrap();
        assert_eq!(feed.advisories[0].name, "Java4Shell");
    }

    #[tokio::test]
    async fn fetch_feed_rejects_404() {
        let (_s, _m, url) = serve("{}", 404).await;
        assert!(fetch_feed(&client(), &url).await.is_none());
    }

    #[tokio::test]
    async fn fetch_feed_rejects_oversized_body() {
        let padding = " ".repeat(300 * 1024);
        let body = format!("{FIXTURE}{padding}");
        let (_s, _m, url) = serve(&body, 200).await;
        assert!(fetch_feed(&client(), &url).await.is_none());
    }

    #[tokio::test]
    async fn fetch_feed_rejects_malformed_json() {
        let (_s, _m, url) = serve("{ not json", 200).await;
        assert!(fetch_feed(&client(), &url).await.is_none());
    }

    #[tokio::test]
    async fn fetch_feed_rejects_unknown_schema() {
        let (_s, _m, url) = serve(r#"{ "schema": 2, "advisories": [] }"#, 200).await;
        assert!(fetch_feed(&client(), &url).await.is_none());
    }

    #[tokio::test]
    async fn failed_refresh_keeps_previous_list() {
        let app = crate::setup_managers_for_test().await;
        let manager = app.java_manager();
        assert!(manager.advisories.read().await.is_empty());

        let mut server = mockito::Server::new_async().await;
        let ok = server
            .mock("GET", "/advisories.json")
            .with_status(200)
            .with_body(FIXTURE)
            .create_async()
            .await;
        let url = format!("{}/advisories.json", server.url());

        manager.refresh_advisories_from(&app.app, &url).await;
        assert_eq!(manager.advisories.read().await[0].name, "Java4Shell");

        ok.remove_async().await;
        server
            .mock("GET", "/advisories.json")
            .with_status(500)
            .create_async()
            .await;

        manager.refresh_advisories_from(&app.app, &url).await;
        assert_eq!(manager.advisories.read().await[0].name, "Java4Shell");
    }

    /// The CDN sends `Cache-Control: max-age=3600`, which the app's caching
    /// client would keep in its disk cache. Advisories are remote only.
    #[tokio::test]
    async fn the_feed_is_never_kept_on_disk() {
        let app = crate::setup_managers_for_test().await;
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/advisories.json")
            .with_status(200)
            .with_header("cache-control", "max-age=3600")
            .with_header("etag", "\"v1\"")
            .with_body(FIXTURE)
            .create_async()
            .await;
        let url = format!("{}/advisories.json", server.url());

        app.java_manager()
            .refresh_advisories_from(&app.app, &url)
            .await;

        assert_eq!(
            app.java_manager().advisories.read().await[0].name,
            "Java4Shell"
        );
        let cached = carbon_repos::repos::http_cache::get_cached(&app.db, &url)
            .await
            .unwrap();
        assert!(cached.is_none(), "the feed was cached on disk");
    }

    /// Serves a feed body one byte over the cap, then never ends it.
    async fn serve_endless_oversized_feed() -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let _ = socket.read(&mut request).await;
            let chunk = vec![b' '; MAX_FEED_BYTES + 1];
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                      cache-control: max-age=3600\r\ntransfer-encoding: chunked\r\n\r\n",
                )
                .await
                .unwrap();
            socket
                .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                .await
                .unwrap();
            socket.write_all(&chunk).await.unwrap();
            socket.write_all(b"\r\n").await.unwrap();
            std::future::pending::<()>().await;
        });

        format!("http://{addr}/advisories.json")
    }

    #[tokio::test]
    async fn the_size_cap_holds_while_the_feed_streams() {
        let app = crate::setup_managers_for_test().await;
        let url = serve_endless_oversized_feed().await;

        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            app.java_manager().refresh_advisories_from(&app.app, &url),
        )
        .await
        .expect("the feed should be dropped as soon as it passes the cap");

        assert!(app.java_manager().advisories.read().await.is_empty());
    }

    #[test]
    fn the_feed_url_holds_with_or_without_a_trailing_slash() {
        for (base, url) in [
            (
                "https://meta.gdl.gg/",
                "https://meta.gdl.gg/java/v1/advisories.json",
            ),
            (
                "https://meta.gdl.gg",
                "https://meta.gdl.gg/java/v1/advisories.json",
            ),
            (
                "http://localhost:9005/meta/",
                "http://localhost:9005/meta/java/v1/advisories.json",
            ),
            (
                "http://localhost:9005/meta",
                "http://localhost:9005/meta/java/v1/advisories.json",
            ),
        ] {
            assert_eq!(
                advisories_url(base).map(|u| u.to_string()).ok().as_deref(),
                Some(url),
                "{base}"
            );
        }
    }

    #[tokio::test]
    async fn dismissals_round_trip() {
        let app = crate::setup_managers_for_test().await;
        assert!(get_dismissed(&app.db).await.unwrap().is_empty());

        add_dismissed(&app.db, vec!["CVE-1|a".into()])
            .await
            .unwrap();
        add_dismissed(&app.db, vec!["CVE-2|b".into()])
            .await
            .unwrap();

        assert_eq!(
            get_dismissed(&app.db).await.unwrap(),
            HashSet::from(["CVE-1|a".to_string(), "CVE-2|b".to_string()])
        );
    }

    #[test]
    fn known_issues_for_build_flags_affected_builds() {
        let advisories = feed().advisories;
        let cpu = FakeCpu { vbmi: true };
        let v = |s| JavaBuildVersion::parse(s).unwrap();

        assert_eq!(
            known_issues_for_build(&v("21.0.7"), "x86_64", &advisories, &cpu),
            vec!["Java4Shell".to_string()]
        );
        assert!(known_issues_for_build(&v("21.0.13.1"), "x86_64", &advisories, &cpu).is_empty());
        assert!(known_issues_for_build(&v("21.0.7"), "arm64", &advisories, &cpu).is_empty());
    }

    #[test]
    fn mitigation_args_for_affected_build_on_vbmi_host() {
        let advisories = feed().advisories;
        let cpu = FakeCpu { vbmi: true };

        assert_eq!(
            mitigation_args("17.0.20.1", "x86_64", &advisories, &cpu),
            vec!["-XX:-UseBASE64Intrinsics".to_string()]
        );
        assert!(mitigation_args("17.0.21.1", "x86_64", &advisories, &cpu).is_empty());
        assert!(mitigation_args("17.0.20.1", "arm64", &advisories, &cpu).is_empty());
        assert!(mitigation_args("garbage", "x86_64", &advisories, &cpu).is_empty());
    }

    #[test]
    fn mitigation_args_empty_without_vbmi() {
        let advisories = feed().advisories;
        assert!(
            mitigation_args("17.0.20.1", "x86_64", &advisories, &FakeCpu { vbmi: false })
                .is_empty()
        );
    }

    fn unfixed(id: &str, major: u32) -> JavaAdvisory {
        JavaAdvisory {
            id: id.into(),
            name: id.into(),
            severity: "critical".into(),
            url: "https://example.com".into(),
            affected: vec![AffectedRange {
                major,
                introduced: None,
                fixed: None,
            }],
            arch: vec![],
            cpu_features: vec![],
            mitigation_jvm_args: vec![],
        }
    }

    #[tokio::test]
    async fn matches_say_whether_an_instance_uses_the_java() {
        use crate::managers::java::test_support::{Fixture, fixture};

        let Fixture { app, .. } = fixture().await;
        *app.java_manager().advisories.write().await =
            vec![unfixed("CVE-21", 21), unfixed("CVE-22", 22)];

        let statuses = advisory_match_statuses(&app.app).await.unwrap();
        let in_use = |id: &str| {
            statuses
                .iter()
                .find(|s| s.matched.java_id == id)
                .unwrap_or_else(|| panic!("no match for {id}"))
                .in_use
        };

        // "Pathed" sets the found 21 directly.
        assert!(in_use("l21"));
        // "Auto" gets the newest managed 21 through Automatic.
        assert!(in_use("m21-new"));
        assert!(!in_use("m21-old"));
        assert!(!in_use("l22"));
    }

    /// The matches are queried all session and recomputed on every instance
    /// change, so they never wait on a catalog fetch.
    #[tokio::test]
    async fn matches_never_wait_on_a_catalog_fetch() {
        use crate::managers::java::test_support::{Fixture, fixture};

        let Fixture { app, .. } = fixture().await;
        *app.java_manager().advisories.write().await = vec![unfixed("CVE-21", 21)];
        let java_manager = app.java_manager();
        let _fetching = java_manager
            .managed_service
            .hold_azul_fetch_for_test()
            .await;

        let statuses = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            advisory_match_statuses(&app.app),
        )
        .await
        .expect("the matches waited on the catalog fetch")
        .unwrap();

        assert!(!statuses.is_empty());
    }

    #[tokio::test]
    async fn matches_go_stale_with_what_decides_usage() {
        use crate::api::keys::{instance::GET_ALL_INSTANCES, java::GET_AVAILABLE_JAVAS};

        let mut app = crate::setup_managers_for_test().await;
        while app.invalidation_recv.try_recv().is_ok() {}

        for source in [GET_ALL_INSTANCES, GET_AVAILABLE_JAVAS] {
            app.app.invalidate(source, None);

            let mut keys = Vec::new();
            while let Ok(event) = app.invalidation_recv.try_recv() {
                keys.push(event.key);
            }
            assert!(
                keys.contains(&GET_JAVA_ADVISORY_MATCHES.full),
                "{source}: {keys:?}"
            );
        }
    }

    #[test]
    fn mitigation_args_are_deduplicated_in_first_seen_order() {
        let mut advisories = feed().advisories;
        let mut second = advisories[0].clone();
        second.id = "OTHER".into();
        second.mitigation_jvm_args = vec!["-XX:-UseBASE64Intrinsics".into(), "-Dfoo=1".into()];
        advisories.push(second);

        assert_eq!(
            mitigation_args("17.0.20.1", "x86_64", &advisories, &FakeCpu { vbmi: true }),
            vec![
                "-XX:-UseBASE64Intrinsics".to_string(),
                "-Dfoo=1".to_string()
            ]
        );
    }
}
