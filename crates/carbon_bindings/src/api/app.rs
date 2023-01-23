use super::GlobalContext;
use rspc::{Router, RouterBuilderLike};

pub(super) fn mount() -> impl RouterBuilderLike<GlobalContext> {
    Router::new()
        .query("getTheme", |t| {
            t(|_ctx: GlobalContext, _args: ()| async move { Ok("main") })
        })
        .mutation("setTheme", |t| {
            t(|ctx: GlobalContext, v: String| async move {
                ctx.read().await.invalidate("app.getTheme", None);
            })
        })
}
