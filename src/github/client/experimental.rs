//! Experimental API methods

use super::GitHubClient;
use crate::github::error::GitHubError;

impl GitHubClient {
    /// Request a Copilot review
    pub fn request_copilot_review(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncTask<Result<(), GitHubError>> {
        crate::github::request_copilot_review::request_copilot_review(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }
}
