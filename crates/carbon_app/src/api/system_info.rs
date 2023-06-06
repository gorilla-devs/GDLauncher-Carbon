use crate::{
    api::{
        keys::systeminfo::{GET_TOTAL_RAM, GET_USED_RAM},
        router::router,
    },
    managers::App,
};
use rspc::RouterBuilderLike;

pub(super) fn mount() -> impl RouterBuilderLike<App, Meta = ()> {
    router! {
        query GET_TOTAL_RAM[app, _args: ()] {
            Ok(app.system_info_manager().get_total_ram().await.to_string())
        }

        query GET_USED_RAM[app, _args: ()] {
            Ok(app.system_info_manager().get_used_ram().await.to_string())
        }
    }
}
