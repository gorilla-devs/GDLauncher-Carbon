use crate::{api::router::router, managers::App};
use rspc::{RouterBuilderLike, Type};

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query TEST_QUERY[app, _args: ()] {
            let response = app.modplatforms_manager();
            response.some_api_request().await?;

            Ok(())
        }
    }
}
