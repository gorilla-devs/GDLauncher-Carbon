use crate::api::managers::App;
use crate::api::router::router;
use rspc::RouterBuilderLike;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        // mutation SEND_EVENT[app, event: FEEvent] {
        //     app.metrics_manager().track_event(event.into()).await?;
        //     Ok(())
        //  }
    }
}
