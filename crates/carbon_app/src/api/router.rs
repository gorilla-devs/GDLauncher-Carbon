/// Creates an RSPC router.
///
/// Router entries use the form
///
/// `<rspc function name> <endpoint key>[<app>, <args>: <args type (optional)>] { ... }`
///
/// # Notes
/// See [keys](crate::api::keys) for endpoint keys
///
/// # Examples
///
/// ```ignore
/// router! {
///     query QUERY_KEY[app, args: MyArgsType] {
///         todo!()
///     }
///
///     mutation MUTATION_KEY[app, args: MyArgsType] {
///         todo!()
///     }
/// }
/// ```
macro_rules! router {
    {$($type:ident $endpoint:path [$app:tt, $args:tt: $args_ty:ty] $block:block)*} => {{
        let mut router = ::rspc::Router::<$crate::managers::App>::new();
        $(
            router = router.$type($endpoint.local, |t| {
                t(|$app: $crate::managers::App, $args: $args_ty| async move {
                    let mut block: ::anyhow::Result::<_> = (|| async move { $block })().await;
                    block = ::anyhow::Context::context(block, $crate::api::router::Endpoint($endpoint.full));
                    block.map_err($crate::error::anyhow_into_rspc_error)
                })
            });
        )*
        router
    }}
}

use derive_more::Display;
pub(crate) use router;

#[derive(Display)]
#[display(fmt = "endpoint {_0}")]
pub struct Endpoint(pub &'static str);
