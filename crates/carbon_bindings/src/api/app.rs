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
            t(|_ctx: GlobalContext, _args: ()| async move {
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                Ok(time as u32)
            })
        })
        .mutation("setTheme", |t| {
            t(|ctx: GlobalContext, v: String| async move {
                ctx.read().await.invalidate("app.getTheme", None);
            })
        })
}
