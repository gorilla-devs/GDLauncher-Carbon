use super::GlobalContext;
use rspc::{Router, RouterBuilderLike, Type};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Type, Serialize)]
struct Theme {
    name: String,
}

pub(super) fn mount() -> impl RouterBuilderLike<GlobalContext> {
    Router::new()
        .query("getTheme", |t| {
            t(|ctx: GlobalContext, _args: ()| async move {
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                Ok(time as u32)
            })
        })
        .mutation("setTheme", |t| {
            t(|_, v: String| {
                // invalidate_query!("app.getTheme");
            })
        })
}
