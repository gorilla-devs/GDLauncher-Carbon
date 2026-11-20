//! State shared by the overview, instance Java and Settings › Java action
//! tests: a fixed manifest and catalogs, so nothing goes online.

pub(crate) use crate::domain::{
    instance::{
        InstanceId,
        info::{GameVersion, JavaOverride},
    },
    java::{JavaComponentType, SystemJavaProfileName},
};
use crate::{
    App,
    domain::instance::info::{ModLoader, ModLoaderType, StandardVersion},
    managers::{
        instance::InstanceVersionSource,
        java::{
            build_version::JavaBuildVersion,
            instance_java::JavaUsage,
            managed::{BuildSource, ManagedJavaBuild, ManagedJavaCatalog},
            overview::{JavaEntry, JavaGroup, JavaOverview},
            resolve::host_arch,
        },
    },
    setup_managers_for_test,
};
pub(crate) use carbon_repos::repos::java::{self as java_repo, JavaRow};
use daedalus::minecraft::VersionManifest;

pub(crate) fn build(id: &str, version: &[u32]) -> ManagedJavaBuild {
    ManagedJavaBuild {
        id: id.into(),
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

/// The releases each Java serves in the tests, newest first.
pub(crate) const TEST_VERSIONS: [(&str, &str, &str); 9] = [
    ("26.1", "release", "java-runtime-epsilon"),
    ("26.1-snapshot-1", "snapshot", "java-runtime-epsilon"),
    ("1.21.4", "release", "java-runtime-delta"),
    ("1.20.5", "release", "java-runtime-delta"),
    ("1.20.4", "release", "java-runtime-gamma"),
    ("1.18", "release", "java-runtime-beta"),
    ("1.17", "release", "java-runtime-alpha"),
    ("1.16.5", "release", "jre-legacy"),
    ("1.0", "release", "jre-legacy"),
];

/// [`TEST_VERSIONS`] as Minecraft's manifest.
pub(crate) fn test_manifest() -> VersionManifest {
    manifest_of(&TEST_VERSIONS)
}

/// A manifest of `(id, type, java profile)` entries, newest first.
pub(crate) fn manifest_of(entries: &[(&str, &str, &str)]) -> VersionManifest {
    let versions: Vec<serde_json::Value> = entries
        .iter()
        .map(|(id, ty, profile)| {
            serde_json::json!({
                "id": id,
                "type": ty,
                "url": "",
                "time": "2026-01-01T00:00:00Z",
                "releaseTime": "2026-01-01T00:00:00Z",
                "sha1": "",
                "complianceLevel": 1,
                "javaProfile": profile,
            })
        })
        .collect();

    serde_json::from_value(serde_json::json!({
        "latest": { "release": "26.1", "snapshot": "26.1-snapshot-1" },
        "versions": versions,
    }))
    .unwrap()
}

/// An app with the test manifest and catalogs, so nothing goes online.
pub(crate) async fn test_app(azul: Vec<ManagedJavaBuild>) -> crate::TestEnv {
    let app = setup_managers_for_test().await;
    *app.minecraft_manager().manifest_for_test.lock().unwrap() = Some(Ok(test_manifest()));
    cache_test_catalogs(&app.app, azul).await;
    app
}

/// Serves `azul` as the Azul catalog and an empty Mojang one.
pub(crate) async fn cache_test_catalogs(app: &App, azul: Vec<ManagedJavaBuild>) {
    let manager = app.java_manager();
    manager
        .managed_service
        .cache_azul_catalog_for_test(ManagedJavaCatalog::from_builds(azul))
        .await;
    manager
        .managed_service
        .cache_mojang_catalog_for_test(ManagedJavaCatalog::default())
        .await;
}

/// Makes the Minecraft manifest unreachable, as when offline.
pub(crate) fn manifest_unreachable(app: &App) {
    *app.minecraft_manager().manifest_for_test.lock().unwrap() = Some(Err("offline".into()));
}

pub(crate) fn majors(overview: &JavaOverview) -> Vec<u8> {
    overview.groups.iter().map(|group| group.major).collect()
}

/// Every shown group has a Java in use, or downloads one when needed.
pub(crate) fn assert_groups_consistent(overview: &JavaOverview) {
    for group in &overview.groups {
        assert_eq!(
            group.downloads_when_needed,
            group.in_use_java_id.is_none(),
            "{group:#?}"
        );
    }
}

pub(crate) fn major_of(version: &str) -> i32 {
    let first: i32 = version.split('.').next().unwrap().parse().unwrap();
    if first == 1 {
        version.split('.').nth(1).unwrap().parse().unwrap()
    } else {
        first
    }
}

/// A GDLauncher-installed Java whose install folder holds `size` bytes.
pub(crate) async fn add_managed(
    app: &App,
    id: &str,
    folder: &str,
    version: &str,
    size: usize,
) -> JavaRow {
    let root = app
        .settings_manager()
        .runtime_path
        .get_managed_javas()
        .to_path();
    let bin_dir = root.join(folder).join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let bin = bin_dir.join("java");
    std::fs::write(&bin, vec![0u8; size]).unwrap();

    let row = JavaRow {
        id: id.into(),
        path: bin.to_string_lossy().into_owned(),
        major: major_of(version),
        full_version: version.into(),
        r#type: "managed".into(),
        os: "linux".into(),
        arch: host_arch().to_string(),
        vendor: "Azul Systems, Inc.".into(),
        is_valid: true,
    };
    java_repo::insert_java(&app.db, row.clone()).await.unwrap();
    row
}

pub(crate) async fn add_other(app: &App, id: &str, ty: &str, path: &str, version: &str) -> JavaRow {
    let row = JavaRow {
        id: id.into(),
        path: path.into(),
        major: major_of(version),
        full_version: version.into(),
        r#type: ty.into(),
        os: "linux".into(),
        arch: host_arch().to_string(),
        vendor: "Private Build".into(),
        is_valid: true,
    };
    java_repo::insert_java(&app.db, row.clone()).await.unwrap();
    row
}

pub(crate) async fn add_instance(
    app: &App,
    name: &str,
    release: &str,
    forge: bool,
    java_override: Option<JavaOverride>,
) -> InstanceId {
    let manager = app.instance_manager();
    let group = manager.get_default_group().await.unwrap();
    let modloaders = if forge {
        [ModLoader {
            type_: ModLoaderType::Forge,
            version: "36.2.39".into(),
        }]
        .into_iter()
        .collect()
    } else {
        Default::default()
    };
    let id = manager
        .create_instance(
            group,
            name.into(),
            false,
            InstanceVersionSource::Version(GameVersion::Standard(StandardVersion {
                release: release.into(),
                modloaders,
            })),
            String::new(),
        )
        .await
        .unwrap();

    if java_override.is_some() {
        manager
            .update_instance(crate::domain::instance::InstanceSettingsUpdate {
                instance_id: id,
                name: None,
                use_loaded_icon: None,
                notes: None,
                version: None,
                modloader: None,
                java_override: Some(java_override),
                java_floor: None,
                global_java_args: None,
                extra_java_args: None,
                memory: None,
                pre_launch_hook: None,
                post_exit_hook: None,
                wrapper_command: None,
                game_resolution: None,
                mod_sources: None,
                modpack_locked: None,
            })
            .await
            .unwrap();
    }

    id
}

pub(crate) fn group(overview: &JavaOverview, major: u8) -> &JavaGroup {
    overview
        .groups
        .iter()
        .find(|g| g.major == major)
        .unwrap_or_else(|| panic!("no Java {major} group in {overview:#?}"))
}

pub(crate) fn entry<'a>(overview: &'a JavaOverview, id: &str) -> &'a JavaEntry {
    overview
        .groups
        .iter()
        .flat_map(|g| g.javas.iter())
        .chain(overview.other_javas.iter())
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no entry {id} in {overview:#?}"))
}

pub(crate) fn usage(name: &str, id: InstanceId, direct: bool) -> JavaUsage {
    JavaUsage {
        instance_id: id,
        name: name.into(),
        direct,
        bounded: false,
        via_profile: None,
    }
}

/// A managed 21.0.13.1 and 21.0.7, a found 21.0.8, a found 22, and two
/// 1.21.4 instances: one on Automatic, one set to the found 21.0.8.
pub(crate) struct Fixture {
    pub(crate) app: crate::TestEnv,
    pub(crate) auto: InstanceId,
    pub(crate) pathed: InstanceId,
    pub(crate) found21: JavaRow,
}

pub(crate) async fn fixture() -> Fixture {
    let app = test_app(vec![build("b21", &[21, 0, 13, 1])]).await;
    add_managed(&app.app, "m21-new", "zulu21.0.13.1-a", "21.0.13.1", 1000).await;
    add_managed(&app.app, "m21-old", "zulu21.0.7-b", "21.0.7", 700).await;
    let found21 = add_other(
        &app.app,
        "l21",
        "local",
        "/usr/lib/jvm/java-21/bin/java",
        "21.0.8",
    )
    .await;
    add_other(
        &app.app,
        "l22",
        "local",
        "/usr/lib/jvm/java-22/bin/java",
        "22.0.2",
    )
    .await;

    let auto = add_instance(&app.app, "Auto", "1.21.4", false, None).await;
    let pathed = add_instance(
        &app.app,
        "Pathed",
        "1.21.4",
        false,
        Some(JavaOverride::Path(Some(found21.path.clone()))),
    )
    .await;

    Fixture {
        app,
        auto,
        pathed,
        found21,
    }
}

pub(crate) async fn entry_path(app: &App, id: &str) -> String {
    java_repo::get_java_by_id(&app.db, id)
        .await
        .unwrap()
        .unwrap()
        .path
}

pub(crate) async fn linked(app: &App, profile: SystemJavaProfileName) -> Option<String> {
    java_repo::get_profile(&app.db, &profile.to_string())
        .await
        .unwrap()
        .unwrap()
        .java_id
}
