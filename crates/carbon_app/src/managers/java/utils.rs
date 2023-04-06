use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[cfg(target_os = "windows")]
pub static PATH_SEPARATOR: &str = ";";

#[cfg(not(target_os = "windows"))]
pub static PATH_SEPARATOR: &str = ":";

const JAVA_CHECK_APP: &[u8; 1013] = include_bytes!("JavaCheck.class");
pub const JAVA_CHECK_APP_NAME: &str = "JavaCheck.class";

pub async fn locate_java_check_class() -> anyhow::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let java_check_path = temp_dir.join(JAVA_CHECK_APP_NAME);
    if !java_check_path.exists() {
        let mut file = tokio::fs::File::create(&java_check_path).await?;

        file.write_all(JAVA_CHECK_APP).await?;
    }

    Ok(java_check_path)
}

#[cfg(test)]
mod test {
    use crate::managers::java::utils::{locate_java_check_class, JAVA_CHECK_APP_NAME};

    #[tokio::test]
    async fn test_locate_java_check_class_and_execute() {
        let temp_dir = std::env::temp_dir();
        let java_check_path_env = temp_dir.join(JAVA_CHECK_APP_NAME);
        let _ = std::fs::remove_file(&java_check_path_env);

        let java_check_path = locate_java_check_class().await.unwrap();
        assert!(
            java_check_path == java_check_path_env,
            "Java check path is unexpected"
        );
        assert!(java_check_path.exists(), "Java check path does not exist");

        let proc = tokio::process::Command::new("java")
            .current_dir(temp_dir)
            .arg(JAVA_CHECK_APP_NAME.strip_suffix(".class").unwrap())
            .output()
            .await
            .unwrap();

        assert!(
            proc.status.code() == Some(0),
            "Java check exit code is not 0"
        );
        let _ = std::fs::remove_file(&java_check_path_env);
    }
}
