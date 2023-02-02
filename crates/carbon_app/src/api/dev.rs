use crate::app::AppContainer;
use crate::try_in_router;
use env_logger::Builder;
use rspc::{ErrorCode, Router, RouterBuilderLike};

pub(super) fn mount() -> impl RouterBuilderLike<AppContainer> {
    Router::new().mutation("setLogLevel", |t| {
        t(|_: AppContainer, new_log_level: String| {
            let new_log_level = try_in_router!(new_log_level.as_str().parse())?;
            Builder::new().filter_level(new_log_level).init();
            Ok(())
        })
    })
}
