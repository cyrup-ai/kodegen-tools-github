//! Release Assets API methods

use super::GitHubClient;

impl GitHubClient {
    /// Upload an asset to a release
    ///
    /// Requires the release ID and binary content of the file.
    /// Returns the uploaded asset information including download URL.
    pub async fn upload_release_asset(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        options: crate::github::upload_release_asset::UploadAssetOptions,
    ) -> Result<octocrab::models::repos::Asset, crate::github::error::GitHubError> {
        crate::github::upload_release_asset::upload_release_asset(
            self.inner.clone(),
            &owner.into(),
            &repo.into(),
            options,
        )
        .await
        .map_err(crate::github::error::GitHubError::from)
    }

    /// Delete a release asset
    pub async fn delete_release_asset(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        asset_id: u64,
    ) -> Result<(), crate::github::error::GitHubError> {
        crate::github::upload_release_asset::delete_release_asset(
            self.inner.clone(),
            &owner.into(),
            &repo.into(),
            asset_id,
        )
        .await
        .map_err(crate::github::error::GitHubError::from)
    }
}
