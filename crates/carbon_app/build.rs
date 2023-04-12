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
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    let version_json = std::fs::read_to_string("../../packages/config/version.json").unwrap();
    let version_json: serde_json::Value = serde_json::from_str(&version_json).unwrap();

    let version = version_json["version"].as_str().unwrap();
    let channel = version_json["channel"].as_str().unwrap();

    println!("cargo:rustc-env=APP_VERSION={version}{channel}+{git_hash}");

    if parent_env_path.exists() {
        for file in dotenvy::from_filename_iter(parent_env_path).unwrap() {
            let (key, value) = file.unwrap();
            println!("cargo:rustc-env={key}={value}");
        }
    }
}
