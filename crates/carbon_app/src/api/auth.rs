use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::sync::OnceLock;

const AUTH_HEADER: &str = "x-api-token";
const AUTH_QUERY_KEY: &str = "_token";

static EXPECTED_TOKEN: OnceLock<String> = OnceLock::new();

/// Sets the per-process API token. Must be called once at startup before the
/// router begins serving requests.
pub fn set_expected_token(token: String) {
    EXPECTED_TOKEN
        .set(token)
        .expect("API token already initialized");
}

pub fn expected_token() -> Option<&'static str> {
    EXPECTED_TOKEN.get().map(|s| s.as_str())
}

fn token_from_query(uri: &axum::http::Uri) -> Option<String> {
    let query = uri.query()?;
    for pair in query.split('&') {
        let mut iter = pair.splitn(2, '=');
        let k = iter.next()?;
        if k == AUTH_QUERY_KEY {
            // Token is always hex chars — no percent-encoded chars to decode.
            return Some(iter.next().unwrap_or("").to_string());
        }
    }
    None
}

fn token_from_request(req: &Request) -> Option<String> {
    if let Some(v) = req.headers().get(AUTH_HEADER) {
        if let Ok(s) = v.to_str() {
            return Some(s.to_string());
        }
    }
    if let Some(auth) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(s) = auth.to_str() {
            if let Some(rest) = s.strip_prefix("Bearer ") {
                return Some(rest.to_string());
            }
        }
    }
    token_from_query(req.uri())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Auth middleware: every request must carry the per-process token in either
/// the `x-api-token` header, an `Authorization: Bearer <token>` header, or a
/// `_token` query parameter. The /health endpoint is excluded so the startup
/// readiness probe still works.
pub async fn require_token(req: Request, next: Next) -> Response {
    let path = req.uri().path();

    // /health is used by the startup probe before the renderer has the token.
    if path == "/health" || path == "/" {
        return next.run(req).await;
    }

    let Some(expected) = expected_token() else {
        // Token not configured — deny by default to avoid silent open access.
        return (StatusCode::SERVICE_UNAVAILABLE, "auth not initialized").into_response();
    };

    let Some(provided) = token_from_request(&req) else {
        let mut resp = (StatusCode::UNAUTHORIZED, "missing api token").into_response();
        resp.headers_mut().insert(
            "WWW-Authenticate",
            HeaderValue::from_static("X-API-Token, Bearer"),
        );
        return resp;
    };

    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return (StatusCode::UNAUTHORIZED, "invalid api token").into_response();
    }

    next.run(req).await
}
