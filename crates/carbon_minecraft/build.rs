use std::path::PathBuf;

fn main() {
    let root_project_path = env!("CARGO_MANIFEST_DIR", "unable to retrieve project root path!");
    if cfg!(test) {
        let test_build_output_path = env!("CARGO_TARGET_TMPDIR", "unable to get CARGO_TARGET_TMPDIR !!!");
        let origin_test_assets_folder = &PathBuf::from(test_build_output_path).join("test_assets");
        let destination_test_assets_folder = &PathBuf::from(root_project_path).join("test_assets");
        eprintln!("copying test assets folder from {origin_test_assets_folder} to {destination_test_assets_folder} ...");
        std::fs::remove_dir(destination_test_assets_folder).expect("unable to remove folder");
        std::fs::copy(origin_test_assets_folder, destination_test_assets_folder).expect("unable to copy folder");
        eprintln!("test assets folder successfully copied from {origin_test_assets_folder} to {destination_test_assets_folder} ...");
    }
}