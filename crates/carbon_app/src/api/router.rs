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

                    let span = ::tracing::info_span!($endpoint.span_key);
                    let block: ::core::result::Result::<_, $crate::api::router::router_rt_helper!($($rtmarker)?)>
                        = ::tracing::Instrument::instrument(async move {
                            ::tracing::trace!("Running endpoint");
                            $block
                        }, span).await;

                    block.map_err(|e| {
                        let mut e = ::core::convert::Into::<$crate::error::FeError>::into(e);
                        e.extend($crate::error::CauseSegment::from_display($crate::api::router::Endpoint($endpoint.full)));
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

use derive_more::Display;
pub(crate) use router;
pub(crate) use router_rt_helper;

#[derive(Display)]
#[display(fmt = "endpoint {_0}")]
pub struct Endpoint(pub &'static str);
