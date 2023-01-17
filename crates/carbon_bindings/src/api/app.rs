use super::Ctx;
use rspc::{Router, RouterBuilderLike, Type};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Type, Serialize)]
struct Theme {
    name: String,
}

pub(super) fn mount() -> impl RouterBuilderLike<()> {
    Router::new().query("getTheme", |t| {
        t(|ctx: (), _args: ()| async move {
            let theme = Theme {
                name: "default".to_string(),
            };
            Ok(theme)
        })
    })
}
