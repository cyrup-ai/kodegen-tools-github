//! Repositories API methods

use super::GitHubClient;
use crate::github::error::GitHubError;

impl GitHubClient {
    /// Get file contents
    pub fn get_file_contents(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        path: impl Into<String>,
        ref_name: Option<String>,
    ) -> crate::runtime::AsyncTask<Result<Vec<octocrab::models::repos::Content>, GitHubError>> {
        crate::github::get_file_contents::get_file_contents(
            self.inner.clone(),
            owner,
            repo,
            path,
            ref_name,
        )
    }

    /// Create or update a file
    #[must_use]
    pub fn create_or_update_file(
        &self,
        request: crate::github::CreateOrUpdateFileRequest,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::repos::FileUpdate, GitHubError>> {
        crate::github::create_or_update_file::create_or_update_file(self.inner.clone(), request)
    }

    /// List branches
    pub fn list_branches(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        page: Option<u32>,
        per_page: Option<u8>,
    ) -> crate::runtime::AsyncTask<Result<Vec<octocrab::models::repos::Branch>, GitHubError>> {
        crate::github::list_branches::list_branches(self.inner.clone(), owner, repo, page, per_page)
    }

    /// Create a branch
    pub fn create_branch(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        branch_name: impl Into<String>,
        sha: impl Into<String>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::repos::Ref, GitHubError>> {
        crate::github::create_branch::create_branch(
            self.inner.clone(),
            owner,
            repo,
            branch_name,
            sha,
        )
    }

    /// Delete a branch
    pub fn delete_branch(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        branch_name: impl Into<String>,
    ) -> crate::runtime::AsyncTask<Result<(), GitHubError>> {
        crate::github::delete_branch::delete_branch(
            self.inner.clone(),
            owner,
            repo,
            branch_name,
        )
    }

    /// List commits
    pub fn list_commits(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        options: crate::github::ListCommitsOptions,
    ) -> crate::runtime::AsyncTask<Result<Vec<octocrab::models::repos::RepoCommit>, GitHubError>>
    {
        crate::github::list_commits::list_commits(self.inner.clone(), owner, repo, options)
    }

    /// Get a commit
    pub fn get_commit(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        commit_sha: impl Into<String>,
        page: Option<u32>,
        per_page: Option<u8>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::repos::RepoCommit, GitHubError>> {
        crate::github::get_commit::get_commit(
            self.inner.clone(),
            owner,
            repo,
            commit_sha,
            page,
            per_page,
        )
    }

    /// Search code
    pub fn search_code(
        &self,
        query: impl Into<String>,
        sort: Option<String>,
        order: Option<String>,
        page: Option<u32>,
        per_page: Option<u8>,
        enrich_stars: bool,
    ) -> crate::runtime::AsyncTask<Result<octocrab::Page<octocrab::models::Code>, GitHubError>>
    {
        crate::github::search_code::search_code(
            self.inner.clone(),
            query,
            sort,
            order,
            page,
            per_page,
            enrich_stars,
        )
    }

    /// Create a repository
    pub fn create_repository(
        &self,
        name: impl Into<String>,
        description: Option<String>,
        private: Option<bool>,
        auto_init: Option<bool>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::Repository, GitHubError>> {
        crate::github::create_repository::create_repository(
            self.inner.clone(),
            name,
            description,
            private,
            auto_init,
        )
    }

    /// Fork a repository
    pub fn fork_repository(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        organization: Option<String>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::Repository, GitHubError>> {
        crate::github::fork_repository::fork_repository(
            self.inner.clone(),
            owner,
            repo,
            organization,
        )
    }

    /// Push files to a repository
    pub fn push_files(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        branch: impl Into<String>,
        files: std::collections::HashMap<String, String>,
        commit_message: impl Into<String>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::repos::Commit, GitHubError>> {
        crate::github::push_files::push_files(
            self.inner.clone(),
            owner,
            repo,
            branch,
            files,
            commit_message,
        )
    }
}
