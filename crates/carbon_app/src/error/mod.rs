pub mod request;

use serde::Serialize;

#[derive(Serialize)]
struct FeError {
    cause: Vec<CauseSegment>,
    backtrace: String,
}

#[derive(Serialize)]
struct CauseSegment {
    display: String,
    debug: String,
}

pub fn anyhow_into_fe_error(error: anyhow::Error) -> rspc::Error {
    let error = FeError {
        cause: error
            .chain()
            .map(|entry| CauseSegment {
                display: format!("{entry}"),
                debug: format!("{entry:#?}"),
            })
            .collect(),
        backtrace: format!("{}", error.backtrace()),
    };

    rspc::Error::new(
        rspc::ErrorCode::InternalServerError,
        serde_json::to_string_pretty(&error).expect("could not convert error to json"),
    )
}

pub fn anyhow_into_rspc_error(error: anyhow::Error) -> rspc::Error {
    rspc::Error::new(
        rspc::ErrorCode::InternalServerError,
        serde_json::to_string_pretty(&anyhow_into_fe_error(error))
            .expect("could not convert error to json"),
    )
}

type AxumError = (axum::http::StatusCode, String);

pub fn anyhow_into_axum_error(error: anyhow::Error) -> AxumError {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        serde_json::to_string_pretty(&anyhow_into_fe_error(error))
            .expect("could not convert error to json"),
    )
}

pub fn anyhow_into_rspc(value: anyhow::Error) -> rspc::Error {
    rspc::Error::new(
        rspc::ErrorCode::InternalServerError,
        format!("backend error: {value:#?}"),
    )
}
