use std::error::Error;
pub mod request;

macro_rules! define_single_error {
    ($name:ident::$variant:ident($error:path)) => {
        #[derive(Error, Debug)]
        pub enum $name {
            #[error("{0}")]
            $variant(#[from] $error),
        }
    };
}

pub(crate) use define_single_error;

pub fn into_rspc<E: Error>(err: E) -> rspc::Error {
    rspc::Error::new(
        rspc::ErrorCode::InternalServerError,
        String::from("backend error: {e:#?}"),
    )
}
