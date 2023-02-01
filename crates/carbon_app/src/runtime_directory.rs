use std::path::PathBuf;

use tokio::io::AsyncReadExt;

pub(crate) async fn set_runtime_directory_override() {
    let path = directories::ProjectDirs::from("com", "gorilladevs", "gdlauncher_carbon")
        .unwrap()
        .data_dir()
        .to_path_buf()
        .join("gdl_store")
        .join("runtime_directory_override.txt");

    if path.exists() {
        let mut file = tokio::fs::File::open(path).await.unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).await.unwrap();
        let path = PathBuf::from(contents.trim());

        if !path.exists() {
            tokio::fs::create_dir_all(&path).await.unwrap();
        }
        std::env::set_current_dir(path).unwrap();
    } else {
        let path = path.parent().unwrap();

        // open finder to this directory
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(path)
                .output()
                .expect("failed to open finder");
        }

        tokio::fs::create_dir_all(path).await.unwrap();
        std::env::set_current_dir(path).unwrap();
    }
}
