use std::path::PathBuf;

use tokio::io::AsyncReadExt;

pub(crate) async fn get_runtime_path_override() -> PathBuf {
    let mut base_path = directories::ProjectDirs::from("com", "gorilladevs", "gdlauncher_carbon")
        .unwrap()
        .data_dir()
        .to_path_buf();

    let override_path = base_path.join("runtime_path_override.txt");

    if override_path.exists() {
        println!("Runtime path override file exists, reading");
        let mut file = tokio::fs::File::open(&override_path).await.unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).await.unwrap();
        let path = PathBuf::from(contents.trim());
        println!("Runtime path override is {}", path.display());

        if !path.exists() {
            println!("Runtime path override does not exist. Creating");
            tokio::fs::create_dir_all(&path).await.unwrap();
        }
        base_path = path;
    } else {
        // open finder to this directory
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(&base_path)
                .output()
                .expect("failed to open finder");
        }

        println!(
            "Runtime path override file does not exist. Using base {}",
            base_path.display()
        );
        tokio::fs::create_dir_all(&base_path).await.unwrap();
    }

    base_path
}
