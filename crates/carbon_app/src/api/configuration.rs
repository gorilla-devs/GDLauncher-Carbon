use crate::api::keys::app::*;
use crate::api::router::{router, try_in_router};
use crate::managers::Managers;
use rspc::RouterBuilderLike;

pub(super) fn mount() -> impl RouterBuilderLike<Managers> {
    router! {
        query GET_THEME[app, _args: ()] {
            try_in_router!(app.configuration_manager.get_theme().await)
        }

        mutation SET_THEME[app, new_theme: String] {
            try_in_router!(app.configuration_manager.set_theme(new_theme.clone()).await)?;
            Ok(())
        }
    }
}
