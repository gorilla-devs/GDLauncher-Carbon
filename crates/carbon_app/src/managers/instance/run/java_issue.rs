//! Wrong-Java crashes: after the game exits with an error, the end of its log
//! tells which Java a mod needed. The instance keeps that as its Java issue
//! until it next launches, and one action moves it to that Java, or a newer
//! one, and plays.

use crate::{
    App,
    api::keys::instance::{GET_ALL_INSTANCES, INSTANCE_DETAILS},
    domain::{
        instance::{self as domain, InstanceId, info::JavaOverride},
        java::{Java, JavaArch, JavaComponent, JavaComponentType, JavaVendor},
    },
    managers::{
        ManagerRef,
        instance::{InstanceManager, InvalidInstanceIdError, log::GameLog},
        java::{
            instance_java::{InstanceJavaDetails, MismatchFixTo, instance_java_details_cached},
            managed::{AlreadyInstalling, ManagedJavaCatalog},
            resolve::{JavaRequirement, fix_candidates_where, host_arch, javas_from_rows},
        },
    },
};
use anyhow::{Context, anyhow, bail};
use carbon_repos::repos::java as java_repo;
use regex::Regex;
use std::{borrow::Cow, sync::LazyLock};

/// How many of the last log lines a crash is looked for in.
pub const SCANNED_LINES: usize = 300;

/// Class file version 52 is Java 8, and each Java adds one.
const CLASS_FILE_VERSION_OFFSET: u32 = 44;

/// Java 9 and later: "… (class file version 65.0), this version of the Java
/// Runtime only recognizes class file versions up to 61.0". The first number
/// is the one the class needs.
static CLASS_FILE_VERSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"class file version (\d+)").unwrap());

/// Java 8 and older: "UnsupportedClassVersionError: … Unsupported
/// major.minor version 52.0".
static MAJOR_MINOR_VERSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"UnsupportedClassVersionError.*major\.minor version (\d+)").unwrap()
});

/// Fabric and Quilt: "Mod 'Sodium' (sodium) 0.6.0 requires version 21 or
/// later of 'OpenJDK 64-Bit Server VM' (java), …". Fabric names a dependency
/// `'name' (id)`, and the java mod's name is the JVM's, so it is matched by
/// its id. Other loaders write `of 'Java'` or `of java`.
static REQUIRES_JAVA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)requires version (\d+(?:\.\d+)*) or later of (?:'[^']*' \(java\)|'java'|java(?:[\s,!.]|$))",
    )
    .unwrap()
});

/// Fabric's suggested fix: "Replace 'OpenJDK 64-Bit Server VM' (java) 17
/// with version 21 or later."
static REPLACE_JAVA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)replace '[^']*' \(java\) \S+ with version (\d+(?:\.\d+)*) or later").unwrap()
});

/// The Java major the last [`SCANNED_LINES`] of `lines` say the game needed,
/// the highest when several do. A line may hold several lines of text.
pub fn detect_java_issue(lines: &[&str]) -> Option<u8> {
    let start = lines.len().saturating_sub(SCANNED_LINES);

    lines[start..]
        .iter()
        .filter_map(|line| needed_major(line))
        .max()
}

fn needed_major(line: &str) -> Option<u8> {
    let class_version = CLASS_FILE_VERSION
        .captures(line)
        .or_else(|| MAJOR_MINOR_VERSION.captures(line));
    if let Some(captures) = class_version {
        let version: u32 = captures[1].parse().ok()?;
        return u8::try_from(version.checked_sub(CLASS_FILE_VERSION_OFFSET)?).ok();
    }

    let captures = REQUIRES_JAVA
        .captures(line)
        .or_else(|| REPLACE_JAVA.captures(line))?;
    java_major(&captures[1])
}

/// `22` and `17.0.1` are their first number, and the legacy `1.8` is 8.
fn java_major(version: &str) -> Option<u8> {
    let mut parts = version.split('.');
    let first: u8 = parts.next()?.parse().ok()?;

    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// The Java major a crashed game's log asks for, when it is newer than
/// `used_major`, the one it ran on. These messages all mean "this Java or
/// later", so a newer Java than they ask for is never the problem. Each of
/// the last [`SCANNED_LINES`] log entries is read with the exception logged
/// with it, where a crash often is.
pub fn needed_major_in(log: &GameLog, used_major: u8) -> Option<u8> {
    let entries = log.get_span(log.len().saturating_sub(SCANNED_LINES)..);
    let texts: Vec<Cow<str>> = entries
        .iter()
        .map(|entry| match &entry.throwable {
            Some(throwable) => Cow::Owned(format!("{}\n{throwable}", entry.message)),
            None => Cow::Borrowed(entry.message.as_str()),
        })
        .collect();
    let lines: Vec<&str> = texts.iter().map(|text| text.as_ref()).collect();

    detect_java_issue(&lines).filter(|needed| *needed > used_major)
}

/// A game that crashed because its Java was older than one of its mods
/// needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JavaIssue {
    /// The oldest Java the crash says works.
    pub needed_major: u8,
    pub used_major: u8,
    /// The Java a fix would use (see [`planned_fix_major`]).
    pub fix_major: u8,
}

impl JavaIssue {
    /// The issue of a crash on Java `used_major` that needed `needed_major`,
    /// with the Java a fix would use worked out from the installed Javas and
    /// the last Azul catalog fetched, never waiting on the network.
    pub async fn plan(app: &App, needed_major: u8, used_major: u8) -> anyhow::Result<Self> {
        let javas = javas_from_rows(java_repo::get_all_java(&app.db).await?);
        let catalog = app
            .java_manager()
            .managed_service
            .cached_catalog(JavaVendor::Azul);

        Ok(Self {
            needed_major,
            used_major,
            fix_major: planned_fix_major(needed_major, &javas, catalog.as_ref(), host_arch()),
        })
    }
}

/// The Javas a fix after a crash may move an instance to, best first (see
/// [`fix_candidates_where`]): the valid ones of `needed` or later.
pub(crate) fn fix_candidates(needed: u8, javas: &[Java], host_arch: JavaArch) -> Vec<(String, u8)> {
    fix_candidates_where(javas, host_arch, |_, major| major >= needed)
}

/// The oldest major of `needed` or later the catalog has a build of.
pub(crate) fn catalog_major_for(needed: u8, catalog: &ManagedJavaCatalog) -> Option<u8> {
    catalog
        .majors
        .iter()
        .filter(|m| m.major >= needed && !m.builds.is_empty())
        .map(|m| m.major)
        .min()
}

/// The Java major a fix would use: the first of [`fix_candidates`], else the
/// one [`catalog_major_for`] picks, else `needed` when neither tells.
pub(crate) fn planned_fix_major(
    needed: u8,
    javas: &[Java],
    catalog: Option<&ManagedJavaCatalog>,
    host_arch: JavaArch,
) -> u8 {
    fix_candidates(needed, javas, host_arch)
        .first()
        .map(|(_, major)| *major)
        .or_else(|| catalog.and_then(|catalog| catalog_major_for(needed, catalog)))
        .unwrap_or(needed)
}

/// A launch held back because the instance is set to a Java that doesn't fit
/// it (see `InstanceDetails::java_override_mismatch`). The UI asks first:
/// the fix ([`fix_java_mismatch`](ManagerRef::fix_java_mismatch)) and play,
/// or play anyway, launching with the check skipped.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{instance_name} is set to Java {used_major}, which doesn't fit it")]
pub struct JavaMismatchError {
    pub instance_id: i32,
    pub instance_name: String,
    /// The Java Automatic would give it.
    pub needed_major: u8,
    pub used_major: u8,
    /// The Java the fix moves it to.
    pub fix_major: u8,
}

impl crate::error::FeErrorCode for JavaMismatchError {
    fn error_code(&self) -> &'static str {
        "JAVA_MISMATCH"
    }

    /// As the crash help dialog takes it.
    fn error_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "instanceId": self.instance_id,
            "instanceName": self.instance_name,
            "neededMajor": self.needed_major,
            "usedMajor": self.used_major,
            "fixMajor": self.fix_major,
        }))
    }
}

/// Each try either finds a Java or waits out another install of the build it
/// would download.
const FIX_ATTEMPTS: usize = 3;

/// The Java a fix moves an instance to: the first of [`fix_candidates`] that
/// runs, else the newest Azul build of the major [`catalog_major_for`] picks,
/// installed with the install progress every Java install has.
async fn java_for_fix(app: &App, needed: u8) -> anyhow::Result<JavaComponent> {
    let no_download = || format!("Could not download Java {needed} or later");

    for _ in 0..FIX_ATTEMPTS {
        let javas = javas_from_rows(java_repo::get_all_java(&app.db).await?);
        for (id, major) in fix_candidates(needed, &javas, host_arch()) {
            let req = JavaRequirement::of_major(major);
            // One that fails its check is marked invalid, so it isn't tried
            // again.
            if let Some(java) = app.java_manager().check_java_for(&id, &req).await? {
                return Ok(java);
            }
        }

        let catalog = app
            .java_manager()
            .managed_service
            .get_catalog(JavaVendor::Azul)
            .await
            .with_context(no_download)?;
        let Some(major) = catalog_major_for(needed, &catalog) else {
            bail!(no_download());
        };
        let failed = || format!("Could not download Java {major}");

        let req = JavaRequirement::of_major(major);
        match app
            .java_manager()
            .require_java_install(&req, false, None)
            .await
        {
            Ok(Some(java)) => return Ok(java),
            Ok(None) => bail!(failed()),
            Err(err) => {
                let Some(in_flight) = err.downcast_ref::<AlreadyInstalling>() else {
                    return Err(err.context(failed()));
                };
                // Its result is in the database once it ends.
                app.java_manager()
                    .managed_service
                    .wait_for_install(&in_flight.build_id, None)
                    .await
                    .with_context(failed)?;
            }
        }
    }

    bail!(no_download())
}

impl ManagerRef<'_, InstanceManager> {
    /// The Java issue of the instance's last game, if it crashed with one.
    pub async fn java_issue(self, instance_id: InstanceId) -> anyhow::Result<Option<JavaIssue>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        Ok(instance.type_.data()?.java_issue)
    }

    pub async fn set_java_issue(
        self,
        instance_id: InstanceId,
        issue: Option<JavaIssue>,
    ) -> anyhow::Result<()> {
        {
            let mut instances = self.instances.write().await;
            let instance = instances
                .get_mut(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;
            let data = instance.type_.data_mut()?;
            if data.java_issue == issue {
                return Ok(());
            }
            data.java_issue = issue;
        }

        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some((*instance_id).into()));

        Ok(())
    }

    /// Sets the instance to a Java its last crash says works, installing one
    /// when there is none, and launches it.
    pub async fn fix_java_and_launch(self, instance_id: InstanceId) -> anyhow::Result<()> {
        // Before anything downloads: without an account nothing launches.
        let account = self
            .app
            .account_manager()
            .get_active_account()
            .await?
            .ok_or_else(|| anyhow!("attempted to launch instance without an account"))?;

        self.fix_java_issue(instance_id).await?;
        // After the fix, so "Play anyway" plays the fixed Java.
        self.check_launch_memory(instance_id).await?;
        self.prepare_game(instance_id, Some(account), None, false)
            .await?;

        Ok(())
    }

    /// Moves the instance to a Java its last crash says works (see
    /// [`java_for_fix`]), raises its Java floor to that Java's major, so the
    /// fix fits and Automatic picks that major, and clears the issue. A
    /// GDLauncher Java leaves the instance on Automatic, which then picks it
    /// and follows its updates; a found or user-added one is set in the
    /// instance.
    async fn fix_java_issue(self, instance_id: InstanceId) -> anyhow::Result<()> {
        let issue = self
            .java_issue(instance_id)
            .await?
            .ok_or_else(|| anyhow!("This instance has no Java problem to fix"))?;
        if self.is_launch_active(instance_id).await? {
            bail!("Close the game first");
        }

        let java = java_for_fix(self.app, issue.needed_major).await?;
        let floor = u8::try_from(java.version.major)?;
        let java_override = match java._type {
            JavaComponentType::Managed => None,
            JavaComponentType::Local | JavaComponentType::Custom => {
                Some(JavaOverride::Path(Some(java.path)))
            }
        };

        self.set_instance_java(instance_id, java_override, Some(Some(floor)))
            .await?;
        self.set_java_issue(instance_id, None).await
    }

    /// Writes the instance's Java override, and its Java floor when `floor`
    /// is set.
    async fn set_instance_java(
        self,
        instance_id: InstanceId,
        java_override: Option<JavaOverride>,
        floor: Option<Option<u8>>,
    ) -> anyhow::Result<()> {
        self.update_instance(domain::InstanceSettingsUpdate {
            instance_id,
            name: None,
            use_loaded_icon: None,
            notes: None,
            version: None,
            modloader: None,
            java_override: Some(java_override),
            java_floor: floor,
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
        .await?;

        Ok(())
    }

    /// What a launch asks first when the instance is set to a Java that
    /// doesn't fit it; `None` when it fits, or is on Automatic.
    pub async fn java_mismatch_before_launch(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Option<JavaMismatchError>> {
        let (name, details) = self.java_details_from_cache(instance_id).await?;
        if !details.mismatch {
            return Ok(None);
        }

        let used = details
            .resolved
            .and_then(|resolved| resolved.resolution.major);
        let (Some(fix), Some(used_major), Some(need)) = (details.mismatch_fix, used, details.need)
        else {
            return Ok(None);
        };

        Ok(Some(JavaMismatchError {
            instance_id: instance_id.0,
            instance_name: name,
            needed_major: need.major,
            used_major,
            fix_major: fix.major,
        }))
    }

    /// The instance's name and what its details say about its Java, from the
    /// version data there is (see [`instance_java_details_cached`]): PLAY
    /// never waits on the network for it.
    async fn java_details_from_cache(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<(String, InstanceJavaDetails)> {
        let (name, java_override, version, floor) = {
            let instances = self.instances.read().await;
            let data = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?
                .data()?;

            (
                data.name().to_string(),
                data.java_override().cloned(),
                data.game_version().cloned(),
                data.java_floor(),
            )
        };

        let details =
            instance_java_details_cached(self.app, java_override.as_ref(), version.as_ref(), floor)
                .await?;

        Ok((name, details))
    }

    /// Moves an instance set to a Java that doesn't fit it onto one that
    /// does, as its details plan it (`InstanceDetails::java_mismatch_fix`):
    /// Automatic, with the floor raised when that's what makes Automatic
    /// pick the GDLauncher Java the plan found, or a found or user-added
    /// Java set in the instance. One of those that no longer starts leaves
    /// the instance on Automatic, which downloads what fits. Does nothing
    /// when the Java fits already.
    pub async fn fix_java_mismatch(self, instance_id: InstanceId) -> anyhow::Result<()> {
        if self.is_launch_active(instance_id).await? {
            bail!("Close the game first");
        }

        let (_, details) = self.java_details_from_cache(instance_id).await?;
        let Some(fix) = details.mismatch_fix else {
            tracing::debug!("instance {} already fits its Java", instance_id.0);
            return Ok(());
        };

        let (java_override, floor) = match fix.to {
            MismatchFixTo::Automatic { floor } => (None, floor.map(Some)),
            MismatchFixTo::Java { id, path, req } => {
                match self.app.java_manager().check_java_for(&id, &req).await? {
                    Some(_) => (Some(JavaOverride::Path(Some(path))), None),
                    None => (None, None),
                }
            }
        };

        self.set_instance_java(instance_id, java_override, floor)
            .await
    }

    /// [`fix_java_mismatch`](Self::fix_java_mismatch), then launches.
    pub async fn fix_java_mismatch_and_launch(self, instance_id: InstanceId) -> anyhow::Result<()> {
        // Before anything changes: without an account nothing launches.
        let account = self
            .app
            .account_manager()
            .get_active_account()
            .await?
            .ok_or_else(|| anyhow!("attempted to launch instance without an account"))?;

        self.fix_java_mismatch(instance_id).await?;
        // After the fix, so "Play anyway" plays the fixed Java.
        self.check_launch_memory(instance_id).await?;
        self.prepare_game(instance_id, Some(account), None, false)
            .await?;

        Ok(())
    }

    /// Whether the instance is queued, preparing or running.
    async fn is_launch_active(self, instance_id: InstanceId) -> anyhow::Result<bool> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        Ok(instance.type_.data()?.is_launch_active())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::java::{JavaOs, JavaVersion},
        managers::{instance::log::LogEntry, java::instance_java::JavaVia},
    };

    const CLASS_VERSION_61: &str = "java.lang.UnsupportedClassVersionError: \
        net/fabricmc/loader/impl/launch/knot/KnotClient has been compiled by a more recent \
        version of the Java Runtime (class file version 61.0), this version of the Java \
        Runtime only recognizes class file versions up to 52.0";
    const CLASS_VERSION_65: &str = "Exception in thread \"main\" \
        java.lang.UnsupportedClassVersionError: me/jellysquid/mods/sodium/SodiumClientMod \
        has been compiled by a more recent version of the Java Runtime (class file version \
        65.0), this version of the Java Runtime only recognizes class file versions up to 61.0";
    const FABRIC_22: &str = "\t - Mod 'Some Mod' (somemod) 2.0.0 requires version 22 or \
        later of 'Java', but only the wrong version is present: 21!";

    #[test]
    fn reads_the_needed_major_from_the_class_file_version() {
        assert_eq!(
            detect_java_issue(&["[main/INFO]: Loading", CLASS_VERSION_61]),
            Some(17)
        );
        assert_eq!(
            detect_java_issue(&[
                CLASS_VERSION_65,
                "\tat java.base/java.lang.ClassLoader.defineClass1(Native Method)"
            ]),
            Some(21)
        );
        assert_eq!(
            detect_java_issue(&[
                "java.lang.UnsupportedClassVersionError: a/B : Unsupported major.minor version 52.0"
            ]),
            Some(8)
        );
    }

    #[test]
    fn reads_the_needed_major_from_fabric_and_quilt() {
        assert_eq!(detect_java_issue(&[FABRIC_22]), Some(22));
        assert_eq!(
            detect_java_issue(&[
                "Mod 'Fabric API' (fabric-api) 0.100.1 requires version 22 or later of java, which is missing!"
            ]),
            Some(22)
        );
        assert_eq!(
            detect_java_issue(&["requires version 22 or later of JAVA"]),
            Some(22)
        );
        assert_eq!(
            detect_java_issue(&["requires version 1.8 or later of 'Java'"]),
            Some(8)
        );
    }

    /// fabric-loader 0.19.5 names a dependency `'name' (id)`, and its builtin
    /// java mod by `java.vm.name`.
    const FABRIC_REAL: &str = "\t - Mod 'Sodium' (sodium) 0.6.0 requires version 21 or later \
        of 'OpenJDK 64-Bit Server VM' (java), but only the wrong version is present: 17!";
    const FABRIC_SOLUTION: &str =
        "\t - Replace 'OpenJDK 64-Bit Server VM' (java) 17 with version 21 or later.";

    #[test]
    fn reads_fabrics_real_message_and_solution() {
        assert_eq!(detect_java_issue(&[FABRIC_REAL]), Some(21));
        assert_eq!(detect_java_issue(&[FABRIC_SOLUTION]), Some(21));
    }

    #[test]
    fn a_mod_named_like_java_is_no_java_issue() {
        assert_eq!(
            detect_java_issue(&[
                "\t - Mod 'Shaders' (shaders) 1.0 requires version 2.1 or later of 'Java Utils' (javautils), which is missing!",
                "\t - Mod 'X' (x) 1.0 requires version 3 or later of 'javalib' (javalib), which is missing!",
                "\t - Replace 'Java Utils' (javautils) 1.0 with version 2.1 or later.",
            ]),
            None
        );
    }

    #[test]
    fn an_unrelated_crash_is_no_java_issue() {
        assert_eq!(
            detect_java_issue(&[
                "---- Minecraft Crash Report ----",
                "java.lang.NullPointerException: Cannot invoke \"net.minecraft.class_310.method_1551()\"",
                "Mod 'Iris' (iris) 1.7.0 requires version 0.5.8 or later of 'Sodium' (sodium), which is missing!",
                "Mod 'X' (x) 1.0 requires version 2.0 or later of 'javascript-engine', which is missing!",
                "Process exited with code 1",
            ]),
            None
        );
    }

    #[test]
    fn only_the_last_lines_are_scanned() {
        let mut lines = vec![CLASS_VERSION_65];
        lines.extend(std::iter::repeat_n(
            "[Render thread/INFO]: still running",
            SCANNED_LINES,
        ));
        assert_eq!(detect_java_issue(&lines), None);

        // One line fewer and the match is inside them.
        assert_eq!(detect_java_issue(&lines[..SCANNED_LINES]), Some(21));
    }

    #[test]
    fn several_matches_need_the_highest() {
        assert_eq!(
            detect_java_issue(&[CLASS_VERSION_65, FABRIC_22, CLASS_VERSION_61]),
            Some(22)
        );
    }

    fn log_of(lines: &[&str]) -> GameLog {
        let mut log = GameLog::new();
        for line in lines {
            log.add_entry(LogEntry::system_message(line));
        }
        log
    }

    #[test]
    fn a_crash_is_an_issue_only_on_an_older_java() {
        let log = log_of(&["[main/INFO]: Loading", CLASS_VERSION_65, "exit code: 1"]);

        assert_eq!(needed_major_in(&log, 17), Some(21));
        assert_eq!(needed_major_in(&log, 21), None);
        assert_eq!(needed_major_in(&log, 25), None);
        assert_eq!(needed_major_in(&log_of(&["exit code: 1"]), 17), None);
    }

    #[test]
    fn a_crash_only_in_a_logged_exception_is_found() {
        let mut log = log_of(&["[main/INFO]: Loading"]);
        log.add_entry(LogEntry {
            throwable: Some(format!(
                "{CLASS_VERSION_65}\n\tat java.base/java.lang.ClassLoader.defineClass1(Native Method)"
            )),
            ..LogEntry::system_error("Failed to start the game")
        });
        log.add_entry(LogEntry::system_message("exit code: 1"));

        assert_eq!(needed_major_in(&log, 17), Some(21));
    }

    fn java(id: &str, ty: JavaComponentType, version: &str, arch: JavaArch, valid: bool) -> Java {
        Java {
            id: id.into(),
            component: JavaComponent {
                path: format!("/javas/{id}/bin/java"),
                arch,
                os: JavaOs::Linux,
                _type: ty,
                version: JavaVersion::try_from(version).unwrap(),
                vendor: "Azul Systems, Inc.".into(),
            },
            is_valid: valid,
        }
    }

    #[test]
    fn a_fix_prefers_the_oldest_java_that_works_then_gdlaunchers_then_native_then_newest() {
        let native = JavaArch::X86_64;
        let other = JavaArch::Arm64;
        let javas = [
            java(
                "found-new",
                JavaComponentType::Local,
                "21.0.9",
                native,
                true,
            ),
            java(
                "managed-old",
                JavaComponentType::Managed,
                "21.0.5",
                native,
                true,
            ),
            java(
                "managed-foreign",
                JavaComponentType::Managed,
                "21.0.8",
                other,
                true,
            ),
            java(
                "managed-new",
                JavaComponentType::Managed,
                "21.0.7",
                native,
                true,
            ),
            java(
                "managed-broken",
                JavaComponentType::Managed,
                "21.0.10",
                native,
                false,
            ),
            java("user", JavaComponentType::Custom, "21.0.3", native, true),
            java(
                "managed-17",
                JavaComponentType::Managed,
                "17.0.13",
                native,
                true,
            ),
            java(
                "managed-22",
                JavaComponentType::Managed,
                "22.0.2",
                native,
                true,
            ),
        ];
        let ids = |candidates: Vec<(String, u8)>| {
            candidates.into_iter().map(|(id, _)| id).collect::<Vec<_>>()
        };

        assert_eq!(
            ids(fix_candidates(21, &javas, native)),
            [
                "managed-new",
                "managed-old",
                "managed-foreign",
                "found-new",
                "user",
                "managed-22"
            ]
        );
        // Every Java of that major or later, oldest major first.
        assert_eq!(
            fix_candidates(17, &javas, native).first(),
            Some(&("managed-17".to_string(), 17))
        );
    }

    #[test]
    fn a_fix_has_no_candidates_without_a_valid_java_of_the_major() {
        let javas = [
            java(
                "m17",
                JavaComponentType::Managed,
                "17.0.13",
                JavaArch::X86_64,
                true,
            ),
            java(
                "broken21",
                JavaComponentType::Managed,
                "21.0.5",
                JavaArch::X86_64,
                false,
            ),
        ];
        assert!(fix_candidates(21, &javas, JavaArch::X86_64).is_empty());
    }

    fn catalog_of(majors: &[u8]) -> ManagedJavaCatalog {
        use crate::managers::java::test_support::build;

        ManagedJavaCatalog::from_builds(
            majors
                .iter()
                .map(|major| build(&format!("b{major}"), &[*major as u32, 0, 1]))
                .collect(),
        )
    }

    #[test]
    fn a_fix_plans_the_oldest_java_that_works_installed_or_downloadable() {
        let native = JavaArch::X86_64;
        let only17 = [java(
            "m17",
            JavaComponentType::Managed,
            "17.0.13",
            native,
            true,
        )];

        // Needs 16, only 17 installed: 17.
        assert_eq!(planned_fix_major(16, &only17, None, native), 17);
        // Nothing installed, and the catalog has no 16 (Linux): 17.
        let no16 = catalog_of(&[25, 21, 17, 8]);
        assert_eq!(planned_fix_major(16, &[], Some(&no16), native), 17);
        assert_eq!(catalog_major_for(16, &no16), Some(17));
        assert_eq!(catalog_major_for(26, &no16), None);
        // Nothing to tell from: the Java the crash asked for.
        assert_eq!(planned_fix_major(16, &[], None, native), 16);
        assert_eq!(planned_fix_major(26, &[], Some(&no16), native), 26);
    }

    /// A `java` the real checker accepts: it prints what JavaCheck would.
    #[cfg(unix)]
    fn fake_java(dir: &std::path::Path, version: &str) -> String {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(dir).unwrap();
        let bin = dir.join("java");
        std::fs::write(
            &bin,
            format!(
                "#!/bin/sh\necho java.version={version}\necho os.arch={}\necho java.vendor=Azul Systems, Inc.\n",
                std::env::consts::ARCH
            ),
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin.to_string_lossy().into_owned()
    }

    #[cfg(unix)]
    async fn add_runnable(app: &App, id: &str, ty: &str, version: &str) -> String {
        use crate::managers::java::test_support::{JavaRow, major_of};

        let path = fake_java(
            &app.settings_manager()
                .runtime_path
                .get_temp()
                .to_path()
                .join(id)
                .join("bin"),
            version,
        );
        java_repo::insert_java(
            &app.db,
            JavaRow {
                id: id.into(),
                path: path.clone(),
                major: major_of(version),
                full_version: version.into(),
                r#type: ty.into(),
                os: "linux".into(),
                arch: host_arch().to_string(),
                vendor: "Azul Systems, Inc.".into(),
                is_valid: true,
            },
        )
        .await
        .unwrap();
        path
    }

    const CRASH_ON_17: JavaIssue = JavaIssue {
        needed_major: 21,
        used_major: 17,
        fix_major: 21,
    };

    /// A GDLauncher Java: the instance stays on Automatic, which picks it
    /// through the floor and follows its updates.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_fix_onto_a_gdlauncher_java_keeps_the_instance_on_automatic() {
        use crate::managers::java::test_support::{add_instance, test_app};

        let app = test_app(vec![]).await;
        add_runnable(&app.app, "m21", "managed", "21.0.5").await;
        add_runnable(&app.app, "found21", "local", "21.0.9").await;
        add_runnable(&app.app, "m17", "managed", "17.0.13").await;
        let manager = app.instance_manager();
        let id = add_instance(&app.app, "Crashy", "1.20.4", false, None).await;
        manager.set_java_issue(id, Some(CRASH_ON_17)).await.unwrap();

        manager.fix_java_issue(id).await.unwrap();

        let details = manager.instance_details(id).await.unwrap();
        assert!(
            details.java_override.is_none(),
            "{:?}",
            details.java_override
        );
        assert_eq!(details.java_floor, Some(21));
        let resolved = details.resolved_java.unwrap().resolution;
        assert_eq!(
            (resolved.java_id.as_deref(), resolved.via),
            (Some("m21"), JavaVia::Automatic)
        );
        assert_eq!(details.java_issue, None);
    }

    /// No GDLauncher Java of that major: the found one is set in the
    /// instance, with the floor.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_fix_onto_a_found_java_sets_it_in_the_instance() {
        use crate::managers::java::test_support::{add_instance, test_app};

        let app = test_app(vec![]).await;
        let found = add_runnable(&app.app, "found21", "local", "21.0.9").await;
        add_runnable(&app.app, "m17", "managed", "17.0.13").await;
        let manager = app.instance_manager();
        let id = add_instance(&app.app, "Crashy", "1.20.4", false, None).await;
        manager.set_java_issue(id, Some(CRASH_ON_17)).await.unwrap();

        manager.fix_java_issue(id).await.unwrap();

        let details = manager.instance_details(id).await.unwrap();
        assert!(
            matches!(&details.java_override, Some(JavaOverride::Path(Some(path))) if *path == found),
            "{:?}",
            details.java_override
        );
        assert_eq!(details.java_floor, Some(21));
        assert!(!details.java_override_mismatch);
        assert_eq!(details.java_issue, None);
    }

    /// A mod on a 1.20.4 pack (Minecraft needs Java 17) needed Java 21. The
    /// fix is no mismatch, and Automatic keeps the instance on Java 21.
    #[cfg(unix)]
    #[tokio::test]
    async fn after_a_fix_the_newer_java_fits_and_automatic_keeps_it() {
        use crate::managers::java::{
            instance_java::java_mismatches,
            test_support::{add_instance, test_app},
        };

        let app = test_app(vec![]).await;
        let managed21 = add_runnable(&app.app, "m21", "managed", "21.0.5").await;
        add_runnable(&app.app, "m17", "managed", "17.0.13").await;
        let manager = app.instance_manager();
        let id = add_instance(&app.app, "Modded", "1.20.4", false, None).await;
        manager.set_java_issue(id, Some(CRASH_ON_17)).await.unwrap();

        manager.fix_java_issue(id).await.unwrap();

        // Set to the fix's Java by hand, it fits too.
        set_override(&app.app, id, Some(JavaOverride::Path(Some(managed21)))).await;
        let details = manager.instance_details(id).await.unwrap();
        assert!(!details.java_override_mismatch);
        assert!(java_mismatches(&app.app).await.is_empty());

        // "Use Automatic" must not bring the crash back.
        set_override(&app.app, id, None).await;
        let details = manager.instance_details(id).await.unwrap();
        let resolved = details.resolved_java.unwrap().resolution;
        assert_eq!(
            (resolved.java_id.as_deref(), resolved.major),
            (Some("m21"), Some(21))
        );
        assert_eq!(details.required_java.map(|req| req.major), Some(21));
        assert!(!details.java_override_mismatch);

        // Java 17 no longer fits it.
        let m17 = java_repo::get_java_by_id(&app.db, "m17")
            .await
            .unwrap()
            .unwrap();
        set_override(&app.app, id, Some(JavaOverride::Path(Some(m17.path)))).await;
        assert!(
            manager
                .instance_details(id)
                .await
                .unwrap()
                .java_override_mismatch
        );
    }

    async fn set_override(app: &App, instance_id: InstanceId, java_override: Option<JavaOverride>) {
        app.instance_manager()
            .update_instance(domain::InstanceSettingsUpdate {
                instance_id,
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

    #[tokio::test]
    async fn a_fix_that_cannot_install_the_java_keeps_the_issue() {
        use crate::managers::java::test_support::{add_instance, test_app};

        // No Java 21 here, and the catalog has nothing to install.
        let app = test_app(vec![]).await;
        let manager = app.instance_manager();
        let id = add_instance(&app.app, "Crashy", "1.20.4", false, None).await;
        manager.set_java_issue(id, Some(CRASH_ON_17)).await.unwrap();

        let error = manager.fix_java_issue(id).await.unwrap_err();
        assert_eq!(error.to_string(), "Could not download Java 21 or later");

        let details = manager.instance_details(id).await.unwrap();
        assert!(
            details.java_override.is_none(),
            "{:?}",
            details.java_override
        );
        assert_eq!(details.java_issue, Some(CRASH_ON_17));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_fix_for_java_16_uses_an_installed_java_17() {
        use crate::managers::java::test_support::{add_instance, test_app};

        let app = test_app(vec![]).await;
        let java17 = add_runnable(&app.app, "m17", "managed", "17.0.13").await;
        add_runnable(&app.app, "m21", "managed", "21.0.5").await;
        let manager = app.instance_manager();
        let id = add_instance(&app.app, "Old modpack", "1.17", false, None).await;
        let issue = JavaIssue::plan(&app.app, 16, 8).await.unwrap();
        assert_eq!(issue.fix_major, 17);
        manager.set_java_issue(id, Some(issue)).await.unwrap();

        manager.fix_java_issue(id).await.unwrap();

        let details = manager.instance_details(id).await.unwrap();
        assert!(
            details.java_override.is_none(),
            "{:?}",
            details.java_override
        );
        assert_eq!(details.java_floor, Some(17));
        assert_eq!(
            details.resolved_java.unwrap().resolution.path.as_deref(),
            Some(java17.as_str())
        );
    }

    #[tokio::test]
    async fn a_fix_for_java_16_downloads_17_when_the_catalog_has_no_16() {
        use crate::managers::java::test_support::{add_instance, build, test_app};

        // The build can't really download, so the fix fails on it, naming it.
        let app = test_app(vec![build("b17", &[17, 0, 13]), build("b21", &[21, 0, 5])]).await;
        let manager = app.instance_manager();
        let id = add_instance(&app.app, "Old modpack", "1.17", false, None).await;
        let issue = JavaIssue::plan(&app.app, 16, 8).await.unwrap();
        assert_eq!(issue.fix_major, 17);
        manager.set_java_issue(id, Some(issue)).await.unwrap();

        let error = manager.fix_java_issue(id).await.unwrap_err();
        assert_eq!(error.to_string(), "Could not download Java 17");
    }

    /// An instance set to Java 17 for Minecraft 1.21.4, which needs 21: the
    /// launch asks first, and the fix moves it to a Java 21.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_launch_on_a_java_that_does_not_fit_asks_first() {
        use crate::managers::java::test_support::{add_instance, test_app};

        let app = test_app(vec![]).await;
        let java17 = add_runnable(&app.app, "l17", "local", "17.0.13").await;
        let manager = app.instance_manager();
        // The check reads only the version data there is: the library's
        // tiles load it.
        crate::managers::java::minecraft_majors::minecraft_version_profiles(&app.app).await;
        let id = add_instance(
            &app.app,
            "Survival",
            "1.21.4",
            false,
            Some(JavaOverride::Path(Some(java17))),
        )
        .await;

        let asked = manager.java_mismatch_before_launch(id).await.unwrap();
        assert_eq!(
            asked,
            Some(JavaMismatchError {
                instance_id: id.0,
                instance_name: "Survival".into(),
                needed_major: 21,
                used_major: 17,
                fix_major: 21,
            })
        );

        // The UI tells it by its code.
        let error = crate::error::FeError::from_anyhow(&asked.unwrap().into());
        let error = serde_json::to_value(&error).unwrap();
        assert_eq!(error["cause"][0]["code"], "JAVA_MISMATCH");
        assert_eq!(error["cause"][0]["data"]["fixMajor"], 21);
        assert_eq!(error["cause"][0]["data"]["usedMajor"], 17);
        assert_eq!(error["cause"][0]["data"]["instanceName"], "Survival");

        // A found Java 21 fits: the fix sets it.
        let java21 = add_runnable(&app.app, "l21", "local", "21.0.9").await;
        manager.fix_java_mismatch(id).await.unwrap();
        let details = manager.instance_details(id).await.unwrap();
        assert!(
            matches!(&details.java_override, Some(JavaOverride::Path(Some(path))) if *path == java21),
            "{:?}",
            details.java_override
        );
        assert!(!details.java_override_mismatch);
        assert_eq!(manager.java_mismatch_before_launch(id).await.unwrap(), None);

        // On Automatic, nothing to ask.
        let auto = add_instance(&app.app, "Auto", "1.21.4", false, None).await;
        assert_eq!(
            manager.java_mismatch_before_launch(auto).await.unwrap(),
            None
        );
    }

    /// GDLauncher's Java for 1.17 is 17 while Java 16 could download: the
    /// fix goes back to Automatic with the floor at 17.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_mismatch_fix_onto_a_gdlauncher_java_keeps_automatic() {
        use crate::managers::java::test_support::{add_instance, build, test_app};

        let app = test_app(vec![build("b16", &[16, 0, 2]), build("b17", &[17, 0, 13])]).await;
        let java11 = add_runnable(&app.app, "l11", "local", "11.0.20").await;
        let java17 = add_runnable(&app.app, "m17", "managed", "17.0.13").await;
        let manager = app.instance_manager();
        // The check reads only the version data there is: the library's
        // tiles load it.
        crate::managers::java::minecraft_majors::minecraft_version_profiles(&app.app).await;
        let id = add_instance(
            &app.app,
            "Caves",
            "1.17",
            false,
            Some(JavaOverride::Path(Some(java11))),
        )
        .await;
        assert_eq!(
            manager
                .java_mismatch_before_launch(id)
                .await
                .unwrap()
                .map(|asked| asked.fix_major),
            Some(17)
        );

        manager.fix_java_mismatch(id).await.unwrap();

        let details = manager.instance_details(id).await.unwrap();
        assert!(
            details.java_override.is_none(),
            "{:?}",
            details.java_override
        );
        assert_eq!(details.java_floor, Some(17));
        assert_eq!(
            details.resolved_java.unwrap().resolution.path.as_deref(),
            Some(java17.as_str())
        );
    }

    /// "Use Java N and play" launches too, so it warns about low memory as
    /// PLAY does, after the fix: "Play anyway" then plays the fixed Java.
    #[tokio::test]
    async fn the_fix_before_launch_still_warns_about_low_memory() {
        use crate::managers::{
            instance::run::InsufficientMemoryError,
            java::test_support::{add_instance, add_other, test_app},
        };

        let app = test_app(vec![]).await;
        sign_in_offline(&app.app).await;
        *app.system_info_manager()
            .available_ram_for_test
            .lock()
            .unwrap() = Some(1024 * 1024 * 1024);
        let l17 = add_other(&app.app, "l17", "local", "/opt/jdk17/bin/java", "17.0.12").await;
        let manager = app.instance_manager();
        let id = add_instance(
            &app.app,
            "Survival",
            "1.21.4",
            false,
            Some(JavaOverride::Path(Some(l17.path))),
        )
        .await;
        // What asked loaded the version data, as the library's tiles do.
        assert!(
            manager
                .instance_details(id)
                .await
                .unwrap()
                .java_override_mismatch
        );

        let error = manager.fix_java_mismatch_and_launch(id).await.unwrap_err();

        assert!(
            error.downcast_ref::<InsufficientMemoryError>().is_some(),
            "{error:#}"
        );
        let details = manager.instance_details(id).await.unwrap();
        assert!(
            details.java_override.is_none(),
            "{:?}",
            details.java_override
        );
        assert!(!manager.is_launch_active(id).await.unwrap());
    }

    /// The same for the crash fix's "Use Java N and play".
    #[cfg(unix)]
    #[tokio::test]
    async fn the_crash_fix_still_warns_about_low_memory() {
        use crate::managers::{
            instance::run::InsufficientMemoryError,
            java::test_support::{add_instance, test_app},
        };

        let app = test_app(vec![]).await;
        sign_in_offline(&app.app).await;
        *app.system_info_manager()
            .available_ram_for_test
            .lock()
            .unwrap() = Some(1024 * 1024 * 1024);
        add_runnable(&app.app, "m21", "managed", "21.0.5").await;
        let manager = app.instance_manager();
        let id = add_instance(&app.app, "Crashy", "1.20.4", false, None).await;
        manager.set_java_issue(id, Some(CRASH_ON_17)).await.unwrap();

        let error = manager.fix_java_and_launch(id).await.unwrap_err();

        assert!(
            error.downcast_ref::<InsufficientMemoryError>().is_some(),
            "{error:#}"
        );
        assert_eq!(
            manager.instance_details(id).await.unwrap().java_floor,
            Some(21)
        );
        assert!(!manager.is_launch_active(id).await.unwrap());
    }

    /// Offline, PLAY must not wait on Minecraft's manifest: the check before
    /// a launch reads the version data it has, however old.
    #[tokio::test]
    async fn the_check_before_launch_never_fetches_the_version_data() {
        use crate::managers::java::{
            minecraft_majors::age_version_profiles_for_test,
            test_support::{TEST_VERSIONS, add_instance, add_other, manifest_of, test_app},
        };

        let app = test_app(vec![]).await;
        let l17 = add_other(&app.app, "l17", "local", "/opt/jdk17/bin/java", "17.0.12").await;
        let manager = app.instance_manager();
        let id = add_instance(
            &app.app,
            "Survival",
            "1.21.4",
            false,
            Some(JavaOverride::Path(Some(l17.path))),
        )
        .await;
        // Loads the version data: 1.21.4 needs Java 21.
        assert!(
            manager
                .instance_details(id)
                .await
                .unwrap()
                .java_override_mismatch
        );

        // Old now, and a fetch would say 1.21.4 runs on Java 17.
        age_version_profiles_for_test(&app.app).await;
        let refetched: Vec<_> = TEST_VERSIONS
            .iter()
            .map(|&(id, ty, profile)| match id {
                "1.21.4" => (id, ty, "java-runtime-gamma"),
                _ => (id, ty, profile),
            })
            .collect();
        *app.minecraft_manager().manifest_for_test.lock().unwrap() =
            Some(Ok(manifest_of(&refetched)));

        let asked = manager.java_mismatch_before_launch(id).await.unwrap();
        assert_eq!(asked.map(|asked| asked.fix_major), Some(21));
    }

    async fn sign_in_offline(app: &App) {
        carbon_repos::repos::account::insert_account_offline(
            &app.db,
            "offline-uuid",
            "Player",
            carbon_repos::dbtypes::DbDateTime(chrono::Utc::now().into()),
            None,
        )
        .await
        .unwrap();
        app.account_manager()
            .set_active_uuid(Some("offline-uuid".into()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn no_fix_while_the_game_is_still_running() {
        use crate::managers::instance::run::LaunchState;
        use crate::managers::java::test_support::{add_instance, test_app};

        let app = test_app(vec![]).await;
        let manager = app.instance_manager();
        let id = add_instance(&app.app, "Crashy", "1.20.4", false, None).await;
        manager.set_java_issue(id, Some(CRASH_ON_17)).await.unwrap();
        manager
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .type_
            .data_mut()
            .unwrap()
            .state = LaunchState::Queued(crate::domain::vtask::VisualTaskId(1));

        let error = manager.fix_java_issue(id).await.unwrap_err();
        assert_eq!(error.to_string(), "Close the game first");
    }

    #[tokio::test]
    async fn nothing_to_fix_without_an_issue() {
        use crate::managers::java::test_support::{add_instance, test_app};

        let app = test_app(vec![]).await;
        let id = add_instance(&app.app, "Fine", "1.20.4", false, None).await;

        assert!(app.instance_manager().fix_java_issue(id).await.is_err());
    }
}
