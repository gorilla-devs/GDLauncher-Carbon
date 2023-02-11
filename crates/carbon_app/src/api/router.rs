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
        let mut router = ::rspc::Router::<$crate::managers::Managers>::new();
        $(
            router = router.$type($endpoint.local, |t| {
                t(|$app: $crate::managers::Managers, $args: $args_ty| async move { $block })
            });
        )*
        router
    }}
}

/// Macro that perform a generic conversion towards rspc::Error,
/// error that have to be mapped MUST impl Debug trait
///
/// # Examples
///
/// ```ignore
///
///     fn main(){
///         let to_parse = "abcd";
///         let rspc_error = to_parse.parse().map_err(|error| {
///             rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
///         }).unwrap_err();
///
///         assert_eq!(rspc_error, try_in_router!(to_parse.parse()).unwrap_err())
///
///     }
/// ``
macro_rules! try_in_router {
    ($result:expr) => {
        $result.map_err(|error| {
            rspc::Error::new(rspc::ErrorCode::InternalServerError, format!("{:?}", error))
        })
    };
}

pub(crate) use router;
pub(crate) use try_in_router;
