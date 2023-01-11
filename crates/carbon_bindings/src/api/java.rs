use rspc::{Router, RouterBuilder, RouterBuilderLike};

use super::Ctx;

pub(super) fn mount() -> impl RouterBuilderLike<()> {
    Router::new().query("getAvailableJavas", |t| {
        t(|ctx, _args: ()| async move {
            // let javas = ctx.java_manager.get_available_javas().await?;
            Ok(())
        })
    })
}
