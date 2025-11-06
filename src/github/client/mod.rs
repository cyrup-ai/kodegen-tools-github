//! GitHub API client wrapper
//!
//! Provides clean API for GitHub operations without exposing Octocrab.
//!
//! # Examples
//!
//! ```rust,no_run
//! use gitgix::GitHubClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let gh = GitHubClient::with_token("ghp_...")?;
//!
//!     // Use with any GitHub operation
//!     let issue = gitgix::create_issue(
//!         gh,
//!         "owner",
//!         "repo",
//!         "Issue title",
//!         None, None, None
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::github::error::{GitHubError, GitHubResult};
use jsonwebtoken::EncodingKey;
use octocrab::{Octocrab, models::AppId};
use std::sync::Arc;

mod issues;
mod pull_requests;
mod repositories;
mod users;
mod security;
mod releases;
mod experimental;

/// GitHub API client wrapper that encapsulates Octocrab.
///
/// Provides clean API without exposing Octocrab dependency.
/// Cloning is cheap (Arc clone).
#[derive(Clone, Debug)]
pub struct GitHubClient {
    inner: Arc<Octocrab>,
}

impl GitHubClient {
    /// Create a new client builder
    #[must_use]
    pub fn builder() -> GitHubClientBuilder {
        GitHubClientBuilder::new()
    }

    /// Convenience: create client with personal access token
    pub fn with_token(token: impl Into<String>) -> GitHubResult<Self> {
        Self::builder().personal_token(token).build()
    }

    /// Get inner Octocrab client
    #[must_use]
    pub fn inner(&self) -> &Arc<Octocrab> {
        &self.inner
    }
}

/// Builder for creating `GitHubClient` with various authentication methods
pub struct GitHubClientBuilder {
    token: Option<String>,
    app_auth: Option<(AppId, String)>,
    base_uri: Option<String>,
}

impl GitHubClientBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: None,
            app_auth: None,
            base_uri: None,
        }
    }

    /// Set personal access token for authentication
    pub fn personal_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set GitHub App authentication (app ID and private key)
    pub fn app(mut self, app_id: AppId, private_key: impl Into<String>) -> Self {
        self.app_auth = Some((app_id, private_key.into()));
        self
    }

    /// Set base URI (for GitHub Enterprise)
    pub fn base_uri(mut self, uri: impl Into<String>) -> Self {
        self.base_uri = Some(uri.into());
        self
    }

    /// Build the `GitHubClient`
    pub fn build(self) -> GitHubResult<GitHubClient> {
        let mut builder = Octocrab::builder();

        // Set authentication
        if let Some(token) = self.token {
            builder = builder.personal_token(token);
        } else if let Some((app_id, private_key)) = self.app_auth {
            let key = EncodingKey::from_rsa_pem(private_key.as_bytes())
                .map_err(|e| GitHubError::ClientSetup(format!("Invalid RSA key: {e}")))?;
            builder = builder.app(app_id, key);
        }

        // Set base URI if provided
        if let Some(uri) = self.base_uri {
            builder = builder
                .base_uri(&uri)
                .map_err(|e| GitHubError::ClientSetup(e.to_string()))?;
        }

        // Build Octocrab instance
        let octocrab = builder
            .build()
            .map_err(|e| GitHubError::ClientSetup(e.to_string()))?;

        Ok(GitHubClient {
            inner: Arc::new(octocrab),
        })
    }
}

impl Default for GitHubClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
