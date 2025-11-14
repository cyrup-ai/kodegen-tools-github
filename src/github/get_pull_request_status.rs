//! GitHub Pull Request status retrieval operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::{CombinedStatus, pulls::PullRequest}};
use serde::Serialize;
use std::sync::Arc;

/// Combined pull request and status information.
#[derive(Debug, Clone, Serialize)]
pub struct PullRequestStatus {
    pub pr: PullRequest,
    pub combined_status: CombinedStatus,
}

/// Get combined status for a PR (via HEAD SHA).
pub(crate) fn get_pull_request_status(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    pr_number: u64,
) -> AsyncTask<Result<PullRequestStatus, GitHubError>> {
    let (owner, repo) = (owner.into(), repo.into());
    spawn_task(async move {
        let pr = inner
            .pulls(&owner, &repo)
            .get(pr_number)
            .await
            .map_err(GitHubError::from)?;

        let sha = pr.head.sha.clone();

        // Use direct GET since combined_status_for_ref doesn't support raw commit SHAs
        let combined_status: CombinedStatus = inner
            .get(
                format!("/repos/{owner}/{repo}/commits/{sha}/status"),
                None::<&()>,
            )
            .await
            .map_err(GitHubError::from)?;

        Ok(PullRequestStatus { pr, combined_status })
    })
}
