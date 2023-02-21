use std::path::PathBuf;

use tokio::io::AsyncReadExt;

pub(crate) async fn get_runtime_path_override() -> PathBuf {
    let path = directories::ProjectDirs::from("com", "gorilladevs", "gdlauncher_carbon")
        .unwrap()
        .data_dir()
        .to_path_buf()
        .join("runtime_path_override.txt");

    if path.exists() {
        let mut file = tokio::fs::File::open(&path).await.unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).await.unwrap();
        let path = PathBuf::from(contents.trim());

        if !path.exists() {
            tokio::fs::create_dir_all(&path).await.unwrap();
        }
    } else {
        let path = path.parent().unwrap();

        // open finder to this directory
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(&path)
                .output()
                .expect("failed to open finder");
        }

        tokio::fs::create_dir_all(&path).await.unwrap();
    }

    path
}
