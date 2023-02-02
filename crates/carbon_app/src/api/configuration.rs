use crate::app::AppContainer;
use crate::{
    into_router_mutation_response, into_router_query_response,
    try_in_router,
};
use rspc::{ErrorCode, Router, RouterBuilderLike};

pub(super) fn mount() -> impl RouterBuilderLike<AppContainer> {
    Router::new()
        .query("getTheme", |t| {
            t(|app: AppContainer, _args: ()| async move {
                into_router_query_response!{
                    {
                        let app = app.read().await;
                        let configuration_manager = try_in_router!(app.get_configuration_manager().await)?;
                        configuration_manager.get_theme().await
                    },
                    String
                }
            })
        })
        .mutation("setTheme", |t| {
            t(|app: AppContainer, new_theme: String| async move {
                into_router_mutation_response!{
                    app,
                    "app.getTheme",
                    String,
                    {
                        let app = app.read().await;
                        let configuration_manager = try_in_router!(app.get_configuration_manager().await)?;
                        configuration_manager.set_theme(new_theme.clone()).await
                    }
                }
            })
        })
}
