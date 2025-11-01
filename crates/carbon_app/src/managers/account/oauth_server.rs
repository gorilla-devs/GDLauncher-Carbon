use anyhow::{Result, bail};
use axum::{
    Router,
    extract::{Query, State},
    response::{Html, IntoResponse},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{RwLock, oneshot},
    time::{Duration, timeout},
};
use tracing::{debug, error, info, warn};

/// OAuth callback server that runs temporarily to receive the authorization code
/// from Microsoft's OAuth flow. Implements RFC 8252 (OAuth 2.0 for Native Apps)
/// Section 7.3 - Loopback Interface Redirection.
pub struct OAuthCallbackServer {
    /// The authorization code received from the OAuth callback
    code: Arc<RwLock<Option<String>>>,
    /// Sender to signal when the code has been received
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl OAuthCallbackServer {
    /// Creates a new OAuth callback server
    pub fn new() -> Self {
        Self {
            code: Arc::new(RwLock::new(None)),
            shutdown_tx: None,
        }
    }

    /// Start the callback server and return the port it's listening on
    ///
    /// Returns: (server_task, local_address)
    ///
    /// The server will automatically shut down after receiving the authorization code
    /// or when the returned OAuthCallbackHandle is dropped.
    pub async fn start(mut self) -> Result<OAuthCallbackHandle> {
        // Try to bind to high-numbered ports to avoid conflicts with common services
        // These ports are in the ephemeral range (49152-65535) and unlikely to be in use
        // This ensures the redirect URI is predictable for OAuth providers like Microsoft
        const PREFERRED_PORTS: &[u16] = &[52847, 53829, 54911, 51743, 50127];

        let mut listener = None;
        let mut last_error = None;

        for &port in PREFERRED_PORTS {
            match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                Ok(l) => {
                    listener = Some(l);
                    break;
                }
                Err(e) => {
                    warn!("Failed to bind to port {}: {}", port, e);
                    last_error = Some(e);
                }
            }
        }

        let listener = listener.ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to bind OAuth server to any preferred port. Last error: {:?}",
                last_error
            )
        })?;

        let local_addr = listener.local_addr()?;

        info!("OAuth callback server listening on {}", local_addr);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let code_clone = self.code.clone();

        // Build the Axum router
        let app = Router::new()
            .route("/auth", get(oauth_callback))
            .with_state(code_clone);

        // Spawn the server task
        let server_task = tokio::spawn(async move {
            let server = axum::serve(listener, app);

            // Graceful shutdown when signal received
            tokio::select! {
                result = server => {
                    if let Err(e) = result {
                        error!("OAuth callback server error: {}", e);
                    }
                }
                _ = shutdown_rx => {
                    debug!("OAuth callback server shutting down");
                }
            }
        });

        Ok(OAuthCallbackHandle {
            code: self.code,
            local_addr,
            server_task,
            _shutdown_tx: self.shutdown_tx,
        })
    }
}

/// Handle to the running OAuth callback server
pub struct OAuthCallbackHandle {
    code: Arc<RwLock<Option<String>>>,
    local_addr: SocketAddr,
    server_task: tokio::task::JoinHandle<()>,
    _shutdown_tx: Option<oneshot::Sender<()>>,
}

impl OAuthCallbackHandle {
    /// Get the local address the server is listening on
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get the port the server is listening on
    pub fn port(&self) -> u16 {
        self.local_addr.port()
    }

    /// Get the redirect URI for Microsoft OAuth
    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/auth", self.port())
    }

    /// Wait for the authorization code to be received
    ///
    /// Returns Ok(code) if received within timeout, Err otherwise
    pub async fn wait_for_code(&self, timeout_duration: Duration) -> Result<String> {
        let result = timeout(timeout_duration, async {
            // Poll for code every 500ms
            loop {
                {
                    let code_lock = self.code.read().await;
                    if let Some(code) = code_lock.as_ref() {
                        return Ok(code.clone());
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await;

        match result {
            Ok(Ok(code)) => {
                info!("Received OAuth authorization code");
                Ok(code)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                warn!("OAuth callback timeout - no code received");
                bail!("Timeout waiting for OAuth callback")
            }
        }
    }

    /// Shutdown the server
    pub async fn shutdown(mut self) {
        // Send shutdown signal to gracefully stop the server
        if let Some(tx) = self._shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.server_task.await;
        debug!("OAuth callback server stopped");
    }
}

/// Query parameters from OAuth callback
#[derive(Debug, Deserialize)]
struct OAuthCallbackQuery {
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
    state: Option<String>,
}

/// OAuth callback handler - receives the authorization code from Microsoft
async fn oauth_callback(
    Query(params): Query<OAuthCallbackQuery>,
    State(code_storage): State<Arc<RwLock<Option<String>>>>,
) -> impl IntoResponse {
    debug!("OAuth callback received: {:?}", params);

    // Check for error in callback
    if let Some(error) = params.error {
        let description = params
            .error_description
            .unwrap_or_else(|| "Unknown error".to_string());
        error!("OAuth callback error: {} - {}", error, description);

        return Html(error_page(&error, &description));
    }

    // Extract authorization code
    if let Some(code) = params.code {
        // Store the code
        *code_storage.write().await = Some(code);

        info!("OAuth authorization code received successfully");
        Html(success_page())
    } else {
        warn!("OAuth callback received without code or error");
        Html(error_page(
            "invalid_request",
            "No authorization code received",
        ))
    }
}

/// HTML page shown on successful authentication
fn success_page() -> String {
    include_str!("oauth_success.html").to_string()
}

/// HTML page shown on authentication error
fn error_page(error: &str, description: &str) -> String {
    include_str!("oauth_error.html")
        .replace("{ERROR_TYPE}", error)
        .replace("{ERROR_DESCRIPTION}", description)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_starts_and_stops() {
        let server = OAuthCallbackServer::new();
        let handle = server.start().await.expect("Server should start");

        assert!(handle.port() > 0);
        assert_eq!(handle.local_addr().ip().to_string(), "127.0.0.1");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_timeout_when_no_code_received() {
        let server = OAuthCallbackServer::new();
        let handle = server.start().await.expect("Server should start");

        let result = handle.wait_for_code(Duration::from_millis(100)).await;
        assert!(result.is_err());

        handle.shutdown().await;
    }
}
