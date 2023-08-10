use std::{env, path::PathBuf};

pub(crate) async fn get_runtime_path_override() -> PathBuf {
    let mut path: Option<PathBuf> = None;
    #[cfg(debug_assertions)]
    {
        path = Some(PathBuf::from(env!("RUNTIME_PATH")));
    }
    #[cfg(not(debug_assertions))]
    {
        let mut args = env::args();

        for n in 0.. {
            let arg = args.nth(n);
            if arg.is_none() {
                break;
            }
            let arg = arg.unwrap();
            if arg == "--runtime_path" {
                let path_str = args.nth(n).unwrap();
                path = Some(PathBuf::from(path_str));
                break;
            }
        }
    }

    path.expect("Runtime path not found").join("data")
}
