//! Per-major default Javas: the Java a user chose for a Java version, which
//! Automatic uses before GDLauncher's own builds.

use crate::domain::java::SystemJavaProfileName;
use carbon_repos::db_error::DbError;
use carbon_repos::db_exec::{Db, ReadAccess, WriteAccess};
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::app_configuration::{AppConfigurationPatch, AppConfigurationRow};
use carbon_repos::repos::{frontend_preference, java as java_repo};
use std::collections::{BTreeMap, HashMap};

/// A JSON object `{"<major>": "<javaId>"}`.
pub const JAVA_MAJOR_DEFAULTS_KEY: &str = "java_major_defaults";
/// `"1"` once [`migrate_major_defaults`] has run.
const JAVA_MAJOR_DEFAULTS_MIGRATED_KEY: &str = "java_major_defaults_migrated";

/// Per major, the profiles whose link becomes its default in the migration,
/// in the order they are tried: the primary profile first.
const MIGRATED_PROFILES: [&[SystemJavaProfileName]; 5] = [
    &[
        SystemJavaProfileName::Legacy,
        SystemJavaProfileName::LegacyFixed1,
    ],
    &[SystemJavaProfileName::Alpha],
    &[
        SystemJavaProfileName::Gamma,
        SystemJavaProfileName::Beta,
        SystemJavaProfileName::GammaSnapshot,
    ],
    &[SystemJavaProfileName::Delta],
    &[SystemJavaProfileName::Epsilon],
];

type Defaults = BTreeMap<u8, String>;

/// A missing or unparsable value reads as empty.
fn read_defaults(conn: &impl ReadAccess) -> Result<Defaults, DbError> {
    let pref = frontend_preference::get_preference_conn(conn, JAVA_MAJOR_DEFAULTS_KEY)?;

    Ok(pref
        .and_then(|p| serde_json::from_str::<Defaults>(&p.value).ok())
        .unwrap_or_default())
}

fn write_defaults(conn: &impl WriteAccess, defaults: &Defaults) -> Result<(), DbError> {
    let value = serde_json::to_string(defaults).map_err(|e| DbError::Conversion(e.to_string()))?;
    frontend_preference::upsert_preference_conn(conn, JAVA_MAJOR_DEFAULTS_KEY, &value, now())?;

    Ok(())
}

fn now() -> DbDateTime {
    DbDateTime(chrono::Utc::now().fixed_offset())
}

/// Reads, changes and writes the defaults in one write dispatch, so two
/// changes never lose each other. `change` returns whether it changed
/// anything; nothing is written otherwise.
async fn update_major_defaults(
    db: &Db,
    change: impl FnOnce(&mut Defaults) -> bool + Send + 'static,
) -> anyhow::Result<bool> {
    let changed = db
        .write(move |mut conn| {
            let tx = conn.transaction()?;
            let mut defaults = read_defaults(&tx)?;
            let changed = change(&mut defaults);
            if changed {
                write_defaults(&tx, &defaults)?;
            }
            tx.commit()?;
            Ok(changed)
        })
        .await?;

    Ok(changed)
}

pub async fn get_major_defaults(db: &Db) -> anyhow::Result<HashMap<u8, String>> {
    let defaults = db.read(|conn| read_defaults(&conn)).await?;

    Ok(defaults.into_iter().collect())
}

/// `None` clears the default, so the major goes back to automatic.
pub async fn set_major_default(db: &Db, major: u8, java_id: Option<String>) -> anyhow::Result<()> {
    update_major_defaults(db, move |defaults| match java_id {
        Some(id) => defaults.insert(major, id.clone()).as_ref() != Some(&id),
        None => defaults.remove(&major).is_some(),
    })
    .await?;

    Ok(())
}

/// Removes every default that points at `java_id`. Returns whether any did.
pub(crate) async fn clear_major_defaults_for(db: &Db, java_id: &str) -> anyhow::Result<bool> {
    let java_id = java_id.to_string();

    update_major_defaults(db, move |defaults| {
        let before = defaults.len();
        defaults.retain(|_, id| *id != java_id);
        defaults.len() != before
    })
    .await
}

/// Points every default at `old_id` to `new_id`. Returns whether any moved.
pub(crate) async fn move_major_defaults(
    db: &Db,
    old_id: &str,
    new_id: &str,
) -> anyhow::Result<bool> {
    let (old_id, new_id) = (old_id.to_string(), new_id.to_string());

    update_major_defaults(db, move |defaults| {
        let mut moved = false;
        for id in defaults.values_mut().filter(|id| **id == old_id) {
            id.clone_from(&new_id);
            moved = true;
        }
        moved
    })
    .await
}

/// The Java each major's profiles link to, by major: the primary profile's
/// link, else the first link among the major's other profiles (see
/// [`MIGRATED_PROFILES`]). Only a link to an existing Java of that major
/// counts; one that fails its check today is kept, since the pick skips an
/// invalid default until it works again.
async fn linked_javas_by_major(db: &Db) -> anyhow::Result<Defaults> {
    let profiles = java_repo::get_all_profiles(db).await?;
    let javas = java_repo::get_all_java(db).await?;
    let mut linked = Defaults::new();

    for group in MIGRATED_PROFILES {
        let major = group[0].major();
        let java_id = group.iter().find_map(|profile| {
            let name = profile.to_string();
            let java_id = profiles
                .iter()
                .find(|row| row.name == name)?
                .java_id
                .as_ref()?;

            javas
                .iter()
                .any(|java| &java.id == java_id && java.major == i32::from(major))
                .then(|| java_id.clone())
        });

        if let Some(java_id) = java_id {
            linked.insert(major, java_id);
        }
    }

    Ok(linked)
}

/// Whether [`migrate_major_defaults`] has run. Until it has, the system
/// profiles hold the Javas picked by hand that it reads, so nothing may
/// relink them.
pub async fn major_defaults_migrated(db: &Db) -> anyhow::Result<bool> {
    Ok(
        frontend_preference::get_preference(db, JAVA_MAJOR_DEFAULTS_MIGRATED_KEY)
            .await?
            .is_some_and(|pref| pref.value == "1"),
    )
}

/// Runs once. When `autoManageJavaSystemProfiles` was off, the Java each major's
/// profiles link to ([`linked_javas_by_major`]) becomes its default, so existing
/// choices survive Automatic taking over. Either way the setting ends up on.
/// Returns whether it wrote any default.
pub async fn migrate_major_defaults(
    db: &Db,
    app_config: &AppConfigurationRow,
) -> anyhow::Result<bool> {
    if major_defaults_migrated(db).await? {
        return Ok(false);
    }

    let linked = if app_config.auto_manage_java_system_profiles {
        Defaults::new()
    } else {
        linked_javas_by_major(db).await?
    };
    let wrote_defaults = !linked.is_empty();

    // One transaction, so a failure leaves the migration to run again next
    // start rather than half done.
    db.write(move |mut conn| {
        let tx = conn.transaction()?;

        if !linked.is_empty() {
            let mut defaults = read_defaults(&tx)?;
            for (major, java_id) in linked {
                defaults.entry(major).or_insert(java_id);
            }
            write_defaults(&tx, &defaults)?;
        }

        let auto_on = AppConfigurationPatch {
            auto_manage_java_system_profiles: Some(true),
            ..Default::default()
        };
        if let Some(query) = auto_on.build() {
            query.execute(&tx)?;
        }

        frontend_preference::upsert_preference_conn(
            &tx,
            JAVA_MAJOR_DEFAULTS_MIGRATED_KEY,
            "1",
            now(),
        )?;
        tx.commit()?;
        Ok(())
    })
    .await?;

    Ok(wrote_defaults)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{instance::info, java::SystemJavaProfileName},
        managers::java::{
            resolve::{AutomaticPick, PickReason, resolve_automatic},
            updates::tests::{make_instance, override_of, setup_old_and_new},
        },
        setup_managers_for_test,
    };
    use carbon_repos::repos::java::{self as java_repo, JavaRow};

    async fn set_auto_manage(app: &crate::App, on: bool) {
        app.db
            .write(move |conn| {
                let patch = carbon_repos::repos::app_configuration::AppConfigurationPatch {
                    auto_manage_java_system_profiles: Some(on),
                    ..Default::default()
                };
                Ok(patch.build().map(|q| q.execute(&conn)).transpose()?)
            })
            .await
            .unwrap();
    }

    async fn auto_manage(app: &crate::App) -> bool {
        app.settings_manager()
            .get_settings()
            .await
            .unwrap()
            .auto_manage_java_system_profiles
    }

    async fn migrate(app: &crate::App) -> bool {
        let settings = app.settings_manager().get_settings().await.unwrap();
        migrate_major_defaults(&app.db, &settings).await.unwrap()
    }

    fn custom_row(id: &str, path: &str, major: i32, version: &str, valid: bool) -> JavaRow {
        JavaRow {
            id: id.into(),
            path: path.into(),
            major,
            full_version: version.into(),
            r#type: "custom".into(),
            os: "linux".into(),
            arch: "x64".into(),
            vendor: "Eclipse Adoptium".into(),
            is_valid: valid,
        }
    }

    async fn link(app: &crate::App, profile: SystemJavaProfileName, java_id: &str) {
        java_repo::set_profile_java(&app.db, &profile.to_string(), Some(java_id))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn defaults_round_trip_and_clear() {
        let app = setup_managers_for_test().await;
        assert!(get_major_defaults(&app.db).await.unwrap().is_empty());

        set_major_default(&app.db, 17, Some("a".into()))
            .await
            .unwrap();
        set_major_default(&app.db, 21, Some("b".into()))
            .await
            .unwrap();
        set_major_default(&app.db, 17, None).await.unwrap();

        assert_eq!(
            get_major_defaults(&app.db).await.unwrap(),
            HashMap::from([(21, "b".to_string())])
        );
    }

    #[tokio::test]
    async fn migration_turns_off_auto_into_defaults() {
        let app = setup_managers_for_test().await;
        set_auto_manage(&app, false).await;
        java_repo::insert_java(
            &app.db,
            custom_row("jdk17", "/opt/jdk17/bin/java", 17, "17.0.12", true),
        )
        .await
        .unwrap();
        java_repo::insert_java(
            &app.db,
            custom_row("broken21", "/opt/jdk21/bin/java", 21, "21.0.5", false),
        )
        .await
        .unwrap();
        link(&app, SystemJavaProfileName::Gamma, "jdk17").await;
        link(&app, SystemJavaProfileName::Delta, "broken21").await;

        assert!(migrate(&app).await);

        // A Java that fails its check today is still the user's choice; the
        // pick skips it while it stays invalid.
        let migrated = HashMap::from([(17, "jdk17".to_string()), (21, "broken21".to_string())]);
        assert_eq!(get_major_defaults(&app.db).await.unwrap(), migrated);
        assert!(auto_manage(&app).await);
        // A 1.18 instance (beta) and a 1.20.4 one (gamma) both resolve to it.
        for profile in [SystemJavaProfileName::Beta, SystemJavaProfileName::Gamma] {
            assert_eq!(
                resolve_automatic(&app.app, profile).await.unwrap(),
                AutomaticPick::Java {
                    id: "jdk17".into(),
                    reason: PickReason::UserDefault
                }
            );
        }

        // Runs once: turning auto off again and relinking changes nothing.
        set_auto_manage(&app, false).await;
        java_repo::insert_java(
            &app.db,
            custom_row("jdk21", "/opt/jdk21b/bin/java", 21, "21.0.5", true),
        )
        .await
        .unwrap();
        link(&app, SystemJavaProfileName::Delta, "jdk21").await;

        assert!(!migrate(&app).await);
        assert_eq!(get_major_defaults(&app.db).await.unwrap(), migrated);
        assert!(!auto_manage(&app).await);
    }

    #[tokio::test]
    async fn migration_falls_back_to_a_majors_other_profiles() {
        let app = setup_managers_for_test().await;
        set_auto_manage(&app, false).await;
        for row in [
            custom_row("jdk17", "/opt/jdk17/bin/java", 17, "17.0.12", true),
            custom_row("other17", "/opt/other17/bin/java", 17, "17.0.10", true),
            custom_row("jdk8", "/opt/jdk8/bin/java", 8, "1.8.0_302", true),
        ] {
            java_repo::insert_java(&app.db, row).await.unwrap();
        }
        // Gamma (the primary) is empty; 1.18 needs beta.
        link(&app, SystemJavaProfileName::Beta, "jdk17").await;
        link(&app, SystemJavaProfileName::LegacyFixed1, "jdk8").await;
        // A link to a Java of another major is not a choice for this one.
        link(&app, SystemJavaProfileName::Delta, "other17").await;

        assert!(migrate(&app).await);

        assert_eq!(
            get_major_defaults(&app.db).await.unwrap(),
            HashMap::from([(17, "jdk17".to_string()), (8, "jdk8".to_string())])
        );
        assert_eq!(
            resolve_automatic(&app.app, SystemJavaProfileName::Beta)
                .await
                .unwrap(),
            AutomaticPick::Java {
                id: "jdk17".into(),
                reason: PickReason::UserDefault
            }
        );
    }

    #[tokio::test]
    async fn migration_keeps_the_primary_link_even_to_an_invalid_java() {
        let app = setup_managers_for_test().await;
        set_auto_manage(&app, false).await;
        for row in [
            custom_row("broken17", "/opt/broken17/bin/java", 17, "17.0.12", false),
            custom_row("jdk17", "/opt/jdk17/bin/java", 17, "17.0.10", true),
        ] {
            java_repo::insert_java(&app.db, row).await.unwrap();
        }
        link(&app, SystemJavaProfileName::Gamma, "broken17").await;
        link(&app, SystemJavaProfileName::Beta, "jdk17").await;

        assert!(migrate(&app).await);

        assert_eq!(
            get_major_defaults(&app.db).await.unwrap(),
            HashMap::from([(17, "broken17".to_string())])
        );
    }

    #[tokio::test]
    async fn migration_with_auto_on_writes_no_defaults() {
        let app = setup_managers_for_test().await;
        java_repo::insert_java(
            &app.db,
            custom_row("found17", "/usr/lib/jvm/17/bin/java", 17, "17.0.12", true),
        )
        .await
        .unwrap();
        link(&app, SystemJavaProfileName::Gamma, "found17").await;

        assert!(!migrate(&app).await);

        assert!(get_major_defaults(&app.db).await.unwrap().is_empty());
        assert!(auto_manage(&app).await);
        // Marked as done: a later switch to manual is not migrated either.
        set_auto_manage(&app, false).await;
        assert!(!migrate(&app).await);
        assert!(get_major_defaults(&app.db).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn deleting_default_java_clears_default_and_path_overrides() {
        let app = setup_managers_for_test().await;
        let path = "/opt/jdk17/bin/java";
        java_repo::insert_java(&app.db, custom_row("c17", path, 17, "17.0.12", true))
            .await
            .unwrap();
        set_major_default(&app.db, 17, Some("c17".into()))
            .await
            .unwrap();
        set_major_default(&app.db, 21, Some("other".into()))
            .await
            .unwrap();
        let pointing = make_instance(
            &app.app,
            "pointing",
            info::JavaOverride::Path(Some(path.into())),
        )
        .await;
        let elsewhere = make_instance(
            &app.app,
            "elsewhere",
            info::JavaOverride::Path(Some("/opt/other/bin/java".into())),
        )
        .await;

        app.java_manager()
            .delete_java_version("c17".into())
            .await
            .unwrap();

        assert_eq!(
            get_major_defaults(&app.db).await.unwrap(),
            HashMap::from([(21, "other".to_string())])
        );
        assert!(override_of(&app.app, pointing).await.is_none());
        assert!(matches!(
            override_of(&app.app, elsewhere).await,
            Some(info::JavaOverride::Path(Some(p))) if p == "/opt/other/bin/java"
        ));
    }

    #[tokio::test]
    async fn replacing_managed_java_moves_default() {
        let app = setup_managers_for_test().await;
        let (old, new) = setup_old_and_new(&app.app).await;
        set_major_default(&app.db, 17, Some(old.id.clone()))
            .await
            .unwrap();

        app.java_manager()
            .replace_managed_java(&old, &new)
            .await
            .unwrap();

        assert_eq!(
            get_major_defaults(&app.db).await.unwrap(),
            HashMap::from([(17, new.id.clone())])
        );
    }
}
