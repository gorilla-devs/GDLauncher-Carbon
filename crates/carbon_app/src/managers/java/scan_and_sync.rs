use super::{discovery::Discovery, java_checker::JavaChecker};
use crate::domain::java::{
    JavaArch, JavaComponent, JavaComponentType, JavaVersion, SystemJavaProfileName,
};
use carbon_repos::{DbPool, models, queries};
use std::path::PathBuf;
use strum::IntoEnumIterator;
use tracing::{info, trace, warn};

#[tracing::instrument(level = "trace", skip(db_pool))]
async fn get_java_component_from_db(
    db_pool: &DbPool,
    path: String,
) -> anyhow::Result<Option<models::Java>> {
    let pool = db_pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let java = queries::java::FindJavaByPath::fetch_optional(&conn, &path)?;
        Ok(java)
    })
    .await?
}

#[tracing::instrument(level = "trace", skip(db_pool))]
pub async fn upsert_java_component_to_db(
    db_pool: &DbPool,
    java_component: JavaComponent,
) -> anyhow::Result<String> {
    let already_existing_component =
        get_java_component_from_db(db_pool, java_component.path.clone()).await?;

    let already_existing_component = already_existing_component
        .map(|data| {
            (
                JavaComponent::try_from(data.clone()),
                data.is_valid,
                data.id,
            )
        })
        .and_then(|res| {
            let resp = res.0.ok();

            match resp {
                Some(val) => Some((val, res.1, res.2)),
                None => None,
            }
        });

    if let Some((component, _is_valid, id)) = already_existing_component {
        if component == java_component {
            let pool = db_pool.clone();
            let id_clone = id.clone();
            tokio::task::spawn_blocking(move || {
                let conn = pool.get()?;
                queries::java::UpdateJavaValid::execute(&conn, &id_clone, true)?;
                Ok::<_, anyhow::Error>(())
            })
            .await??;

            return Ok(id);
        } else if component.version.major == java_component.version.major {
            let pool = db_pool.clone();
            let id_clone = id.clone();
            tokio::task::spawn_blocking(move || {
                let conn = pool.get()?;
                conn.execute(
                    "UPDATE Java SET fullVersion = ?2, arch = ?3, os = ?4, vendor = ?5, isValid = 1 WHERE id = ?1",
                    rusqlite::params![
                        &id_clone,
                        &java_component.version.to_string(),
                        &java_component.arch.to_string(),
                        &java_component.os.to_string(),
                        &java_component.vendor
                    ],
                )?;
                Ok::<_, anyhow::Error>(())
            })
            .await??;

            return Ok(id);
        } else {
            anyhow::bail!(
                "Java component with same path but different major version already exists"
            );
        }
    } else {
        let pool = db_pool.clone();
        let new_id = uuid::Uuid::new_v4().to_string();
        let id_clone = new_id.clone();

        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::java::CreateJava::execute(
                &conn,
                &id_clone,
                &java_component.path,
                java_component.version.major as i32,
                &java_component.version.to_string(),
                &java_component._type.to_string(),
                &java_component.os.to_string(),
                &java_component.arch.to_string(),
                &java_component.vendor,
                true,
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(new_id)
    }
}

#[tracing::instrument(level = "trace", skip(db_pool))]
async fn update_java_component_in_db_to_invalid(
    db_pool: &DbPool,
    path: String,
) -> anyhow::Result<()> {
    let pool = db_pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute(
            "UPDATE Java SET isValid = 0 WHERE path = ?1",
            rusqlite::params![&path],
        )?;
        Ok::<_, anyhow::Error>(())
    })
    .await??;

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_local<T, G>(
    db_pool: &DbPool,
    discovery: &T,
    java_checker: &G,
) -> anyhow::Result<()>
where
    T: Discovery,
    G: JavaChecker,
{
    let local_javas = discovery.find_java_paths().await;

    // Get java profiles with their associated java paths using typed JOIN result
    let pool = db_pool.clone();
    let java_profiles_with_paths: Vec<models::JavaProfileWithPath> =
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let results = queries::java::ListJavaProfilesWithJavaPath::fetch_all(&conn)?;
            Ok::<_, anyhow::Error>(results)
        })
        .await??;

    for local_java in &local_javas {
        trace!("Analyzing local java: {:?}", local_java);

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

        let db_entry =
            get_java_component_from_db(db_pool, resolved_java_path.to_string_lossy().to_string())
                .await?;

        if let Some(db_entry) = &db_entry {
            if JavaComponentType::try_from(&*db_entry.java_type)? != JavaComponentType::Local {
                continue;
            }
        }

        let is_java_used_in_profile = java_profiles_with_paths.iter().any(|profile| {
            profile
                .java_path
                .as_ref()
                .map(|p| p == &resolved_java_path.display().to_string())
                .unwrap_or(false)
        });

        match (java_bin_info, db_entry) {
            // If it is valid, check whether it's in the DB
            (Ok(java_component), Some(_db_entry)) => {
                trace!("Java is valid: {:?}", java_component);
                upsert_java_component_to_db(db_pool, java_component).await?;
            }
            (Ok(java_component), None) => {
                trace!("Java is valid: {:?}", java_component);
                upsert_java_component_to_db(db_pool, java_component).await?;
            }
            // If it isn't valid, check whether it's in the DB
            (Err(err), db_entry) => {
                trace!("Java is invalid due to: {:?}", err);

                // If it is in the db, update it to invalid
                if db_entry.is_some() {
                    if is_java_used_in_profile {
                        update_java_component_in_db_to_invalid(
                            db_pool,
                            resolved_java_path.display().to_string(),
                        )
                        .await?;
                    } else {
                        let pool = db_pool.clone();
                        let path = resolved_java_path.display().to_string();
                        tokio::task::spawn_blocking(move || {
                            let conn = pool.get()?;
                            queries::java::DeleteJavaByPath::execute(&conn, &path)?;
                            Ok::<_, anyhow::Error>(())
                        })
                        .await??;
                    }
                }
            }
        }
    }

    // Cleanup unscanned local javas (if they are not default)
    let pool = db_pool.clone();
    let local_javas_from_db: Vec<models::Java> = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM Java WHERE type = ?1")?;
        let javas = stmt
            .query_map(
                rusqlite::params![JavaComponentType::Local.to_string()],
                |row| models::Java::from_row(row),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, anyhow::Error>(javas)
    })
    .await??;

    for local_java_from_db in local_javas_from_db {
        trace!(
            "Checking if java {} has been scanned",
            local_java_from_db.path
        );
        let has_been_scanned = local_javas
            .iter()
            .any(|local_java| local_java_from_db.path == local_java.display().to_string());

        if has_been_scanned {
            continue;
        }

        let is_used_in_profile = java_profiles_with_paths.iter().any(|profile| {
            profile
                .java_path
                .as_ref()
                .map(|p| p == &local_java_from_db.path)
                .unwrap_or(false)
        });

        if is_used_in_profile {
            update_java_component_in_db_to_invalid(db_pool, local_java_from_db.path).await?;
        } else {
            let pool = db_pool.clone();
            let path = local_java_from_db.path;
            tokio::task::spawn_blocking(move || {
                let conn = pool.get()?;
                queries::java::DeleteJavaByPath::execute(&conn, &path)?;
                Ok::<_, anyhow::Error>(())
            })
            .await??;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_custom<G>(db_pool: &DbPool, java_checker: &G) -> anyhow::Result<()>
where
    G: JavaChecker,
{
    let pool = db_pool.clone();
    let custom_javas: Vec<models::Java> = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM Java WHERE type = ?1")?;
        let javas = stmt
            .query_map(
                rusqlite::params![JavaComponentType::Custom.to_string()],
                |row| models::Java::from_row(row),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, anyhow::Error>(javas)
    })
    .await??;

    for custom_java in custom_javas {
        let java_bin_info = java_checker
            .get_bin_info(
                &PathBuf::from(custom_java.path.clone()),
                JavaComponentType::Custom,
            )
            .await;

        if java_bin_info.is_err() {
            update_java_component_in_db_to_invalid(db_pool, custom_java.path).await?;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_managed<T, G>(
    db_pool: &DbPool,
    discovery: &T,
    java_checker: &G,
) -> anyhow::Result<()>
where
    T: Discovery,
    G: JavaChecker,
{
    let pool = db_pool.clone();
    let managed_javas: Vec<models::Java> = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM Java WHERE type = ?1")?;
        let javas = stmt
            .query_map(
                rusqlite::params![JavaComponentType::Managed.to_string()],
                |row| models::Java::from_row(row),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, anyhow::Error>(javas)
    })
    .await??;

    // Get java profiles with their associated java paths using typed JOIN result
    let pool = db_pool.clone();
    let java_profiles_with_paths: Vec<models::JavaProfileWithPath> =
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let results = queries::java::ListJavaProfilesWithJavaPath::fetch_all(&conn)?;
            Ok::<_, anyhow::Error>(results)
        })
        .await??;

    for managed_java in &managed_javas {
        let java_bin_info = java_checker
            .get_bin_info(
                &PathBuf::from(managed_java.path.clone()),
                JavaComponentType::Managed,
            )
            .await;

        let is_java_used_in_profile = java_profiles_with_paths.iter().any(|profile| {
            profile
                .java_path
                .as_ref()
                .map(|p| p == &managed_java.path)
                .unwrap_or(false)
        });

        info!(
            "java {} is used in profile: {}",
            managed_java.path, is_java_used_in_profile
        );

        match (java_bin_info, managed_java.is_valid) {
            (Ok(_java_component), true) => {}
            (Ok(java_component), false) => {
                upsert_java_component_to_db(db_pool, java_component).await?;
            }
            (Err(_), true) => {
                if is_java_used_in_profile {
                    update_java_component_in_db_to_invalid(db_pool, managed_java.path.clone())
                        .await?;
                } else {
                    let pool = db_pool.clone();
                    let path = managed_java.path.clone();
                    tokio::task::spawn_blocking(move || {
                        let conn = pool.get()?;
                        queries::java::DeleteJavaByPath::execute(&conn, &path)?;
                        Ok::<_, anyhow::Error>(())
                    })
                    .await??;
                }
            }
            (Err(_), false) => {
                if !is_java_used_in_profile {
                    let pool = db_pool.clone();
                    let path = managed_java.path.clone();
                    tokio::task::spawn_blocking(move || {
                        let conn = pool.get()?;
                        queries::java::DeleteJavaByPath::execute(&conn, &path)?;
                        Ok::<_, anyhow::Error>(())
                    })
                    .await??;
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

        if let Ok(java_component) = java_bin_info {
            upsert_java_component_to_db(db_pool, java_component).await?;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn sync_system_java_profiles(db_pool: &DbPool) -> anyhow::Result<()> {
    let pool = db_pool.clone();
    let all_javas: Vec<models::Java> = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let javas = queries::java::ListJavas::fetch_all(&conn)?;
        Ok::<_, anyhow::Error>(javas)
    })
    .await??;

    let is32bit = std::env::consts::ARCH == "x86" || std::env::consts::ARCH == "arm";

    for profile in SystemJavaProfileName::iter() {
        trace!("Syncing system java profile: {}", profile.to_string());

        let pool = db_pool.clone();
        let profile_name = profile.to_string();
        let java_in_profile: Option<String> = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let profile = queries::java::FindJavaProfileByName::fetch_one(&conn, &profile_name)
                .map_err(|_| {
                    anyhow::anyhow!("Java system profile {} not found in DB", profile_name)
                })?;
            Ok::<_, anyhow::Error>(profile.java_id)
        })
        .await??;

        if java_in_profile.is_some() {
            trace!(
                "Java system profile {} already has a java",
                profile.to_string()
            );
            continue;
        }

        // Scan for a compatible java
        for java in all_javas.iter() {
            trace!("Checking java {}", java.path);
            if !java.is_valid {
                warn!("Java {} is invalid, skipping", java.path);
                continue;
            }

            let java_version = JavaVersion::try_from(java.full_version.as_str())?;
            let java_arch = JavaArch::try_from(java.arch.as_str())?;

            let is_arch_allowed = match java_arch {
                JavaArch::X86_32 | JavaArch::Arm32 => is32bit,
                _ => true,
            };

            if profile.is_java_version_compatible(&java_version) && is_arch_allowed {
                trace!(
                    "Java {} is compatible with profile {}",
                    java.path,
                    profile.to_string()
                );

                let pool = db_pool.clone();
                let profile_name = profile.to_string();
                let java_id = java.id.clone();
                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    queries::java::UpdateJavaProfileJavaId::execute(
                        &conn,
                        &profile_name,
                        Some(&java_id),
                    )?;
                    Ok::<_, anyhow::Error>(())
                })
                .await??;

                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
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

    // Helper function to get java count
    async fn get_java_count(pool: &DbPool) -> i32 {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::CountJavas::fetch_scalar(&conn).unwrap()
        })
        .await
        .unwrap()
    }

    // Helper function to get all javas
    async fn get_all_javas(pool: &DbPool) -> Vec<models::Java> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::ListJavas::fetch_all(&conn).unwrap()
        })
        .await
        .unwrap()
    }

    // Helper to update java profile
    async fn update_java_profile_java_id(pool: &DbPool, profile_name: &str, java_id: Option<&str>) {
        let pool = pool.clone();
        let profile_name = profile_name.to_string();
        let java_id = java_id.map(|s| s.to_string());
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::UpdateJavaProfileJavaId::execute(
                &conn,
                &profile_name,
                java_id.as_deref(),
            )
            .unwrap();
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_add_component_to_db() {
        let app = setup_managers_for_test().await;
        let db_pool = &app.db_pool;

        let java_path = "/usr/bin/java2".to_string();

        let java_component = JavaComponent {
            path: java_path.clone(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };
        let java_components = get_all_javas(db_pool).await;
        assert_eq!(java_components.len(), 0);

        upsert_java_component_to_db(db_pool, java_component.clone())
            .await
            .unwrap();

        let java_components = get_all_javas(db_pool).await;
        assert_eq!(java_components.len(), 1);
        assert_eq!(java_components[0].path, "/usr/bin/java2");
        assert!(java_components[0].is_valid);

        // Set java as invalid
        let pool = db_pool.clone();
        let java_path_clone = java_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            conn.execute(
                "UPDATE Java SET isValid = 0 WHERE path = ?1",
                rusqlite::params![&java_path_clone],
            )
            .unwrap();
        })
        .await
        .unwrap();

        let java_components = get_all_javas(db_pool).await;
        assert_eq!(java_components.len(), 1);
        assert!(!java_components[0].is_valid);

        upsert_java_component_to_db(db_pool, java_component)
            .await
            .unwrap();

        let java_components = get_all_javas(db_pool).await;
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

        let result = upsert_java_component_to_db(db_pool, almost_equal_java_component).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scan_and_sync_local() {
        let app = setup_managers_for_test().await;
        let db_pool = &app.db_pool;

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
        upsert_java_component_to_db(db_pool, component_to_remove)
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

        upsert_java_component_to_db(db_pool, component_to_keep)
            .await
            .unwrap();

        scan_and_sync_local(db_pool, discovery, java_checker)
            .await
            .unwrap();

        let java_components = get_all_javas(db_pool).await;

        println!("{:?}", java_components);

        assert_eq!(java_components.len(), 3);
    }

    #[tokio::test]
    /// This test is to make sure that if a java is invalid and not used in any profile, it will be removed
    /// If it's used in a profile, it will be set as invalid
    async fn test_scan_and_sync_local_broken_javas() {
        let app = setup_managers_for_test().await;
        let db_pool = &app.db_pool;
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

        upsert_java_component_to_db(db_pool, component_to_add)
            .await
            .unwrap();
        let java_id = upsert_java_component_to_db(db_pool, component_to_add_still_used)
            .await
            .unwrap();

        update_java_profile_java_id(
            db_pool,
            &SystemJavaProfileName::Legacy.to_string(),
            Some(&java_id),
        )
        .await;

        scan_and_sync_local(db_pool, discovery, java_checker)
            .await
            .unwrap();

        let java_components = get_all_javas(db_pool).await;

        assert_eq!(java_components.len(), 1);

        assert_eq!(java_components[0].path, "/usr/bin/java1");
        assert!(!java_components[0].is_valid);
    }

    #[tokio::test]
    async fn test_scan_and_sync_managed_broken_javas() {
        let app = setup_managers_for_test().await;
        let db_pool = &app.db_pool;
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

        upsert_java_component_to_db(db_pool, component_to_add)
            .await
            .unwrap();
        let java_id = upsert_java_component_to_db(db_pool, component_to_add_still_used)
            .await
            .unwrap();

        update_java_profile_java_id(
            db_pool,
            &SystemJavaProfileName::Legacy.to_string(),
            Some(&java_id),
        )
        .await;

        scan_and_sync_managed(db_pool, discovery, java_checker)
            .await
            .unwrap();

        let java_components = get_all_javas(db_pool).await;

        assert_eq!(java_components.len(), 1);

        assert_eq!(java_components[0].path, "/my/managed/path1");
        assert!(!java_components[0].is_valid);
    }

    #[tokio::test]
    async fn test_scan_and_sync_custom_broken_javas() {
        let app = setup_managers_for_test().await;
        let db_pool = &app.db_pool;
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

        upsert_java_component_to_db(db_pool, component_to_add)
            .await
            .unwrap();
        let java_id = upsert_java_component_to_db(db_pool, component_to_add_still_used)
            .await
            .unwrap();

        update_java_profile_java_id(
            db_pool,
            &SystemJavaProfileName::Legacy.to_string(),
            Some(&java_id),
        )
        .await;

        scan_and_sync_custom(db_pool, java_checker).await.unwrap();

        let java_components = get_all_javas(db_pool).await;

        assert_eq!(java_components.len(), 2);

        for java_component in java_components {
            assert!(!java_component.is_valid);
        }
    }

    #[tokio::test]
    async fn test_scan_and_sync_managed_on_disk_but_not_on_database() {
        let app = setup_managers_for_test().await;
        let db_pool = &app.db_pool;
        let discovery = &MockDiscovery;
        let java_checker = &MockJavaChecker;

        scan_and_sync_managed(db_pool, discovery, java_checker)
            .await
            .unwrap();

        let java_components = get_all_javas(db_pool).await;

        assert_eq!(java_components.len(), 3);
        for java_component in java_components {
            assert!(java_component.is_valid);
        }
    }

    #[tokio::test]
    async fn test_sync_system_java_profiles_with_profiles() {
        let app = setup_managers_for_test().await;
        let db_pool = &app.db_pool;

        JavaManager::ensure_profiles_in_db(db_pool).await.unwrap();

        // manually set one of the profiles to non-system to make sure it gets updated to system
        let pool = db_pool.clone();
        let profile_name = SystemJavaProfileName::Legacy.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            conn.execute(
                "UPDATE JavaProfile SET isSystemProfile = 0 WHERE name = ?1",
                rusqlite::params![&profile_name],
            )
            .unwrap();
        })
        .await
        .unwrap();

        // Create test java entries
        let pool = db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::CreateJava::execute(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                "my_path1",
                8,
                "1.8.0_282",
                "local",
                "linux",
                "x86_64",
                "Azul Systems, Inc.",
                true,
            )
            .unwrap();
            queries::java::CreateJava::execute(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                "my_path2",
                17,
                "17.0.1",
                "local",
                "linux",
                "x86_64",
                "Azul Systems, Inc.",
                true,
            )
            .unwrap();
            queries::java::CreateJava::execute(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                "my_path3",
                14,
                "14.0.1",
                "local",
                "linux",
                "x86_64",
                "Azul Systems, Inc.",
                false,
            )
            .unwrap();
        })
        .await
        .unwrap();

        JavaManager::ensure_profiles_in_db(db_pool).await.unwrap();
        sync_system_java_profiles(db_pool).await.unwrap();

        let pool = db_pool.clone();
        let all_profiles: Vec<models::JavaProfile> = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::ListJavaProfiles::fetch_all(&conn).unwrap()
        })
        .await
        .unwrap();

        assert!(all_profiles.iter().all(|profile| profile.is_system_profile));

        // Expect 8 and 17 to be there, but not 14 since it's invalid and 16 because not provided
        let pool = db_pool.clone();
        let legacy_profile: models::JavaProfile = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::FindJavaProfileByName::fetch_one(
                &conn,
                &SystemJavaProfileName::Legacy.to_string(),
            )
            .unwrap()
        })
        .await
        .unwrap();

        info!("{:?}", legacy_profile);

        assert!(legacy_profile.java_id.is_some());

        let pool = db_pool.clone();
        let alpha_profile: models::JavaProfile = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::FindJavaProfileByName::fetch_one(
                &conn,
                &SystemJavaProfileName::Alpha.to_string(),
            )
            .unwrap()
        })
        .await
        .unwrap();

        assert!(alpha_profile.java_id.is_none());

        let pool = db_pool.clone();
        let beta_profile: models::JavaProfile = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::FindJavaProfileByName::fetch_one(
                &conn,
                &SystemJavaProfileName::Beta.to_string(),
            )
            .unwrap()
        })
        .await
        .unwrap();

        assert!(beta_profile.java_id.is_some());

        let pool = db_pool.clone();
        let gamma_profile: models::JavaProfile = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::FindJavaProfileByName::fetch_one(
                &conn,
                &SystemJavaProfileName::Gamma.to_string(),
            )
            .unwrap()
        })
        .await
        .unwrap();

        assert!(gamma_profile.java_id.is_some());

        let pool = db_pool.clone();
        let minecraft_exe_profile: models::JavaProfile = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            queries::java::FindJavaProfileByName::fetch_one(
                &conn,
                &SystemJavaProfileName::MinecraftJavaExe.to_string(),
            )
            .unwrap()
        })
        .await
        .unwrap();

        assert!(minecraft_exe_profile.java_id.is_none());
    }

    #[tokio::test]
    async fn test_upsert_java_component_to_db_different_java_configuration() {
        let app = setup_managers_for_test().await;
        let db_pool = &app.db_pool;

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

        upsert_java_component_to_db(db_pool, old_component)
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

        scan_and_sync_local(db_pool, discovery, java_checker)
            .await
            .unwrap();

        upsert_java_component_to_db(db_pool, new_component.clone())
            .await
            .unwrap();

        let java_components = get_all_javas(db_pool).await;

        assert_eq!(java_components.len(), 3);

        let java_component = JavaComponent::try_from(java_components[0].clone()).unwrap();

        assert_eq!(java_component, new_component);
    }
}
