use crate::managers::App;
use axum::{Json, extract::State, http::StatusCode, routing::post};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    request_id: String,
}
fn conflict(error: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::CONFLICT, error.to_string())
}
pub(super) fn mount() -> axum::Router<App> {
    axum::Router::new()
        .route("/prepare-relocation", post(|State(app): State<App>, Json(request): Json<Request>| async move {
            if request.request_id.is_empty() { return Err(conflict(anyhow::anyhow!("Missing request id"))); }
            app.runtime_lifecycle.refresh_adopted();
            app.runtime_lifecycle.begin_relocation(&request.request_id).map_err(conflict)?;
            Ok(Json(serde_json::json!({ "requestId": request.request_id, "runtimePath": app.settings_manager().runtime_path.to_path_buf() })))
        }))
        .route("/cancel-relocation", post(|State(app): State<App>, Json(request): Json<Request>| async move {
            app.runtime_lifecycle.cancel_relocation(&request.request_id).map_err(conflict)?;
            Ok::<_, (StatusCode, String)>(Json(serde_json::json!({ "requestId": request.request_id })))
        }))
        .route("/shutdown", post(|State(app): State<App>, Json(request): Json<Request>| async move {
            // Admission is already frozen and all external owners were absent.
            // No games are killed. Electron must observe actual core exit after this ACK.
            app.runtime_lifecycle.commit_shutdown(&request.request_id).map_err(conflict)?;
            Ok::<_, (StatusCode, String)>(Json(serde_json::json!({ "requestId": request.request_id })))
        }))
}
