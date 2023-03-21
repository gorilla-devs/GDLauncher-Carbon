pub mod request;

use serde::Serialize;

pub fn anyhow_into_fe_error(error: anyhow::Error) -> rspc::Error {
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
