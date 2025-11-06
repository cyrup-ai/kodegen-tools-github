//! Package registry API checkers for outdated dependencies

use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde_json::Value as JsonValue;
use std::time::Duration;
use toml::Value as TomlValue;

use super::types::{CratesIoResponse, NpmPackageInfo, PyPIPackageInfo, USER_AGENT};
use super::version::is_outdated;

/// Check outdated dependencies for Cargo (Rust) projects
pub(crate) async fn check_cargo_outdated(dependencies: &toml::Table, client: &Client) -> u32 {
    let deps: Vec<_> = dependencies
        .iter()
        .filter_map(|(name, version_spec)| {
            let version = match version_spec {
                TomlValue::String(v) => v.clone(),
                TomlValue::Table(t) => {
                    if let Some(TomlValue::String(v)) = t.get("version") {
                        v.clone()
                    } else {
                        return None; // Skip path/git dependencies
                    }
                }
                _ => return None,
            };
            Some((name.clone(), version))
        })
        .collect();

    let client = client.clone();
    let results = stream::iter(deps)
        .map(|(name, version)| {
            let client = client.clone();
            async move {
                let url = format!("https://crates.io/api/v1/crates/{name}");
                if let Ok(Ok(response)) = tokio::time::timeout(
                    Duration::from_secs(5),
                    client.get(&url).header("User-Agent", USER_AGENT).send(),
                )
                .await
                    && let Ok(data) = response.json::<CratesIoResponse>().await
                    && is_outdated(&version, &data.crate_data.max_version)
                {
                    return 1u32;
                }
                0u32
            }
        })
        .buffer_unordered(5)
        .collect::<Vec<_>>()
        .await;

    results.iter().sum()
}

/// Check outdated dependencies for npm (JavaScript/Node) projects
pub(crate) async fn check_npm_outdated(
    dependencies: &serde_json::Map<String, JsonValue>,
    client: &Client,
) -> u32 {
    let deps: Vec<_> = dependencies
        .iter()
        .filter_map(|(name, version_spec)| {
            let version = version_spec.as_str()?.to_string();

            // Filter out special versions and non-semver dependencies
            if version.is_empty()
                || version == "latest"
                || version == "*"
                || version.starts_with("http://")
                || version.starts_with("https://")
                || version.starts_with("git+")
                || version.starts_with("file:")
                || version.starts_with("github:")
                || version.starts_with("gitlab:")
                || version.starts_with("bitbucket:")
            {
                return None;
            }

            Some((name.clone(), version))
        })
        .collect();

    let client = client.clone();
    let results = stream::iter(deps)
        .map(|(name, version)| {
            let client = client.clone();
            async move {
                let url = format!("https://registry.npmjs.org/{name}");
                if let Ok(Ok(response)) =
                    tokio::time::timeout(Duration::from_secs(5), client.get(&url).send()).await
                    && let Ok(data) = response.json::<NpmPackageInfo>().await
                    && is_outdated(&version, &data.dist_tags.latest)
                {
                    return 1u32;
                }
                0u32
            }
        })
        .buffer_unordered(5)
        .collect::<Vec<_>>()
        .await;

    results.iter().sum()
}

/// Check outdated dependencies for pip (Python) projects
pub(crate) async fn check_pypi_outdated(requirements: &[String], client: &Client) -> u32 {
    let deps: Vec<_> = requirements
        .iter()
        .filter_map(|requirement| {
            let requirement = requirement.trim();

            // Skip comments and pip options
            if requirement.starts_with('#') || requirement.starts_with('-') {
                return None;
            }

            // Try operators in order of specificity
            for op in ["==", ">=", "<=", "~=", "!=", ">", "<"] {
                if let Some(idx) = requirement.find(op) {
                    let name = requirement[..idx].trim();
                    let version_part = requirement[idx + op.len()..].trim();
                    // Handle compound specs like ">=1.0,<2.0"
                    let version = version_part.split(',').next().unwrap_or("").trim();

                    if !name.is_empty() && !version.is_empty() {
                        return Some((name.to_string(), version.to_string()));
                    }
                    return None;
                }
            }
            None // No version specifier found
        })
        .collect();

    let client = client.clone();
    let results = stream::iter(deps)
        .map(|(name, version)| {
            let client = client.clone();
            async move {
                let url = format!("https://pypi.org/pypi/{name}/json");
                if let Ok(Ok(response)) =
                    tokio::time::timeout(Duration::from_secs(5), client.get(&url).send()).await
                    && let Ok(data) = response.json::<PyPIPackageInfo>().await
                    && is_outdated(&version, &data.info.version)
                {
                    return 1u32;
                }
                0u32
            }
        })
        .buffer_unordered(5)
        .collect::<Vec<_>>()
        .await;

    results.iter().sum()
}
