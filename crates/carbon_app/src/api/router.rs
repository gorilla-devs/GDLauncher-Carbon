// / Creates an RSPC router.
// /
// / Router entries use the form
// /
// / `<rspc function name> <endpoint key>[<app>, <args>: <args type (optional)>] { ... }`
// /
// / # Notes
// / See [keys](crate::api::keys) for endpoint keys
// /
// / # Examples
// /
// / ```ignore
// / router! {
// /     query QUERY_KEY[app, args: MyArgsType] {
// /         todo!()
// /     }
// /
// /     mutation MUTATION_KEY[app, args: MyArgsType] {
// /         todo!()
// /     }
// / }
// / ```
macro_rules! router {
    {$($type:ident $(($rtmarker:tt))? $endpoint:path [$app:tt, $args:tt: $args_ty:ty] $block:block)*} => {{
        let mut router = ::rspc::Router::<$crate::managers::App>::new();
        $(
            router = router.$type($endpoint.local, |t| {
                t(|$app: $crate::managers::App, $args: $args_ty| async move {

                    let started = $crate::api::router::router_mutation_started!($type, $endpoint.full);
                    let span = ::tracing::info_span!($endpoint.span_key);
                    let block: ::core::result::Result::<_, $crate::api::router::router_rt_helper!($($rtmarker)?)>
                        = ::tracing::Instrument::instrument(async move {
                            ::tracing::trace!("Running endpoint {:?} with args: {:?}", $endpoint.full, $args);
                            $block
                        }, span).await;
                    if let ::core::option::Option::Some(started) = started {
                        ::tracing::debug!(
                            endpoint = $endpoint.full,
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            ok = block.is_ok(),
                            "rspc mutation finished"
                        );
                    }

                    block.map_err(|e| {
                        let mut e = ::core::convert::Into::<$crate::error::FeError>::into(e);
                        e.extend($crate::error::CauseSegment::from_display($crate::api::router::Endpoint($endpoint.full)));
                        ::tracing::debug!({ error = ?e }, "Returning error to frontend from {:?}", $endpoint.full);
                        e.make_rspc()
                    })
                })
            });
        )*
        router
    }}
}

macro_rules! router_rt_helper {
    (*) => {
        _
    };
    () => {
        ::anyhow::Error
    };
}

// Logs a mutation's arrival and returns when it started, so the core log shows
// whether a click reached the core and how long it took to answer. Queries are
// polled every few seconds and stay out of the log. Never logs the arguments:
// they can hold account codes, tokens and paths.
macro_rules! router_mutation_started {
    (mutation, $endpoint:expr) => {{
        ::tracing::debug!(endpoint = $endpoint, "rspc mutation started");
        ::core::option::Option::Some(::std::time::Instant::now())
    }};
    ($other:ident, $endpoint:expr) => {
        ::core::option::Option::<::std::time::Instant>::None
    };
}

use derive_more::Display;
pub(crate) use router;
pub(crate) use router_mutation_started;
pub(crate) use router_rt_helper;

#[derive(Display)]
#[display("endpoint {_0}")]
pub struct Endpoint(pub &'static str);
