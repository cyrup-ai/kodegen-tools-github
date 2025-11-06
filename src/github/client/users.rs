//! Users API methods

use super::GitHubClient;
use crate::github::error::GitHubError;

impl GitHubClient {
    /// Get the authenticated user
    #[must_use]
    pub fn get_me(
        &self,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::Author, GitHubError>> {
        crate::github::get_me::get_me(self.inner.clone())
    }

    /// Search users
    pub fn search_users(
        &self,
        query: impl Into<String>,
        sort: Option<crate::github::search_users::UserSearchSort>,
        order: Option<crate::github::search_users::SearchOrder>,
        page: Option<u32>,
        per_page: Option<u8>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::Page<octocrab::models::Author>, GitHubError>>
    {
        crate::github::search_users::search_users(
            self.inner.clone(),
            query,
            sort,
            order,
            page,
            per_page,
        )
    }
}
