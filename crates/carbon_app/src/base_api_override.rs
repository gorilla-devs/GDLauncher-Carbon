use crate::util::base_api::get_base_api_env;
use std::{env, fs};

pub(crate) fn get_base_api_override() -> String {
    #[allow(unused_assignments)]
    let mut base_api: String = String::from(get_base_api_env!());

    let mut args = env::args();
    while let Some(arg) = args.next() {
        if arg == "--base_api" {
            if let Some(_base_api) = args.next() {
                base_api = String::from(_base_api);
                break;
            }
        }
    }

    tracing::info!("Using base api: {}", base_api);

    base_api
}
