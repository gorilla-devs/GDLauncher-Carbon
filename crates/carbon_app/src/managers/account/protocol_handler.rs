use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use url::Url;

/// Protocol handler for OAuth callbacks via gdlauncher:// custom protocol.
///
/// This serves as a fallback mechanism for systems where localhost OAuth
/// redirects don't work properly (e.g., some security software, corporate proxies).
///
/// Expected URL format: `gdlauncher://oauth/callback?code=...&state=...`
///
/// Flow:
/// 1. Microsoft OAuth redirects to gdlauncher://oauth/callback?code=...&state=...
/// 2. OS launches GDLauncher with the protocol URL
/// 3. Electron main process captures the URL via 'open-url' event
/// 4. Frontend calls rspc mutation with the callback data
/// 5. This handler parses and validates the callback
/// 6. Existing enrollment task continues with the authorization code
pub struct ProtocolHandler {
    /// Pending authorization code awaiting consumption
    pending_code: Arc<RwLock<Option<ProtocolOAuthCallback>>>,
}

/// OAuth callback data from protocol URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolOAuthCallback {
    /// Authorization code from Microsoft
    pub code: String,
    /// State parameter for CSRF protection (optional)
    pub state: Option<String>,
    /// Error code if authentication failed
    pub error: Option<String>,
    /// Error description if authentication failed
    pub error_description: Option<String>,
}

impl ProtocolHandler {
    /// Create a new protocol handler
    pub fn new() -> Self {
        Self {
            pending_code: Arc::new(RwLock::new(None)),
        }
    }

    /// Parse and store an OAuth callback from a protocol URL
    ///
    /// Expected URL format: `gdlauncher://oauth/callback?code=...&state=...`
    ///
    /// Note: The URL parser interprets `oauth` as the host component and `/callback`
    /// as the path. The validation logic handles this standard custom protocol format.
    pub async fn handle_callback(&self, protocol_url: &str) -> Result<()> {
        info!("Handling protocol callback: {}", protocol_url);

        // Parse the protocol URL
        let url =
            Url::parse(protocol_url).map_err(|e| anyhow::anyhow!("Invalid protocol URL: {}", e))?;

        // Validate scheme
        if url.scheme() != "gdlauncher" {
            bail!(
                "Invalid protocol scheme: expected 'gdlauncher', got '{}'",
                url.scheme()
            );
        }

        // Validate path
        // The URL parser treats 'oauth' as the host and '/callback' as the path
        // For example: gdlauncher://oauth/callback -> host="oauth", path="/callback"
        let host = url.host_str();
        let path = url.path();

        let is_valid = match (host, path) {
            (Some("oauth"), "/callback") => true,
            // Also accept non-standard formats for compatibility
            (None, "/oauth/callback") | (None, "oauth/callback") => true,
            _ => false,
        };

        if !is_valid {
            bail!(
                "Invalid protocol URL: expected 'gdlauncher://oauth/callback', got host={:?} path='{}'",
                host,
                path
            );
        }

        // Parse query parameters
        let mut callback = ProtocolOAuthCallback {
            code: String::new(),
            state: None,
            error: None,
            error_description: None,
        };

        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "code" => callback.code = value.to_string(),
                "state" => callback.state = Some(value.to_string()),
                "error" => callback.error = Some(value.to_string()),
                "error_description" => callback.error_description = Some(value.to_string()),
                _ => {
                    debug!("Unknown query parameter in protocol callback: {}", key);
                }
            }
        }

        // Check for errors
        if let Some(error) = &callback.error {
            let description = callback
                .error_description
                .as_deref()
                .unwrap_or("Unknown error");
            warn!("OAuth protocol callback error: {} - {}", error, description);
            bail!("OAuth error: {} - {}", error, description);
        }

        // Validate we have a code
        if callback.code.is_empty() {
            bail!("Protocol callback missing authorization code");
        }

        info!("Successfully parsed OAuth protocol callback");

        // Store the pending code
        *self.pending_code.write().await = Some(callback);

        Ok(())
    }

    /// Retrieve and consume the pending authorization code
    ///
    /// This removes the code from storage after retrieving it (one-time use)
    pub async fn take_pending_code(&self) -> Option<ProtocolOAuthCallback> {
        self.pending_code.write().await.take()
    }

    /// Check if there's a pending authorization code
    pub async fn has_pending_code(&self) -> bool {
        self.pending_code.read().await.is_some()
    }

    /// Clear any pending authorization code
    pub async fn clear_pending_code(&self) {
        *self.pending_code.write().await = None;
    }
}

impl Default for ProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_valid_callback() {
        let handler = ProtocolHandler::new();
        let url = "gdlauncher://oauth/callback?code=test_code_123&state=test_state";

        let result = handler.handle_callback(url).await;
        assert!(result.is_ok());

        let callback = handler.take_pending_code().await;
        assert!(callback.is_some());

        let callback = callback.unwrap();
        assert_eq!(callback.code, "test_code_123");
        assert_eq!(callback.state.as_deref(), Some("test_state"));
        assert!(callback.error.is_none());
    }

    #[tokio::test]
    async fn test_parse_error_callback() {
        let handler = ProtocolHandler::new();
        // Test with standard format
        let url =
            "gdlauncher://oauth/callback?error=access_denied&error_description=User%20cancelled";

        let result = handler.handle_callback(url).await;
        assert!(result.is_err());

        let callback = handler.take_pending_code().await;
        assert!(callback.is_none());
    }

    #[tokio::test]
    async fn test_invalid_scheme() {
        let handler = ProtocolHandler::new();
        let url = "http://oauth/callback?code=test";

        let result = handler.handle_callback(url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_path() {
        let handler = ProtocolHandler::new();
        let url = "gdlauncher://wrong/path?code=test";

        let result = handler.handle_callback(url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_code() {
        let handler = ProtocolHandler::new();
        let url = "gdlauncher://oauth/callback";

        let result = handler.handle_callback(url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_take_pending_code_consumes() {
        let handler = ProtocolHandler::new();
        let url = "gdlauncher://oauth/callback?code=test";

        handler.handle_callback(url).await.unwrap();

        assert!(handler.has_pending_code().await);

        let first = handler.take_pending_code().await;
        assert!(first.is_some());

        // Second take should return None
        let second = handler.take_pending_code().await;
        assert!(second.is_none());

        assert!(!handler.has_pending_code().await);
    }
}
