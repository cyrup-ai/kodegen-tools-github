//! Issues API methods

use super::GitHubClient;
use crate::github::error::GitHubError;

impl GitHubClient {
    /// Get a single issue
    pub fn get_issue(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        issue_number: u64,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::get_issue::get_issue(self.inner.clone(), owner, repo, issue_number)
    }

    /// Create a new issue
    pub fn create_issue(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        title: impl Into<String>,
        body: Option<String>,
        assignees: Option<Vec<String>>,
        labels: Option<Vec<String>>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::create_issue::create_issue(
            self.inner.clone(),
            owner,
            repo,
            title,
            body,
            assignees,
            labels,
        )
    }

    /// Add a comment to an issue
    pub fn add_issue_comment(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        issue_number: u64,
        body: impl Into<String>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::issues::Comment, GitHubError>> {
        crate::github::add_issue_comment::add_issue_comment(
            self.inner.clone(),
            owner,
            repo,
            issue_number,
            body,
        )
    }

    /// Get comments for an issue
    pub fn get_issue_comments(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        issue_number: u64,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::issues::Comment, GitHubError>> {
        crate::github::get_issue_comments::get_issue_comments(
            self.inner.clone(),
            owner,
            repo,
            issue_number,
        )
    }

    /// List issues with filters
    #[must_use]
    pub fn list_issues(
        &self,
        request: crate::github::ListIssuesRequest,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::list_issues::list_issues(self.inner.clone(), request)
    }

    /// Update an issue
    #[must_use]
    pub fn update_issue(
        &self,
        request: crate::github::UpdateIssueRequest,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::update_issue::update_issue(self.inner.clone(), request)
    }

    /// Search issues
    pub fn search_issues(
        &self,
        query: impl Into<String>,
        sort: Option<String>,
        order: Option<String>,
        page: Option<u32>,
        per_page: Option<u8>,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::search_issues::search_issues(
            self.inner.clone(),
            query,
            sort,
            order,
            page,
            per_page,
        )
    }
}
