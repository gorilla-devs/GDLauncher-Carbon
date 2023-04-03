use std::path::PathBuf;

use anyhow::bail;
use tokio::process::Command;

use crate::domain::java::{JavaComponent, JavaComponentType};

use super::{
    parser::{parse_cmd_output_java_arch, parse_cmd_output_java_version},
    utils::{locate_java_check_class, JAVA_CHECK_APP_NAME},
};

#[async_trait::async_trait]
pub trait JavaChecker {
    async fn get_bin_info(
        &self,
        path: &PathBuf,
        _type: JavaComponentType,
    ) -> anyhow::Result<JavaComponent>;
}

pub struct RealJavaChecker;

#[async_trait::async_trait]
impl JavaChecker for RealJavaChecker {
    async fn get_bin_info(
        &self,
        java_bin_path: &PathBuf,
        _type: JavaComponentType,
    ) -> anyhow::Result<JavaComponent> {
        let java_checker_path = locate_java_check_class().await?;
        if java_bin_path.to_string_lossy() != "java" && !java_bin_path.exists() {
            bail!(
                "Java binary not found at {}",
                java_bin_path.to_string_lossy()
            );
        }

        // Run java
        let output = Command::new(java_bin_path)
            .current_dir(java_checker_path.parent().expect("This should never fail"))
            .arg(
                JAVA_CHECK_APP_NAME
                    .strip_suffix(".class")
                    .expect("This should never fail"),
            )
            .output()
            .await?;

        let output = String::from_utf8(output.stdout)?;
        let java_version = parse_cmd_output_java_version(&output)?;
        let java_arch = parse_cmd_output_java_arch(&output)?;

        Ok(JavaComponent {
            path: java_bin_path.to_string_lossy().to_string(),
            version: java_version,
            arch: java_arch,
            _type,
        })
    }
}
