use std::{env, path::PathBuf};

pub(crate) async fn get_runtime_path_override() -> PathBuf {
    let path: Option<PathBuf>;
    #[cfg(debug_assertions)]
    {
        path = Some(PathBuf::from(env!("RUNTIME_PATH")));
    }
    #[cfg(not(debug_assertions))]
    {
        let mut args = env::args();
        while let Some(arg) = args.next() {
            if arg == "--runtime-path" {
                if let Some(_path) = args.next() {
                    path = Some(PathBuf::from(_path));
                    break;
                }
            }
        }
    }

    path.expect("Runtime path not found").join("data")
}
