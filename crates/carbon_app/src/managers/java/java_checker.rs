use std::path::Path;

use anyhow::bail;
use tokio::process::Command;
use tracing::instrument;

use crate::domain::java::{JavaArch, JavaComponent, JavaComponentType, JavaOs, JavaVersion};

use super::{
    parser::parse_cmd_output_java,
    utils::{JAVA_CHECK_APP_NAME, locate_java_check_class},
};

#[async_trait::async_trait]
pub trait JavaChecker {
    fn cancellation(&self) -> Option<tokio_util::sync::CancellationToken> {
        None
    }
    async fn get_bin_info(
        &self,
        path: &Path,
        _type: JavaComponentType,
    ) -> anyhow::Result<JavaComponent>;
}

#[derive(Debug, thiserror::Error)]
#[error("Java validation inconclusive: {0}")]
pub struct JavaValidationInconclusive(pub &'static str);

/// Cancellation/refusal, unconfirmed supervision and the launcher's own
/// bookkeeping failing (the probe's process record could not be written) do
/// not establish unusable Java: only the Java process itself can. Preserve
/// typed causes through context; never infer this from prose.
pub fn validation_inconclusive(error: &anyhow::Error) -> bool {
    error.downcast_ref::<JavaValidationInconclusive>().is_some()
        || crate::managers::runtime_lifecycle::child_settlement_unknown(error)
        || crate::managers::runtime_lifecycle::child_bookkeeping_failed(error)
        || error.chain().any(|cause| {
            cause.is::<JavaValidationInconclusive>()
                || cause
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::Interrupted)
        })
}

pub(super) fn admit_probe(
    lifecycle: &std::sync::Arc<crate::managers::runtime_lifecycle::RuntimeLifecycle>,
) -> anyhow::Result<crate::managers::runtime_lifecycle::RuntimeActivity> {
    lifecycle
        .admit_owned(
            "Java probe operation",
            crate::managers::runtime_lifecycle::ActivityPolicy::JavaProbe,
        )
        .map_err(|error| error.context(JavaValidationInconclusive("probe admission refused")))
}

pub struct RealJavaChecker {
    lifecycle: std::sync::Arc<crate::managers::runtime_lifecycle::RuntimeLifecycle>,
    cancel: tokio_util::sync::CancellationToken,
}
impl RealJavaChecker {
    pub fn with_cancel(
        lifecycle: std::sync::Arc<crate::managers::runtime_lifecycle::RuntimeLifecycle>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self { lifecycle, cancel }
    }
    pub fn new(
        lifecycle: std::sync::Arc<crate::managers::runtime_lifecycle::RuntimeLifecycle>,
    ) -> Self {
        Self {
            lifecycle,
            cancel: tokio_util::sync::CancellationToken::new(),
        }
    }
}

#[async_trait::async_trait]
impl JavaChecker for RealJavaChecker {
    fn cancellation(&self) -> Option<tokio_util::sync::CancellationToken> {
        Some(self.cancel.clone())
    }
    #[instrument(skip(self))]
    async fn get_bin_info(
        &self,
        java_bin_path: &Path,
        _type: JavaComponentType,
    ) -> anyhow::Result<JavaComponent> {
        let activity = admit_probe(&self.lifecycle)?;
        let lifecycle = self.lifecycle.clone();
        let parent_cancel = self.cancel.clone();
        let java_bin_path = java_bin_path.to_owned();
        crate::managers::runtime_lifecycle::spawn_retained(activity, move |slot| {
            Box::pin(async move {
                let activity = slot.as_ref().unwrap();
                if activity.cancellation().is_cancelled() || parent_cancel.is_cancelled() {
                    return Err(
                        JavaValidationInconclusive("probe cancelled before validation").into(),
                    );
                }
                let java_checker_path = locate_java_check_class().await.map_err(|error| {
                    error.context(JavaValidationInconclusive("checker publication failed"))
                })?;
                if activity.cancellation().is_cancelled() || parent_cancel.is_cancelled() {
                    return Err(JavaValidationInconclusive("probe cancelled before spawn").into());
                }
                if java_bin_path.to_string_lossy() != "java" && !java_bin_path.exists() {
                    bail!(
                        "Java binary not found at {}",
                        java_bin_path.to_string_lossy()
                    );
                }

                // Run java
                let output_cmd = crate::managers::runtime_lifecycle::tracked_output_reserved(
                    &lifecycle,
                    Command::new(&java_bin_path)
                        .arg("-cp")
                        .arg(java_checker_path.parent().expect("This should never fail"))
                        .arg(
                            JAVA_CHECK_APP_NAME
                                .strip_suffix(".class")
                                .expect("This should never fail"),
                        ),
                    crate::managers::runtime_lifecycle::ActivityPolicy::JavaProbe,
                    None,
                )
                .await
                .map_err(|err| {
                    let error = anyhow::Error::from(err).context(format!(
                        "Cannot execute java checker command on {:?}",
                        java_bin_path
                    ));
                    if crate::managers::runtime_lifecycle::child_bookkeeping_failed(&error) {
                        error.context(JavaValidationInconclusive(
                            "GDLauncher could not record the Java check in its data folder",
                        ))
                    } else if lifecycle.admission_frozen()
                        || activity.cancellation().is_cancelled()
                        || parent_cancel.is_cancelled()
                    {
                        error.context(JavaValidationInconclusive("probe refused or cancelled"))
                    } else {
                        error
                    }
                })?;

                let output = String::from_utf8(output_cmd.stdout)?;
                let error_output = String::from_utf8(output_cmd.stderr)?;

                if !error_output.is_empty() {
                    tracing::warn!(
                        "Java checker command failed on {:?} - {}",
                        java_bin_path,
                        error_output,
                    );
                }

                let parsed_output = match parse_cmd_output_java(&output) {
                    Ok(parsed_output) => parsed_output,
                    Err(err) => {
                        tracing::warn!("Cannot parse java checker output - {}", err);
                        tracing::warn!("Output: {}", output);
                        bail!("Cannot parse java checker output - {}", err);
                    }
                };

                Ok(JavaComponent {
                    path: java_bin_path.to_string_lossy().to_string(),
                    version: parsed_output.version,
                    arch: parsed_output.arch,
                    vendor: parsed_output.vendor,
                    os: JavaOs::try_from(std::env::consts::OS.to_string())?,
                    _type,
                })
            })
        })
        .await
        .map_err(|error| {
            anyhow::Error::new(error)
                .context(JavaValidationInconclusive("probe worker did not return"))
        })?
    }
}

pub struct MockJavaChecker;

#[async_trait::async_trait]
impl JavaChecker for MockJavaChecker {
    async fn get_bin_info(
        &self,
        path: &Path,
        _type: JavaComponentType,
    ) -> anyhow::Result<JavaComponent> {
        Ok(JavaComponent {
            path: path.to_string_lossy().to_string(),
            version: JavaVersion {
                major: 19,
                minor: 0,
                patch: "0".to_string(),
                update_number: None,
                prerelease: None,
                build_metadata: None,
            },
            arch: JavaArch::X86_32,
            _type: JavaComponentType::Local,
            os: JavaOs::Linux,
            vendor: "Azul Systems, Inc.".to_string(),
        })
    }
}

pub struct MockJavaCheckerInvalid;

#[async_trait::async_trait]
impl JavaChecker for MockJavaCheckerInvalid {
    async fn get_bin_info(
        &self,
        _path: &Path,
        _type: JavaComponentType,
    ) -> anyhow::Result<JavaComponent> {
        bail!("Expected failure");
    }
}

#[cfg(test)]
mod bookkeeping_tests {
    use super::*;
    use crate::managers::runtime_lifecycle::RuntimeLifecycle;
    use std::sync::Arc;

    /// A data folder that refuses the probe's process record says nothing
    /// about the Java, so the probe is inconclusive: no caller drops,
    /// invalidates, unlinks or deletes that Java because of it.
    #[tokio::test]
    async fn probe_whose_process_record_cannot_be_written_is_inconclusive() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("__gdl_runtime_writers"), "not a folder").unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        // Any existing file: the probe fails before anything is spawned.
        let java = root.path().join("java");
        std::fs::write(&java, b"never run").unwrap();

        let error = RealJavaChecker::new(lifecycle.clone())
            .get_bin_info(&java, JavaComponentType::Local)
            .await
            .unwrap_err();

        assert!(validation_inconclusive(&error), "{error:#}");
        assert!(
            format!("{error:#}").contains("could not record the Java check"),
            "{error:#}"
        );
        lifecycle.begin_relocation("move").unwrap();
    }

    /// A file the system refuses to run is evidence about that Java, and
    /// stays a conclusive failure.
    #[cfg(unix)]
    #[tokio::test]
    async fn probe_of_a_file_that_cannot_run_is_conclusive() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let java = root.path().join("java");
        std::fs::write(&java, b"not a program").unwrap();

        let error = RealJavaChecker::new(lifecycle)
            .get_bin_info(&java, JavaComponentType::Local)
            .await
            .unwrap_err();

        assert!(!validation_inconclusive(&error), "{error:#}");
    }
}

// #[cfg(test)]
// mod test {
//     #[tokio::test]
//     async fn test_get_bin_info() {
//         use std::path::PathBuf;

//         use crate::domain::java::{JavaArch, JavaComponentType, JavaOs};

//         use super::{JavaChecker, RealJavaChecker};

//         let java_checker = RealJavaChecker;
//         let java_bin_path = PathBuf::from(
//             "\\\\?\\C:\\Program Files\\Eclipse Adoptium\\jdk-17.0.6.10-hotspot\\bin\\java.exe",
//         );

//         let java_component = java_checker
//             .get_bin_info(&java_bin_path, JavaComponentType::Local)
//             .await
//             .unwrap();

//         println!("{:?}", java_component);
//     }
// }
