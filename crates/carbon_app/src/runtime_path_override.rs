use std::{env, fs, path::PathBuf};

pub(crate) async fn get_runtime_path_override() -> PathBuf {
    #[allow(unused_assignments)]
    let mut path: Option<PathBuf> = None;
    #[cfg(debug_assertions)]
    {
        path = Some(PathBuf::from(env!("RUNTIME_PATH")));
    }
    #[cfg(not(debug_assertions))]
    {
        let mut args = env::args();
        while let Some(arg) = args.next() {
            if arg == "--runtime_path" {
                if let Some(_path) = args.next() {
                    path = Some(PathBuf::from(_path));
                    break;
                }
            }
        }
    }

    let data_path = path.expect("Runtime path not found").join("data");

    fs::create_dir_all(&data_path)
        .expect(format!("Failed to create data directory: {:?}", data_path).as_str());

    dunce::canonicalize(data_path).unwrap()
}
