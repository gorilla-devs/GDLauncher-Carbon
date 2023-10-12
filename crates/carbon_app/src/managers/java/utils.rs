use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::{info, trace};

#[cfg(target_os = "windows")]
pub static PATH_SEPARATOR: &str = ";";

#[cfg(not(target_os = "windows"))]
pub static PATH_SEPARATOR: &str = ":";

const JAVA_CHECK_APP: &[u8; 1013] = include_bytes!("JavaCheck.class");
pub const JAVA_CHECK_APP_NAME: &str = "JavaCheck.class";

pub async fn locate_java_check_class() -> anyhow::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let java_check_path = temp_dir.join(JAVA_CHECK_APP_NAME);

    trace!(
        "Checking if JavaCheck is already present in {}",
        temp_dir.display()
    );

    if !java_check_path.exists() {
        trace!("Java Check Path does not exist, writing to disk");
        let mut file = tokio::fs::File::create(&java_check_path).await?;

        file.write_all(JAVA_CHECK_APP).await?;
    }

    info!("JavaCheck located at {}", java_check_path.display());

    Ok(java_check_path)
}
