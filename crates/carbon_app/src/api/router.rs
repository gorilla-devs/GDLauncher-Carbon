macro_rules! router {
    {$($type:ident $endpoint:path [$($args:tt)*] $block:block)*} => {{
        let mut router = ::rspc::Router::<$crate::app::GlobalContext>::new();
        $($crate::api::router::route_boilerplate! { $type(router, $endpoint [$($args)*]) $block })*
        router
    }}
}

macro_rules! route_boilerplate {
    ($func:ident ($router:expr, $endpoint:path [$app:ident, $args:ident: $args_ty:ty]) $block:block) => {
        $router = $router.query($endpoint.local, |t| {
            t(|$app: $crate::app::GlobalContext, $args: $args_ty| async move { $block })
        });
    };
}

pub(crate) use route_boilerplate;
pub(crate) use router;
