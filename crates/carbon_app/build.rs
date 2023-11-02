use std::{path::PathBuf, process::Command};
fn main() {
    let parent_env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(".env");

    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed={}", parent_env_path.display());
    println!("cargo:rerun-if-changed=../../packages/config/version.json");

    let git_commit_author_date = Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .output()
        .unwrap();
    let git_commit_author_date =
        String::from_utf8(git_commit_author_date.stdout).unwrap();

    let version_json =
        std::fs::read_to_string("../../packages/config/version.json").unwrap();
    let version_json: serde_json::Value =
        serde_json::from_str(&version_json).unwrap();

    let version = version_json["version"].as_str().unwrap();
    let channel = version_json["channel"]
        .as_str()
        .map(|s| {
            if s.is_empty() {
                "".to_string()
            } else {
                format!("-{}.{}", s, git_commit_author_date)
            }
        })
        .unwrap_or("".to_string());

    println!("cargo:rustc-env=APP_VERSION={version}{channel}");

    if parent_env_path.exists() {
        for file in dotenvy::from_filename_iter(parent_env_path).unwrap() {
            let (key, value) = file.unwrap();
            println!("cargo:rustc-env={key}={value}");
        }
    }
}
