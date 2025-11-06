//! Security API methods (code scanning and secret scanning)

use super::GitHubClient;
use crate::github::error::GitHubError;

impl GitHubClient {
    /// List code scanning alerts
    pub fn list_code_scanning_alerts(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        state: Option<String>,
        ref_name: Option<String>,
        tool_name: Option<String>,
        severity: Option<String>,
    ) -> crate::runtime::AsyncTask<Result<Vec<serde_json::Value>, GitHubError>> {
        crate::github::code_scanning_alerts::list_code_scanning_alerts(
            self.inner.clone(),
            owner,
            repo,
            state,
            ref_name,
            tool_name,
            severity,
        )
    }

    /// Get a code scanning alert
    pub fn get_code_scanning_alert(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        alert_number: u64,
    ) -> crate::runtime::AsyncTask<Result<serde_json::Value, GitHubError>> {
        crate::github::code_scanning_alerts::get_code_scanning_alert(
            self.inner.clone(),
            owner,
            repo,
            alert_number,
        )
    }

    /// List secret scanning alerts
    pub fn list_secret_scanning_alerts(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        state: Option<String>,
        secret_type: Option<String>,
        resolution: Option<String>,
    ) -> crate::runtime::AsyncTask<
        Result<
            Vec<octocrab::models::repos::secret_scanning_alert::SecretScanningAlert>,
            GitHubError,
        >,
    > {
        crate::github::secret_scanning_alerts::list_secret_scanning_alerts(
            self.inner.clone(),
            owner,
            repo,
            state,
            secret_type,
            resolution,
        )
    }

    /// Get a secret scanning alert
    pub fn get_secret_scanning_alert(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        alert_number: u32,
    ) -> crate::runtime::AsyncTask<
        Result<octocrab::models::repos::secret_scanning_alert::SecretScanningAlert, GitHubError>,
    > {
        crate::github::secret_scanning_alerts::get_secret_scanning_alert(
            self.inner.clone(),
            owner,
            repo,
            alert_number,
        )
    }
}
