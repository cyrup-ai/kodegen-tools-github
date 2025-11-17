//! GitHub branch deletion operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, params::repos::Reference};
use std::sync::Arc;

/// Delete a branch from a repository.
///
/// # Arguments
/// * `inner` - Octocrab client instance
/// * `owner` - Repository owner (user or organization)
/// * `repo` - Repository name
/// * `branch` - Branch name to delete
///
/// # Returns
/// AsyncTask that resolves to Result<(), GitHubError>
///
/// # Notes
/// - This deletes the branch reference from the remote repository
/// - The branch name should not include the "refs/heads/" prefix
/// - Cannot delete the default branch of a repository
/// - Requires push access to the repository
pub(crate) fn delete_branch(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    branch: impl Into<String>,
) -> AsyncTask<Result<(), GitHubError>> {
    let (owner, repo, branch) = (owner.into(), repo.into(), branch.into());
    spawn_task(async move {
        let reference = Reference::Branch(branch);
        inner
            .repos(&owner, &repo)
            .delete_ref(&reference)
            .await
            .map_err(GitHubError::from)
    })
}
