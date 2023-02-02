use crate::app::AppContainer;
use rspc::{ErrorCode, Router, RouterBuilderLike};

pub(super) fn mount() -> impl RouterBuilderLike<AppContainer> {
    Router::new()
        .query("getTheme", |t| {
            t(|app: AppContainer, _args: ()| async move {
                let app = app.read().await;
                let configuration_manager =
                    app.get_configuration_manager().await.map_err(|error| {
                        rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
                    })?;
                configuration_manager
                    .get_theme()
                    .await
                    .map_err(|error| error.into())
            })
        })
        .mutation("setTheme", |t| {
            t(|app: AppContainer, new_theme: String| async move {
                let app = app.read().await;
                let configuration_manager =
                    app.get_configuration_manager().await.map_err(|error| {
                        rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
                    })?;
                configuration_manager
                    .set_theme(new_theme.clone())
                    .await
                    .map_err(|error| {
                        rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
                    })?;
                app.invalidate("app.getTheme", Some(new_theme.into()));
                Ok(())
            })
        })
}
