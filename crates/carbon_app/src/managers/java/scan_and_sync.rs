use super::{
    discovery::Discovery,
    java_checker::JavaChecker,
    major_defaults::{clear_major_defaults_for, get_major_defaults},
    resolve::{AutomaticPick, host_arch, javas_from_rows, pick_automatic, requirement_for},
};
use crate::domain::java::{JavaComponent, JavaComponentType, SystemJavaProfileName};
use carbon_repos::db_exec::Db;
use carbon_repos::repos::java as java_repo;
use std::collections::HashSet;
use std::path::PathBuf;
use strum::IntoEnumIterator;
use tracing::{info, trace, warn};

/// Collects the set of `Java.path` values that any `JavaProfile` currently links
/// to, used to decide whether a broken java may be deleted or must be kept and
/// marked invalid.
async fn java_paths_used_in_profiles(db: &Db) -> anyhow::Result<HashSet<String>> {
    let paths = java_repo::get_profile_linked_java_paths(db)
        .await?
        .into_iter()
        .map(|row| row.path)
        .collect();

    Ok(paths)
}

#[tracing::instrument(level = "trace", skip(db))]
async fn get_java_component_from_db(
    db: &Db,
    path: String,
) -> anyhow::Result<Option<java_repo::JavaRow>> {
    Ok(java_repo::get_java_by_path(db, &path).await?)
}

#[tracing::instrument(level = "trace", skip(db))]
pub async fn upsert_java_component_to_db(
    db: &Db,
    java_component: JavaComponent,
) -> anyhow::Result<String> {
    let already_existing_component =
        get_java_component_from_db(db, java_component.path.clone()).await?;

    let already_existing_component = already_existing_component.and_then(|row| {
        let id = row.id.clone();
        JavaComponent::try_from(row)
            .ok()
            .map(|component| (component, id))
    });

    if let Some((component, id)) = already_existing_component {
        if component == java_component {
            java_repo::set_java_validity(db, &id, true).await?;

            Ok(id)
        } else {
            // Same path, changed Java: update the existing row in place (for example a JDK
            // upgraded at a fixed install path). `path` is unique and the row id is preserved,
            // so this replaces the stored metadata, including a changed major version, instead
            // of failing the scan.
            let major = java_component.version.major as i32;
            let full_version = java_component.version.to_string();
            let arch = java_component.arch.to_string();
            let os = java_component.os.to_string();
            let vendor = java_component.vendor;
            java_repo::update_java_component(db, &id, major, &full_version, &arch, &os, &vendor)
                .await?;

            Ok(id)
        }
    } else {
        let row = java_repo::JavaRow {
            id: uuid::Uuid::new_v4().to_string(),
            path: java_component.path,
            major: java_component.version.major as i32,
            full_version: java_component.version.to_string(),
            r#type: java_component._type.to_string(),
            os: java_component.os.to_string(),
            arch: java_component.arch.to_string(),
            vendor: java_component.vendor,
            is_valid: true,
        };
        let id = row.id.clone();
        java_repo::insert_java(db, row).await?;

        Ok(id)
    }
}

/// Deletes the Java at `path`, and any per-major default that points at it.
#[tracing::instrument(level = "trace", skip(db))]
async fn delete_java_at(db: &Db, path: &str) -> anyhow::Result<()> {
    let Some(java) = java_repo::get_java_by_path(db, path).await? else {
        return Ok(());
    };

    java_repo::delete_java_by_path(db, path).await?;
    clear_major_defaults_for(db, &java.id).await?;

    Ok(())
}

#[tracing::instrument(level = "trace", skip(db))]
async fn update_java_component_in_db_to_invalid(db: &Db, path: String) -> anyhow::Result<()> {
    java_repo::set_java_validity_by_path(db, &path, false).await?;

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_local<T, G>(
    db: &Db,
    discovery: &T,
    java_checker: &G,
) -> anyhow::Result<()>
where
    T: Discovery,
    G: JavaChecker,
{
    let t_discover = std::time::Instant::now();
    let local_javas = discovery.find_java_paths().await;
    tracing::debug!(
        "[startup-timing] discovery.find_java_paths found {} candidate(s) in {:.2}s",
        local_javas.len(),
        t_discover.elapsed().as_secs_f64()
    );

    let used_java_paths = java_paths_used_in_profiles(db).await?;

    let mut inconclusive_paths = HashSet::new();
    for local_java in &local_javas {
        trace!("Analyzing local java: {:?}", local_java);
        let t_probe = std::time::Instant::now();

        let resolved_java_path = match dunce::canonicalize(local_java) {
            Ok(canonical_path) => canonical_path,
            Err(err) => {
                tracing::warn!("Error resolving canonical java path: {}", err);
                local_java.to_path_buf()
            }
        };

        // Verify whether the java is valid
        let java_bin_info = java_checker
            .get_bin_info(&resolved_java_path, JavaComponentType::Local)
            .await;
        tracing::debug!(
            "[startup-timing] probed java {} in {:.2}s",
            resolved_java_path.display(),
            t_probe.elapsed().as_secs_f64()
        );

        let db_entry =
            get_java_component_from_db(db, resolved_java_path.to_string_lossy().to_string())
                .await?;

        if let Some(db_entry) = &db_entry {
            if JavaComponentType::try_from(&*db_entry.r#type)? != JavaComponentType::Local {
                continue;
            }
        }

        let is_java_used_in_profile =
            used_java_paths.contains(&resolved_java_path.display().to_string());

        match (java_bin_info, db_entry) {
            // If it is valid, check whether it's in the DB
            (Ok(java_component), Some(_db_entry)) => {
                trace!("Java is valid: {:?}", java_component);
                upsert_java_component_to_db(db, java_component).await?;
            }
            (Ok(java_component), None) => {
                trace!("Java is valid: {:?}", java_component);
                upsert_java_component_to_db(db, java_component).await?;
            }
            (Err(err), _) if super::java_checker::validation_inconclusive(&err) => {
                inconclusive_paths.insert(resolved_java_path.display().to_string());
                warn!(
                    "Local Java validation inconclusive; preserving row and profile intent: {err:#}"
                );
            }
            // If it isn't valid, check whether it's in the DB
            (Err(err), db_entry) => {
                trace!("Java is invalid due to: {:?}", err);

                // If it is in the db, update it to invalid
                if db_entry.is_some() {
                    if is_java_used_in_profile {
                        update_java_component_in_db_to_invalid(
                            db,
                            resolved_java_path.display().to_string(),
                        )
                        .await?;
                    } else {
                        let path = resolved_java_path.display().to_string();
                        delete_java_at(db, &path).await?;
                    }
                }
            }
        }
    }

    // Cleanup unscanned local javas (if they are not default)
    let local_type = JavaComponentType::Local.to_string();
    let local_javas_from_db = java_repo::get_java_by_type(db, &local_type).await?;

    for local_java_from_db in local_javas_from_db {
        trace!(
            "Checking if java {} has been scanned",
            local_java_from_db.path
        );
        let has_been_scanned = inconclusive_paths.contains(&local_java_from_db.path)
            || local_javas
                .iter()
                .any(|local_java| local_java_from_db.path == local_java.display().to_string());

        if has_been_scanned {
            continue;
        }

        let is_used_in_profile = used_java_paths.contains(&local_java_from_db.path);

        if is_used_in_profile {
            update_java_component_in_db_to_invalid(db, local_java_from_db.path).await?;
        } else {
            let path = local_java_from_db.path;
            delete_java_at(db, &path).await?;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_custom<G>(db: &Db, java_checker: &G) -> anyhow::Result<()>
where
    G: JavaChecker,
{
    let custom_type = JavaComponentType::Custom.to_string();
    let custom_javas = java_repo::get_java_by_type(db, &custom_type).await?;

    for custom_java in custom_javas {
        let java_bin_info = java_checker
            .get_bin_info(
                &PathBuf::from(custom_java.path.clone()),
                JavaComponentType::Custom,
            )
            .await;

        match java_bin_info {
            Ok(component) => {
                upsert_java_component_to_db(db, component).await?;
            }
            Err(err) if super::java_checker::validation_inconclusive(&err) => {
                warn!(
                    "Custom Java validation inconclusive; preserving row and profile intent: {err:#}"
                );
            }
            Err(_) => {
                update_java_component_in_db_to_invalid(db, custom_java.path).await?;
            }
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_managed<T, G>(
    db: &Db,
    discovery: &T,
    java_checker: &G,
) -> anyhow::Result<()>
where
    T: Discovery,
    G: JavaChecker,
{
    let managed_type = JavaComponentType::Managed.to_string();
    let managed_javas = java_repo::get_java_by_type(db, &managed_type).await?;

    let used_java_paths = java_paths_used_in_profiles(db).await?;

    for managed_java in &managed_javas {
        let java_bin_info = java_checker
            .get_bin_info(
                &PathBuf::from(managed_java.path.clone()),
                JavaComponentType::Managed,
            )
            .await;

        let is_java_used_in_profile = used_java_paths.contains(&managed_java.path);

        info!(
            "java {} is used in profile: {}",
            managed_java.path, is_java_used_in_profile
        );

        match (java_bin_info, managed_java.is_valid) {
            (Ok(_java_component), true) => {}
            (Ok(java_component), false) => {
                upsert_java_component_to_db(db, java_component).await?;
            }
            (Err(err), _) if super::java_checker::validation_inconclusive(&err) => {
                warn!(
                    "Managed Java validation inconclusive; preserving row and profile intent: {err:#}"
                );
            }
            (Err(_), true) => {
                if is_java_used_in_profile {
                    update_java_component_in_db_to_invalid(db, managed_java.path.clone()).await?;
                } else {
                    let path = managed_java.path.clone();
                    delete_java_at(db, &path).await?;
                }
            }
            (Err(_), false) => {
                if !is_java_used_in_profile {
                    let path = managed_java.path.clone();
                    delete_java_at(db, &path).await?;
                }
            }
        }
    }

    let javas_on_disk = discovery.find_managed_java_paths().await;

    for java_path in javas_on_disk.iter().filter(|path| {
        !managed_javas
            .iter()
            .any(|java| java.path == path.to_string_lossy().to_string())
    }) {
        let java_bin_info = java_checker
            .get_bin_info(&java_path, JavaComponentType::Managed)
            .await;

        match java_bin_info {
            Ok(java_component) => {
                upsert_java_component_to_db(db, java_component).await?;
            }
            Err(err) if super::java_checker::validation_inconclusive(&err) => {
                warn!("New managed Java validation inconclusive: {err:#}")
            }
            Err(_) => {}
        }
    }

    Ok(())
}

/// Links each system profile to its automatic choice ([`pick_automatic`]),
/// with `flagged` holding the ids of Javas that have a known issue. A profile
/// whose choice is a download keeps its link, so a found Java stays linked
/// until a launch installs GDLauncher's own build.
#[tracing::instrument(level = "trace", skip_all)]
pub async fn sync_system_java_profiles(db: &Db, flagged: &HashSet<String>) -> anyhow::Result<()> {
    let javas = javas_from_rows(java_repo::get_all_java(db).await?);
    let defaults = get_major_defaults(db).await?;
    let profiles = java_repo::get_all_profiles(db).await?;
    let host_arch = host_arch();

    let mut links = Vec::new();
    for profile in SystemJavaProfileName::iter() {
        let name = profile.to_string();
        let linked = profiles
            .iter()
            .find(|row| row.name == name)
            .ok_or_else(|| anyhow::anyhow!("The Java {} settings are missing", profile.major()))?
            .java_id
            .as_deref();

        let pick = pick_automatic(
            &requirement_for(profile),
            &javas,
            &defaults,
            host_arch,
            flagged,
        );
        if let AutomaticPick::Java { id, reason } = pick
            && linked != Some(id.as_str())
        {
            trace!("Linking system java profile {name} to {id} ({reason:?})");
            links.push((name, id));
        }
    }

    if links.is_empty() {
        return Ok(());
    }

    // One writer dispatch and one transaction, so readers never see half the
    // profiles moved.
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        for (name, id) in links {
            java_repo::set_profile_java_conn(&tx, &name, Some(&id))?;
        }
        tx.commit()?;
        Ok(())
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use tracing::info;

    use crate::{
        domain::java::{
            JavaArch, JavaComponent, JavaComponentType, JavaOs, JavaVersion, SystemJavaProfileName,
        },
        managers::java::{
            JavaManager,
            discovery::MockDiscovery,
            java_checker::{MockJavaChecker, MockJavaCheckerInvalid},
            scan_and_sync::{
                scan_and_sync_custom, scan_and_sync_local, scan_and_sync_managed,
                sync_system_java_profiles, upsert_java_component_to_db,
            },
        },
        setup_managers_for_test,
    };
    use carbon_repos::repos::java as java_repo;
    use std::collections::HashSet;

    async fn all_javas(db: &carbon_repos::db_exec::Db) -> Vec<java_repo::JavaRow> {
        java_repo::get_all_java(db).await.unwrap()
    }

    #[tokio::test]
    async fn test_add_component_to_db() {
        let app = setup_managers_for_test().await;
        let db = &app.db;

        let java_path = "/usr/bin/java2".to_string();

        let java_component = JavaComponent {
            path: java_path.clone(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };
        let java_components = all_javas(db).await;
        assert_eq!(java_components.len(), 0);

        upsert_java_component_to_db(db, java_component.clone())
            .await
            .unwrap();

        let java_components = all_javas(db).await;
        assert_eq!(java_components.len(), 1);
        assert_eq!(java_components[0].path, "/usr/bin/java2");
        assert!(java_components[0].is_valid);

        java_repo::set_java_validity_by_path(db, &java_path, false)
            .await
            .unwrap();

        let java_components = all_javas(db).await;
        assert_eq!(java_components.len(), 1);
        assert!(!java_components[0].is_valid);

        upsert_java_component_to_db(db, java_component)
            .await
            .unwrap();

        let java_components = all_javas(db).await;
        assert_eq!(java_components.len(), 1);
        assert!(java_components[0].is_valid);

        let almost_equal_java_component = JavaComponent {
            path: java_path.clone(),
            version: JavaVersion::from_major(9), // different version
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };

        let result = upsert_java_component_to_db(db, almost_equal_java_component).await;

        // A changed major version at the same (unique) path updates the existing row in
        // place rather than failing, so an in-place JDK upgrade does not break the scan.
        assert!(result.is_ok());

        let java_components = all_javas(db).await;
        assert_eq!(java_components.len(), 1);
        assert_eq!(java_components[0].path, java_path);
        assert_eq!(java_components[0].major, 9);
        assert!(java_components[0].is_valid);
    }

    #[tokio::test]
    async fn test_scan_and_sync_local() {
        let app = setup_managers_for_test().await;
        let db = &app.db;

        let discovery = &MockDiscovery;
        let java_checker = &MockJavaChecker;
        // Insert one already existing path (/usr/bin/java) and one that should not exist anymore, hence removed (/usr/bin/java2)

        let component_to_remove = JavaComponent {
            path: "/java1".to_string(),
            version: JavaVersion::from_major(19),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };
        upsert_java_component_to_db(db, component_to_remove)
            .await
            .unwrap();

        let component_to_keep = JavaComponent {
            path: "/java4".to_string(),
            version: JavaVersion::from_major(19),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };

        upsert_java_component_to_db(db, component_to_keep)
            .await
            .unwrap();

        scan_and_sync_local(db, discovery, java_checker)
            .await
            .unwrap();

        let java_components = all_javas(db).await;

        println!("{:?}", java_components);

        assert_eq!(java_components.len(), 3);
    }

    #[tokio::test]
    /// This test is to make sure that if a java is invalid and not used in any profile, it will be removed
    /// If it's used in a profile, it will be set as invalid
    async fn test_scan_and_sync_local_broken_javas() {
        let app = setup_managers_for_test().await;
        let db = &app.db;
        let discovery = &MockDiscovery;
        let java_checker = &MockJavaCheckerInvalid;

        let component_to_add = JavaComponent {
            path: "/usr/bin/java".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };

        let component_to_add_still_used = JavaComponent {
            path: "/usr/bin/java1".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };

        upsert_java_component_to_db(db, component_to_add)
            .await
            .unwrap();
        let java_id = upsert_java_component_to_db(db, component_to_add_still_used)
            .await
            .unwrap();

        let profile_name = SystemJavaProfileName::Legacy.to_string();
        java_repo::set_profile_java(db, &profile_name, Some(&java_id))
            .await
            .unwrap();

        scan_and_sync_local(db, discovery, java_checker)
            .await
            .unwrap();

        let java_components = all_javas(db).await;

        assert_eq!(java_components.len(), 1);

        assert_eq!(java_components[0].path, "/usr/bin/java1");
        assert!(!java_components[0].is_valid);
    }
    #[tokio::test]
    async fn test_scan_and_sync_managed_broken_javas() {
        let app = setup_managers_for_test().await;
        let db = &app.db;
        let java_checker = &MockJavaCheckerInvalid;
        let discovery = &MockDiscovery;

        let component_to_add = JavaComponent {
            path: "/my/managed/path".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Managed,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };
        let component_to_add_still_used = JavaComponent {
            path: "/my/managed/path1".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Managed,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };

        upsert_java_component_to_db(db, component_to_add)
            .await
            .unwrap();
        let java_id = upsert_java_component_to_db(db, component_to_add_still_used)
            .await
            .unwrap();

        let profile_name = SystemJavaProfileName::Legacy.to_string();
        java_repo::set_profile_java(db, &profile_name, Some(&java_id))
            .await
            .unwrap();

        scan_and_sync_managed(db, discovery, java_checker)
            .await
            .unwrap();

        let java_components = all_javas(db).await;

        assert_eq!(java_components.len(), 1);

        assert_eq!(java_components[0].path, "/my/managed/path1");
        assert!(!java_components[0].is_valid);
    }
    #[tokio::test]
    async fn test_scan_and_sync_custom_broken_javas() {
        let app = setup_managers_for_test().await;
        let db = &app.db;
        let java_checker = &MockJavaCheckerInvalid;

        let component_to_add = JavaComponent {
            path: "/my/custom/path".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Custom,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };
        let component_to_add_still_used = JavaComponent {
            path: "/my/custom/path1".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Custom,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };

        upsert_java_component_to_db(db, component_to_add)
            .await
            .unwrap();
        let java_id = upsert_java_component_to_db(db, component_to_add_still_used)
            .await
            .unwrap();

        let profile_name = SystemJavaProfileName::Legacy.to_string();
        java_repo::set_profile_java(db, &profile_name, Some(&java_id))
            .await
            .unwrap();

        scan_and_sync_custom(db, java_checker).await.unwrap();

        let java_components = all_javas(db).await;

        assert_eq!(java_components.len(), 2);

        for java_component in java_components {
            assert!(!java_component.is_valid);
        }
    }

    #[tokio::test]
    async fn deleting_a_found_java_clears_the_default_pointing_at_it() {
        use crate::managers::java::major_defaults::{get_major_defaults, set_major_default};
        use std::collections::HashMap;

        let app = setup_managers_for_test().await;
        let db = &app.db;
        let found = |path: &str, major| JavaComponent {
            path: path.to_string(),
            version: JavaVersion::from_major(major),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_64,
            os: JavaOs::Linux,
            vendor: "Eclipse Adoptium".to_string(),
        };
        // One the scan finds broken, one it no longer finds at all.
        let broken = upsert_java_component_to_db(db, found("/java1", 17))
            .await
            .unwrap();
        let vanished = upsert_java_component_to_db(db, found("/gone/java", 21))
            .await
            .unwrap();
        set_major_default(db, 17, Some(broken)).await.unwrap();
        set_major_default(db, 21, Some(vanished)).await.unwrap();
        set_major_default(db, 8, Some("elsewhere".into()))
            .await
            .unwrap();

        scan_and_sync_local(db, &MockDiscovery, &MockJavaCheckerInvalid)
            .await
            .unwrap();

        assert!(all_javas(db).await.is_empty());
        assert_eq!(
            get_major_defaults(db).await.unwrap(),
            HashMap::from([(8, "elsewhere".to_string())])
        );
    }

    #[tokio::test]
    async fn test_scan_and_sync_managed_on_disk_but_not_on_database() {
        let app = setup_managers_for_test().await;
        let db = &app.db;
        let discovery = &MockDiscovery;
        let java_checker = &MockJavaChecker;

        scan_and_sync_managed(db, discovery, java_checker)
            .await
            .unwrap();

        let java_components = all_javas(db).await;

        assert_eq!(java_components.len(), 3);
        for java_component in java_components {
            assert!(java_component.is_valid);
        }
    }

    #[tokio::test]
    async fn test_sync_system_java_profiles_with_profiles() {
        let app = setup_managers_for_test().await;
        let db = &app.db;

        JavaManager::ensure_profiles_in_db(db).await.unwrap();

        // manually set one of the profiles to non-system to make sure it gets updated to system
        let legacy_name = SystemJavaProfileName::Legacy.to_string();
        java_repo::upsert_profile(db, &legacy_name, false)
            .await
            .unwrap();

        let seed_javas = [
            seed_row("my_path1", 8, "1.8.0_282", "managed", true),
            seed_row("my_path2", 17, "17.0.1", "managed", true),
            seed_row("my_path3", 14, "14.0.1", "managed", false),
            seed_row("my_path4", 21, "21.0.5", "local", true),
        ];
        for java in seed_javas {
            java_repo::insert_java(db, java).await.unwrap();
        }

        JavaManager::ensure_profiles_in_db(db).await.unwrap();
        sync_system_java_profiles(db, &HashSet::new())
            .await
            .unwrap();

        let all_profiles = java_repo::get_all_profiles(db).await.unwrap();
        assert!(all_profiles.iter().all(|profile| profile.is_system_profile));

        // A found Java is never linked automatically.
        let delta_name = SystemJavaProfileName::Delta.to_string();
        let delta_profile = java_repo::get_profile(db, &delta_name)
            .await
            .unwrap()
            .unwrap();
        assert!(delta_profile.java_id.is_none());

        // Expect 8 and 17 to be there, but not 14 since it's invalid and 16 because not provided
        let legacy_name = SystemJavaProfileName::Legacy.to_string();
        let legacy_profile = java_repo::get_profile(db, &legacy_name)
            .await
            .unwrap()
            .unwrap();

        info!("{:?}", legacy_profile);

        assert!(legacy_profile.java_id.is_some());

        let alpha_name = SystemJavaProfileName::Alpha.to_string();
        let alpha_profile = java_repo::get_profile(db, &alpha_name)
            .await
            .unwrap()
            .unwrap();

        assert!(alpha_profile.java_id.is_none());

        let beta_name = SystemJavaProfileName::Beta.to_string();
        let beta_profile = java_repo::get_profile(db, &beta_name)
            .await
            .unwrap()
            .unwrap();

        assert!(beta_profile.java_id.is_some());

        let gamma_name = SystemJavaProfileName::Gamma.to_string();
        let gamma_profile = java_repo::get_profile(db, &gamma_name)
            .await
            .unwrap()
            .unwrap();

        assert!(gamma_profile.java_id.is_some());

        let minecraft_exe_name = SystemJavaProfileName::MinecraftJavaExe.to_string();
        let minecraft_exe_profile = java_repo::get_profile(db, &minecraft_exe_name)
            .await
            .unwrap()
            .unwrap();

        assert!(minecraft_exe_profile.java_id.is_none());
    }

    fn seed_row(
        path: &str,
        major: i32,
        version: &str,
        ty: &str,
        valid: bool,
    ) -> java_repo::JavaRow {
        java_repo::JavaRow {
            id: path.to_string(),
            path: path.to_string(),
            major,
            full_version: version.to_string(),
            r#type: ty.to_string(),
            os: "linux".to_string(),
            arch: crate::managers::java::resolve::host_arch().to_string(),
            vendor: "Azul Systems, Inc.".to_string(),
            is_valid: valid,
        }
    }

    #[tokio::test]
    async fn sync_links_defaults_then_managed_and_keeps_found_links_otherwise() {
        let app = setup_managers_for_test().await;
        let db = &app.db;
        for java in [
            seed_row("l17", 17, "17.0.12", "local", true),
            seed_row("l21", 21, "21.0.5", "local", true),
            seed_row("m21", 21, "21.0.4", "managed", true),
            seed_row("c8", 8, "1.8.0_402", "custom", true),
            seed_row("m8", 8, "1.8.0_492", "managed", true),
            seed_row("m8old", 8, "1.8.0_302", "managed", true),
        ] {
            java_repo::insert_java(db, java).await.unwrap();
        }
        let link = |profile: SystemJavaProfileName, id: &'static str| {
            let db = db.clone();
            async move {
                java_repo::set_profile_java(&db, &profile.to_string(), Some(id))
                    .await
                    .unwrap();
            }
        };
        link(SystemJavaProfileName::Gamma, "l17").await;
        link(SystemJavaProfileName::Delta, "l21").await;
        crate::managers::java::major_defaults::set_major_default(db, 8, Some("c8".into()))
            .await
            .unwrap();

        sync_system_java_profiles(db, &HashSet::new())
            .await
            .unwrap();

        let linked = |profile: SystemJavaProfileName| {
            let db = db.clone();
            async move {
                java_repo::get_profile(&db, &profile.to_string())
                    .await
                    .unwrap()
                    .unwrap()
                    .java_id
            }
        };
        // No managed 17: the found one stays until a launch installs one.
        assert_eq!(
            linked(SystemJavaProfileName::Gamma).await.as_deref(),
            Some("l17")
        );
        assert_eq!(linked(SystemJavaProfileName::Beta).await, None);
        // GDLauncher's own build replaces a found one.
        assert_eq!(
            linked(SystemJavaProfileName::Delta).await.as_deref(),
            Some("m21")
        );
        // The default beats managed builds, and Forge 1.16.5 keeps its
        // below-312 build.
        assert_eq!(
            linked(SystemJavaProfileName::Legacy).await.as_deref(),
            Some("c8")
        );
        assert_eq!(
            linked(SystemJavaProfileName::LegacyFixed1).await.as_deref(),
            Some("m8old")
        );
    }

    #[tokio::test]
    async fn test_upsert_java_component_to_db_different_java_configuration() {
        let app = setup_managers_for_test().await;
        let db = &app.db;

        let discovery = &MockDiscovery;
        let java_checker = &MockJavaChecker;

        let old_component = JavaComponent {
            path: "/java1".to_string(),
            version: JavaVersion::from_major(19),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };

        upsert_java_component_to_db(db, old_component)
            .await
            .unwrap();

        let new_component = JavaComponent {
            path: "/java1".to_string(),
            version: JavaVersion::from_major(19),
            _type: JavaComponentType::Local,
            arch: JavaArch::Arm64,
            os: JavaOs::Windows,
            vendor: "Azul Systems, Inc. New".to_string(),
        };

        scan_and_sync_local(db, discovery, java_checker)
            .await
            .unwrap();

        upsert_java_component_to_db(db, new_component.clone())
            .await
            .unwrap();

        let java_components = all_javas(db).await;

        assert_eq!(java_components.len(), 3);

        let java_component = JavaComponent::try_from(java_components[0].clone()).unwrap();

        assert_eq!(java_component, new_component);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::java::{JavaArch, JavaVersion};

    struct OutcomeChecker(&'static str);
    #[async_trait::async_trait]
    impl JavaChecker for OutcomeChecker {
        async fn get_bin_info(
            &self,
            _: &std::path::Path,
            _: JavaComponentType,
        ) -> anyhow::Result<JavaComponent> {
            match self.0 {
                "unknown" => Err(crate::managers::runtime_lifecycle::ChildSettlementUnknown.into()),
                "bookkeeping" => {
                    Err(crate::managers::runtime_lifecycle::tests_support::bookkeeping_error())
                }
                "cancel" => Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "controlled confirmed cancellation",
                )
                .into()),
                "freeze" => {
                    let lifecycle = std::sync::Arc::new(
                        crate::managers::runtime_lifecycle::RuntimeLifecycle::default(),
                    );
                    lifecycle.reconciliation_finished(Ok(()));
                    lifecycle.begin_shutdown(
                        "fixture",
                        crate::managers::runtime_lifecycle::ShutdownPurpose::Quit,
                    )?;
                    let owner = super::super::java_checker::admit_probe(&lifecycle)?;
                    owner.finish();
                    unreachable!()
                }
                _ => anyhow::bail!("ordinary invalid Java"),
            }
        }
    }
    struct SelectedDiscovery(Vec<std::path::PathBuf>);
    #[async_trait::async_trait]
    impl Discovery for SelectedDiscovery {
        fn new(_: carbon_rt_path::RuntimePath) -> Self {
            Self(vec![])
        }
        async fn find_java_paths(&self) -> Vec<std::path::PathBuf> {
            self.0.clone()
        }
        async fn find_managed_java_paths(&self) -> Vec<std::path::PathBuf> {
            vec![]
        }
    }
    async fn uncertain_scan(kind: JavaComponentType) {
        for mode in ["unknown", "bookkeeping", "cancel", "freeze"] {
            let (root, db) = fixture().await;
            let path = root.path().join("java");
            std::fs::write(&path, b"existing installed bytes").unwrap();
            // Scans store and look up canonical paths. A temp dir often isn't
            // one (8.3 names on Windows, /var -> /private/var on macOS).
            let path = dunce::canonicalize(&path).unwrap();
            let component = JavaComponent {
                path: path.display().to_string(),
                version: JavaVersion::from_major(17),
                _type: kind.clone(),
                arch: JavaArch::X86_64,
                os: JavaOs::Linux,
                vendor: "preserved".into(),
            };
            let id = upsert_java_component_to_db(&db, component).await.unwrap();
            java_repo::insert_profile(&db, "intent", &id).await.unwrap();
            let discovery = SelectedDiscovery(vec![path.clone()]);
            match kind {
                JavaComponentType::Local => {
                    scan_and_sync_local(&db, &discovery, &OutcomeChecker(mode))
                        .await
                        .unwrap()
                }
                JavaComponentType::Custom => scan_and_sync_custom(&db, &OutcomeChecker(mode))
                    .await
                    .unwrap(),
                JavaComponentType::Managed => {
                    scan_and_sync_managed(&db, &discovery, &OutcomeChecker(mode))
                        .await
                        .unwrap()
                }
            }
            let row = java_repo::get_java_by_id(&db, &id)
                .await
                .unwrap()
                .expect("inconclusive scan must retain row identity");
            assert!(row.is_valid, "{kind:?} {mode} is not proof of invalidity");
            assert_eq!(row.vendor, "preserved");
            assert_eq!(
                java_repo::get_profile(&db, "intent")
                    .await
                    .unwrap()
                    .unwrap()
                    .java_id,
                Some(id.clone())
            );
            match kind {
                JavaComponentType::Local => {
                    scan_and_sync_local(&db, &discovery, &OutcomeChecker("invalid"))
                        .await
                        .unwrap()
                }
                JavaComponentType::Custom => scan_and_sync_custom(&db, &OutcomeChecker("invalid"))
                    .await
                    .unwrap(),
                JavaComponentType::Managed => {
                    scan_and_sync_managed(&db, &discovery, &OutcomeChecker("invalid"))
                        .await
                        .unwrap()
                }
            }
            assert!(
                !java_repo::get_java_by_id(&db, &id)
                    .await
                    .unwrap()
                    .unwrap()
                    .is_valid,
                "ordinary invalid still invalidates referenced row"
            );
        }
    }
    #[tokio::test]
    async fn local_scan_preserves_inconclusive_and_invalidates_proven_failure() {
        uncertain_scan(JavaComponentType::Local).await;
    }
    #[tokio::test]
    async fn custom_scan_preserves_inconclusive_and_invalidates_proven_failure() {
        uncertain_scan(JavaComponentType::Custom).await;
    }
    #[tokio::test]
    async fn managed_scan_preserves_inconclusive_and_invalidates_proven_failure() {
        uncertain_scan(JavaComponentType::Managed).await;
    }

    use crate::domain::java::JavaOs;
    struct Checker {
        major: u8,
        fail: bool,
    }
    #[async_trait::async_trait]
    impl JavaChecker for Checker {
        async fn get_bin_info(
            &self,
            path: &std::path::Path,
            kind: JavaComponentType,
        ) -> anyhow::Result<JavaComponent> {
            if self.fail {
                anyhow::bail!("controlled missing binary");
            }
            Ok(JavaComponent {
                path: path.display().to_string(),
                version: JavaVersion::from_major(self.major as u16),
                _type: kind,
                arch: JavaArch::X86_64,
                os: JavaOs::Linux,
                vendor: "refreshed".into(),
            })
        }
    }
    async fn fixture() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("java.db");
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        carbon_repos::get_migrations()
            .0
            .to_latest(&mut conn)
            .unwrap();
        drop(conn);
        let db = Db::open(&path, 2, true).unwrap();
        (dir, db)
    }
    #[tokio::test]
    async fn custom_java_refresh_preserves_identity_and_custom_link() {
        let (_root, db) = fixture().await;
        let component = JavaComponent {
            path: "fixture-java".into(),
            version: JavaVersion::from_major(17),
            _type: JavaComponentType::Custom,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "old".into(),
        };
        let id = upsert_java_component_to_db(&db, component).await.unwrap();
        java_repo::insert_profile(&db, "custom-user", &id)
            .await
            .unwrap();
        java_repo::set_java_validity(&db, &id, false).await.unwrap();
        scan_and_sync_custom(
            &db,
            &Checker {
                major: 21,
                fail: false,
            },
        )
        .await
        .unwrap();
        let row = java_repo::get_java_by_path(&db, "fixture-java")
            .await
            .unwrap()
            .unwrap();
        assert!(row.is_valid);
        assert_eq!(row.id, id);
        assert_eq!(row.major, 21);
        assert_eq!(row.arch, JavaArch::X86_64.to_string());
        assert_eq!(row.vendor, "refreshed");
        assert_eq!(row.r#type, JavaComponentType::Custom.to_string());
        assert_eq!(
            java_repo::get_profile(&db, "custom-user")
                .await
                .unwrap()
                .unwrap()
                .java_id,
            Some(id.clone())
        );
        scan_and_sync_custom(
            &db,
            &Checker {
                major: 21,
                fail: false,
            },
        )
        .await
        .unwrap();
        scan_and_sync_custom(
            &db,
            &Checker {
                major: 21,
                fail: true,
            },
        )
        .await
        .unwrap();
        assert!(
            !java_repo::get_java_by_path(&db, "fixture-java")
                .await
                .unwrap()
                .unwrap()
                .is_valid
        );
        assert_eq!(
            java_repo::get_profile(&db, "custom-user")
                .await
                .unwrap()
                .unwrap()
                .java_id,
            Some(id)
        );
    }
    /// A found Java that failed one check (a drive not mounted yet at
    /// startup) keeps its system profile link, as in 2.0.40, even with another
    /// valid compatible found Java: Play checks it again.
    #[tokio::test]
    async fn java_that_failed_one_check_keeps_its_system_profile_link() {
        let (_root, db) = fixture().await;
        for profile in SystemJavaProfileName::iter() {
            java_repo::upsert_profile(&db, &profile.to_string(), true)
                .await
                .unwrap();
        }
        let java = |path: &str| JavaComponent {
            path: path.into(),
            version: JavaVersion::from_major(21),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_64,
            os: JavaOs::Linux,
            vendor: "fixture".into(),
        };
        upsert_java_component_to_db(&db, java("valid-alternative"))
            .await
            .unwrap();
        let failed_once = upsert_java_component_to_db(&db, java("failed-once"))
            .await
            .unwrap();
        java_repo::set_java_validity(&db, &failed_once, false)
            .await
            .unwrap();
        let profile = SystemJavaProfileName::iter()
            .find(|p| p.is_java_version_compatible(&JavaVersion::from_major(21)))
            .unwrap();
        java_repo::set_profile_java(&db, &profile.to_string(), Some(&failed_once))
            .await
            .unwrap();

        sync_system_java_profiles(&db, &HashSet::new())
            .await
            .unwrap();

        assert_eq!(
            java_repo::get_profile(&db, &profile.to_string())
                .await
                .unwrap()
                .unwrap()
                .java_id,
            Some(failed_once)
        );
    }
    struct EmptyDiscovery;
    #[async_trait::async_trait]
    impl Discovery for EmptyDiscovery {
        fn new(_: carbon_rt_path::RuntimePath) -> Self {
            Self
        }
        async fn find_java_paths(&self) -> Vec<std::path::PathBuf> {
            vec![]
        }
        async fn find_managed_java_paths(&self) -> Vec<std::path::PathBuf> {
            vec![]
        }
    }
    #[tokio::test]
    async fn a_scan_that_links_nothing_and_a_sync_both_keep_a_custom_link() {
        let (_root, db) = fixture().await;
        for profile in SystemJavaProfileName::iter() {
            java_repo::upsert_profile(&db, &profile.to_string(), true)
                .await
                .unwrap();
        }
        let component = JavaComponent {
            path: "custom-fixture".into(),
            version: JavaVersion::from_major(17),
            _type: JavaComponentType::Custom,
            arch: JavaArch::X86_64,
            os: JavaOs::Linux,
            vendor: "fixture".into(),
        };
        let id = upsert_java_component_to_db(&db, component).await.unwrap();
        let incompatible = SystemJavaProfileName::iter()
            .find(|p| !p.is_java_version_compatible(&JavaVersion::from_major(21)))
            .unwrap();
        java_repo::set_profile_java(&db, &incompatible.to_string(), Some(&id))
            .await
            .unwrap();
        super::super::JavaManager::scan_and_sync(
            &db,
            &EmptyDiscovery,
            &Checker {
                major: 21,
                fail: false,
            },
            false,
        )
        .await
        .unwrap();
        assert_eq!(
            java_repo::get_profile(&db, &incompatible.to_string())
                .await
                .unwrap()
                .unwrap()
                .java_id,
            Some(id.clone())
        );
        let compatible = SystemJavaProfileName::iter()
            .find(|p| p.is_java_version_compatible(&JavaVersion::from_major(21)))
            .unwrap();
        java_repo::set_profile_java(&db, &compatible.to_string(), Some(&id))
            .await
            .unwrap();
        sync_system_java_profiles(&db, &HashSet::new())
            .await
            .unwrap();
        assert_eq!(
            java_repo::get_profile(&db, &compatible.to_string())
                .await
                .unwrap()
                .unwrap()
                .java_id,
            Some(id)
        );
    }
}
