use crate::app::AppContainer;
use crate::try_in_router;
use rspc::{ErrorCode, Router, RouterBuilderLike};

pub(super) fn mount() -> impl RouterBuilderLike<AppContainer> {
    Router::new()
        .query("getTheme", |t| {
            t(|app: AppContainer, _args: ()| async move {
                let app = app.read().await;
                let configuration_manager = try_in_router!(app.get_configuration_manager().await)?;
                try_in_router!(configuration_manager.get_theme().await)
            })
        })
        .mutation("setTheme", |t| {
            t(|app: AppContainer, new_theme: String| async move {
                let app = app.read().await;
                let configuration_manager = try_in_router!(app.get_configuration_manager().await)?;
                try_in_router!(configuration_manager.set_theme(new_theme.clone()).await)?;
                app.invalidate("app.getTheme", Some(new_theme.into()));
                Ok(())
            })
        })
}
