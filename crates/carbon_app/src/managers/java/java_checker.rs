use std::path::Path;

use anyhow::bail;
use tokio::process::Command;
use tracing::instrument;

use crate::domain::java::{
    JavaArch, JavaComponent, JavaComponentType, JavaOs, JavaVersion,
};

use super::{
    parser::parse_cmd_output_java,
    utils::{locate_java_check_class, JAVA_CHECK_APP_NAME},
};

#[async_trait::async_trait]
pub trait JavaChecker {
    async fn get_bin_info(
        &self,
        path: &Path,
        _type: JavaComponentType,
    ) -> anyhow::Result<JavaComponent>;
}

pub struct RealJavaChecker;

#[async_trait::async_trait]
impl JavaChecker for RealJavaChecker {
    #[instrument(skip(self))]
    async fn get_bin_info(
        &self,
        java_bin_path: &Path,
        _type: JavaComponentType,
    ) -> anyhow::Result<JavaComponent> {
        let java_checker_path = locate_java_check_class().await?;
        if java_bin_path.to_string_lossy() != "java" && !java_bin_path.exists()
        {
            bail!(
                "Java binary not found at {}",
                java_bin_path.to_string_lossy()
            );
        }

        // Run java
        let output_cmd = Command::new(java_bin_path)
            .arg("-cp")
            .arg(java_checker_path.parent().expect("This should never fail"))
            .arg(
                JAVA_CHECK_APP_NAME
                    .strip_suffix(".class")
                    .expect("This should never fail"),
            )
            .output()
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "Cannot execute java checker command on {:?} - {}",
                    java_bin_path,
                    err
                )
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
