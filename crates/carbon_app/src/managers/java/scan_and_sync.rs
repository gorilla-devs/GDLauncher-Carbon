use std::{path::PathBuf, sync::Arc};

use strum::IntoEnumIterator;
use tracing::{trace, warn};

use crate::{
    db::{read_filters::StringFilter, PrismaClient},
    domain::java::{
        JavaArch, JavaComponent, JavaComponentType, JavaVersion, SystemJavaProfileName,
    },
};

use super::{discovery::Discovery, java_checker::JavaChecker};

#[tracing::instrument(level = "trace", skip(db))]
async fn get_java_component_from_db(
    db: &PrismaClient,
    path: String,
) -> anyhow::Result<Option<crate::db::java::Data>> {
    let res = db
        .java()
        .find_unique(crate::db::java::UniqueWhereParam::PathEquals(path))
        .exec()
        .await?;

    Ok(res)
}

#[tracing::instrument(level = "trace", skip(db))]
pub async fn add_java_component_to_db(
    db: &Arc<PrismaClient>,
    java_component: JavaComponent,
) -> anyhow::Result<String> {
    let already_existing_component =
        get_java_component_from_db(db, java_component.path.clone()).await?;

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

    if let Some((component, is_valid, id)) = already_existing_component {
        if component == java_component {
            if !is_valid {
                db.java()
                    .update(
                        crate::db::java::id::equals(id.clone()),
                        vec![crate::db::java::is_valid::set(true)],
                    )
                    .exec()
                    .await?;
            }

            return Ok(id);
        }

        anyhow::bail!(
            "Component with path {} already exists in db but with different configuration",
            java_component.path
        );
    } else {
        let res = db
            .java()
            .create(
                java_component.path,
                java_component.version.major as i32,
                java_component.version.to_string(),
                java_component._type.to_string(),
                java_component.os.to_string(),
                java_component.arch.to_string(),
                java_component.vendor,
                vec![],
            )
            .exec()
            .await?;

        Ok(res.id)
    }
}

#[tracing::instrument(level = "trace", skip(db))]
async fn update_java_component_in_db_to_invalid(
    db: &Arc<PrismaClient>,
    path: String,
) -> anyhow::Result<()> {
    db.java()
        .update(
            crate::db::java::UniqueWhereParam::PathEquals(path),
            vec![crate::db::java::SetParam::SetIsValid(false)],
        )
        .exec()
        .await?;

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_local<T, G>(
    db: &Arc<PrismaClient>,
    discovery: &T,
    java_checker: &G,
) -> anyhow::Result<()>
where
    T: Discovery,
    G: JavaChecker,
{
    let auto_manage_java = true;
    let local_javas = discovery.find_java_paths().await;
    let java_profiles = db.java_system_profile().find_many(vec![]).exec().await?;

    trace!("Auto Manage Java is {}", auto_manage_java);

    for local_java in &local_javas {
        trace!("Analyzing local java: {:?}", local_java);

        // Attempt to canonicalize path
        let resolved_java_path = match tokio::fs::canonicalize(local_java).await {
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
            get_java_component_from_db(db, resolved_java_path.to_string_lossy().to_string())
                .await?;

        if let Some(db_entry) = &db_entry {
            if JavaComponentType::try_from(&*db_entry.r#type)? != JavaComponentType::Local {
                continue;
            }
        }

        match java_bin_info {
            // If it is valid, check whether it's in the DB
            Ok(java_component) => {
                trace!("Java is valid: {:?}", java_component);
                add_java_component_to_db(db, java_component).await?;
            }
            // If it isn't valid, check whether it's in the DB
            Err(err) => {
                trace!("Java is invalid due to: {:?}", err);
                let is_java_used_in_profile = java_profiles.iter().any(|profile| {
                    let Some(java) = profile.java.as_ref() else {
                        return false;
                    };
                    let Some(java) = java.as_ref() else {
                        return false;
                    };
                    let java_path = java.path.clone();
                    java_path == resolved_java_path.display().to_string()
                });

                // If it is in the db, update it to invalid
                if db_entry.is_some() {
                    if is_java_used_in_profile && !auto_manage_java {
                        update_java_component_in_db_to_invalid(
                            db,
                            resolved_java_path.display().to_string(),
                        )
                        .await?;
                    } else {
                        db.java()
                            .delete(crate::db::java::UniqueWhereParam::PathEquals(
                                resolved_java_path.display().to_string(),
                            ))
                            .exec()
                            .await?;
                    }
                }
            }
        }
    }

    // Cleanup unscanned local javas (if they are not default)
    let local_javas_from_db = db
        .java()
        .find_many(vec![crate::db::java::WhereParam::Type(
            StringFilter::Equals(JavaComponentType::Local.to_string()),
        )])
        .exec()
        .await?;

    for local_java_from_db in local_javas_from_db {
        let has_been_scanned = local_javas
            .iter()
            .any(|local_java| local_java_from_db.path == local_java.display().to_string());

        if has_been_scanned {
            continue;
        }

        let is_used_in_profile = java_profiles
            .iter()
            .filter_map(|profile| {
                let Some(java) = profile.java.as_ref() else {
                    return None;
                };
                let Some(java) = java else {
                    return None;
                };
                Some(java.path.clone())
            })
            .any(|java_profile_path| local_java_from_db.path == java_profile_path);

        if is_used_in_profile && !auto_manage_java {
            update_java_component_in_db_to_invalid(db, local_java_from_db.path).await?;
        } else {
            db.java()
                .delete(crate::db::java::UniqueWhereParam::PathEquals(
                    local_java_from_db.path,
                ))
                .exec()
                .await?;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_custom<G>(db: &Arc<PrismaClient>, java_checker: &G) -> anyhow::Result<()>
where
    G: JavaChecker,
{
    let custom_javas = db
        .java()
        .find_many(vec![crate::db::java::WhereParam::Type(
            StringFilter::Equals(JavaComponentType::Custom.to_string()),
        )])
        .exec()
        .await?;

    for custom_java in custom_javas {
        let java_bin_info = java_checker
            .get_bin_info(
                &PathBuf::from(custom_java.path.clone()),
                JavaComponentType::Custom,
            )
            .await;

        if java_bin_info.is_err() {
            update_java_component_in_db_to_invalid(db, custom_java.path).await?;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn scan_and_sync_managed<G>(
    db: &Arc<PrismaClient>,
    java_checker: &G,
) -> anyhow::Result<()>
where
    G: JavaChecker,
{
    let managed_javas = db
        .java()
        .find_many(vec![crate::db::java::r#type::equals(
            JavaComponentType::Managed.to_string(),
        )])
        .exec()
        .await?;

    for managed_java in managed_javas {
        let java_bin_info = java_checker
            .get_bin_info(
                &PathBuf::from(managed_java.path.clone()),
                JavaComponentType::Managed,
            )
            .await;

        if java_bin_info.is_err() {
            update_java_component_in_db_to_invalid(db, managed_java.path).await?;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn sync_system_java_profiles(db: &Arc<PrismaClient>) -> anyhow::Result<()> {
    let all_javas = db.java().find_many(vec![]).exec().await?;

    let is32bit = std::env::consts::ARCH == "x86" || std::env::consts::ARCH == "arm";

    for profile in SystemJavaProfileName::iter() {
        trace!("Syncing system java profile: {}", profile.to_string());
        let java_in_profile = db
            .java_system_profile()
            .find_unique(crate::db::java_system_profile::name::equals(
                profile.to_string(),
            ))
            .exec()
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Java system profile {} not found in DB",
                    profile.to_string()
                )
            })?
            .java_id;

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
                db.java_system_profile()
                    .update(
                        crate::db::java_system_profile::name::equals(profile.to_string()),
                        vec![crate::db::java_system_profile::java::connect(
                            crate::db::java::id::equals(java.id.clone()),
                        )],
                    )
                    .exec()
                    .await?;
                break;
            }
        }
    }

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
            discovery::MockDiscovery,
            java_checker::{MockJavaChecker, MockJavaCheckerInvalid},
            scan_and_sync::{
                add_java_component_to_db, scan_and_sync_custom, scan_and_sync_local,
                sync_system_java_profiles,
            },
            JavaManager,
        },
        setup_managers_for_test,
    };

    #[tokio::test]
    async fn test_add_component_to_db() {
        let app = setup_managers_for_test().await;
        let db = &app.prisma_client;

        let java_path = "/usr/bin/java2".to_string();

        let java_component = JavaComponent {
            path: java_path.clone(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };
        let java_components = db.java().find_many(vec![]).exec().await.unwrap();
        assert_eq!(java_components.len(), 0);

        add_java_component_to_db(db, java_component.clone())
            .await
            .unwrap();

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();
        assert_eq!(java_components.len(), 1);
        assert_eq!(java_components[0].path, "/usr/bin/java2");
        assert!(java_components[0].is_valid);

        db.java()
            .update(
                crate::db::java::path::equals(java_path.clone()),
                vec![crate::db::java::is_valid::set(false)],
            )
            .exec()
            .await
            .unwrap();

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();
        assert_eq!(java_components.len(), 1);
        assert!(!java_components[0].is_valid);

        add_java_component_to_db(db, java_component).await.unwrap();

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();
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

        let result = add_java_component_to_db(db, almost_equal_java_component).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scan_and_sync_local() {
        let app = setup_managers_for_test().await;
        let db = &app.prisma_client;

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
        add_java_component_to_db(db, component_to_remove)
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

        add_java_component_to_db(db, component_to_keep)
            .await
            .unwrap();

        scan_and_sync_local(db, discovery, java_checker)
            .await
            .unwrap();

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();

        println!("{:?}", java_components);

        assert_eq!(java_components.len(), 3);
    }

    #[tokio::test]
    async fn test_scan_and_sync_local_to_invalid() {
        let app = setup_managers_for_test().await;
        let db = &app.prisma_client;
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

        add_java_component_to_db(db, component_to_add)
            .await
            .unwrap();

        scan_and_sync_local(db, discovery, java_checker)
            .await
            .unwrap();

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();

        // Since the db only contains one component, it should be set as invalid, even tho
        // given that it's not used in any profile, it will be silently removed.
        // The other 2, since they don't already exist, are not added nor updated.

        assert_eq!(java_components.len(), 0);
    }

    #[tokio::test]
    async fn test_scan_and_sync_custom_to_invalid() {
        let app = setup_managers_for_test().await;
        let db = &app.prisma_client;
        let java_checker = &MockJavaCheckerInvalid;

        let component_to_add = JavaComponent {
            path: "/my/custom/path".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Custom,
            arch: JavaArch::X86_32,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        };

        add_java_component_to_db(db, component_to_add)
            .await
            .unwrap();

        scan_and_sync_custom(db, java_checker).await.unwrap();

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();

        // Since the db only contains one component, it should be set as invalid.
        // The other 2, since they don't already exist, are not added nor updated.

        assert_eq!(java_components.len(), 1);
        assert!(!java_components[0].is_valid);
    }

    #[tokio::test]
    async fn test_sync_system_java_profiles_with_profiles() {
        let app = setup_managers_for_test().await;
        let db = &app.prisma_client;

        JavaManager::ensure_profiles_in_db(db).await.unwrap();

        db.java()
            .create_many(vec![
                (
                    "my_path1".to_string(),
                    8,
                    "1.8.0_282".to_string(),
                    "local".to_string(),
                    "linux".to_string(),
                    "x86_64".to_string(),
                    "Azul Systems, Inc.".to_string(),
                    vec![],
                ),
                (
                    "my_path2".to_string(),
                    17,
                    "17.0.1".to_string(),
                    "local".to_string(),
                    "linux".to_string(),
                    "x86_64".to_string(),
                    "Azul Systems, Inc.".to_string(),
                    vec![],
                ),
                (
                    "my_path3".to_string(),
                    14,
                    "17.0.1".to_string(),
                    "local".to_string(),
                    "linux".to_string(),
                    "x86_64".to_string(),
                    "Azul Systems, Inc.".to_string(),
                    vec![crate::db::java::SetParam::SetIsValid(false)],
                ),
            ])
            .exec()
            .await
            .unwrap();

        sync_system_java_profiles(db).await.unwrap();

        // Expect 8 and 17 to be there, but not 14 since it's invalid and 16 because not provided
        let legacy_profile = db
            .java_system_profile()
            .find_unique(
                crate::db::java_system_profile::UniqueWhereParam::NameEquals(
                    SystemJavaProfileName::Legacy.to_string(),
                ),
            )
            .with(crate::db::java_system_profile::java::fetch())
            .exec()
            .await
            .unwrap()
            .unwrap();

        info!("{:?}", legacy_profile);

        assert!(legacy_profile.java.flatten().is_some());

        let alpha_profile = db
            .java_system_profile()
            .find_unique(
                crate::db::java_system_profile::UniqueWhereParam::NameEquals(
                    SystemJavaProfileName::Alpha.to_string(),
                ),
            )
            .with(crate::db::java_system_profile::java::fetch())
            .exec()
            .await
            .unwrap()
            .unwrap();

        assert!(alpha_profile.java.flatten().is_none());

        let beta_profile = db
            .java_system_profile()
            .find_unique(
                crate::db::java_system_profile::UniqueWhereParam::NameEquals(
                    SystemJavaProfileName::Beta.to_string(),
                ),
            )
            .with(crate::db::java_system_profile::java::fetch())
            .exec()
            .await
            .unwrap()
            .unwrap();

        assert!(beta_profile.java.flatten().is_some());

        let gamma_profile = db
            .java_system_profile()
            .find_unique(
                crate::db::java_system_profile::UniqueWhereParam::NameEquals(
                    SystemJavaProfileName::Gamma.to_string(),
                ),
            )
            .with(crate::db::java_system_profile::java::fetch())
            .exec()
            .await
            .unwrap()
            .unwrap();

        assert!(gamma_profile.java.flatten().is_some());

        let minecraft_exe_profile = db
            .java_system_profile()
            .find_unique(
                crate::db::java_system_profile::UniqueWhereParam::NameEquals(
                    SystemJavaProfileName::MinecraftJavaExe.to_string(),
                ),
            )
            .with(crate::db::java_system_profile::java::fetch())
            .exec()
            .await
            .unwrap()
            .unwrap();

        assert!(minecraft_exe_profile.java.flatten().is_none());
    }
}
