use std::{env, fs};

pub(crate) async fn get_base_api_override() -> String {
    #[allow(unused_assignments)]
    let mut base_api: String = String::from(env!("BASE_API"));

    let mut args = env::args();
    while let Some(arg) = args.next() {
        if arg == "--base_api" {
            if let Some(_base_api) = args.next() {
                base_api = String::from(_base_api);
                break;
            }
        }
    }

    base_api
}
