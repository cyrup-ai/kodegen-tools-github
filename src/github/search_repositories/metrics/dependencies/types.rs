//! API response type definitions for package registries

use serde::Deserialize;

/// User-Agent header for registry API requests
pub(crate) const USER_AGENT: &str = concat!("gitgix/", env!("CARGO_PKG_VERSION"));

// Crates.io API response structures
#[derive(Deserialize)]
pub(crate) struct CratesIoResponse {
    #[serde(rename = "crate")]
    pub crate_data: CrateData,
}

#[derive(Deserialize)]
pub(crate) struct CrateData {
    pub max_version: String,
}

// npm registry API response structures
#[derive(Deserialize)]
pub(crate) struct NpmPackageInfo {
    #[serde(rename = "dist-tags")]
    pub dist_tags: DistTags,
}

#[derive(Deserialize)]
pub(crate) struct DistTags {
    pub latest: String,
}

// PyPI API response structures
#[derive(Deserialize)]
pub(crate) struct PyPIPackageInfo {
    pub info: PyPIInfo,
}

#[derive(Deserialize)]
pub(crate) struct PyPIInfo {
    pub version: String,
}
