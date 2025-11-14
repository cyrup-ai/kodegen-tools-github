//! Pull Requests API methods

use super::GitHubClient;
use crate::github::error::GitHubError;

impl GitHubClient {
    /// Create a pull request
    #[must_use]
    pub fn create_pull_request(
        &self,
        request: crate::github::CreatePullRequestRequest,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::pulls::PullRequest, GitHubError>> {
        crate::github::create_pull_request::create_pull_request(self.inner.clone(), request)
    }

    /// Get pull request status
    pub fn get_pull_request_status(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncTask<Result<crate::github::PullRequestStatus, GitHubError>> {
        crate::github::get_pull_request_status::get_pull_request_status(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }

    /// Get pull request comments
    pub fn get_pull_request_comments(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::pulls::Comment, GitHubError>> {
        crate::github::get_pull_request_comments::get_pull_request_comments(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }

    /// Get pull request files
    pub fn get_pull_request_files(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::repos::DiffEntry, GitHubError>> {
        crate::github::get_pull_request_files::get_pull_request_files(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }

    /// Get pull request reviews
    pub fn get_pull_request_reviews(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::pulls::Review, GitHubError>> {
        crate::github::get_pull_request_reviews::get_pull_request_reviews(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }

    /// Create a pull request review
    pub fn create_pull_request_review(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
        options: crate::github::CreatePullRequestReviewOptions,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::pulls::Review, GitHubError>> {
        crate::github::create_pull_request_review::create_pull_request_review(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
            options,
        )
    }

    /// Add a review comment to a pull request
    #[must_use]
    pub fn add_pull_request_review_comment(
        &self,
        request: crate::github::AddPullRequestReviewCommentRequest,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::pulls::ReviewComment, GitHubError>>
    {
        crate::github::add_pull_request_review_comment::add_pull_request_review_comment(
            self.inner.clone(),
            request,
        )
    }

    /// Update a pull request
    pub fn update_pull_request(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
        options: crate::github::UpdatePullRequestOptions,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::pulls::PullRequest, GitHubError>> {
        crate::github::update_pull_request::update_pull_request(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
            options,
        )
    }

    /// Merge a pull request
    pub fn merge_pull_request(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
        options: crate::github::MergePullRequestOptions,
    ) -> crate::runtime::AsyncTask<Result<serde_json::Value, GitHubError>> {
        crate::github::merge_pull_request::merge_pull_request(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
            options,
        )
    }
}
