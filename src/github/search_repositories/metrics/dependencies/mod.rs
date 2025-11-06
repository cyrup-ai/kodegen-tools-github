//! Dependency management and freshness metrics collection

mod freshness;
mod registry;
mod types;
mod version;

use log::warn;
use octocrab::models::repos::dependabot::State;
use reqwest::Client;
use serde_json::Value as JsonValue;
use std::path::Path;
use std::time::Duration;
use toml::Value as TomlValue;

use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::types::DependencyMetrics;

use freshness::calculate_dependency_freshness;
use registry::{check_cargo_outdated, check_npm_outdated, check_pypi_outdated};

/// Collects dependency metrics
pub(crate) async fn collect_dependency_metrics(
    repo_path: &Path,
    config: &SearchConfig,
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
) -> Option<DependencyMetrics> {
    let mut direct_dependencies = 0u32;
    let mut dev_dependencies = 0u32;
    let mut package_managers = Vec::new();
    let mut lock_files_present = false;

    // Store dependencies for version checking
    let mut cargo_deps: Option<toml::Table> = None;
    let mut npm_deps: Option<serde_json::Map<String, JsonValue>> = None;
    let mut python_reqs: Vec<String> = Vec::new();

    // Rust - Cargo.toml
    if let Ok(content) = std::fs::read_to_string(repo_path.join("Cargo.toml")) {
        package_managers.push("Cargo".to_string());
        if let Ok(toml_value) = content.parse::<TomlValue>() {
            if let Some(deps) = toml_value.get("dependencies").and_then(|v| v.as_table()) {
                direct_dependencies += deps.len() as u32;
                cargo_deps = Some(deps.clone());
            }
            if let Some(dev_deps) = toml_value
                .get("dev-dependencies")
                .and_then(|v| v.as_table())
            {
                dev_dependencies += dev_deps.len() as u32;
            }
        }
        if repo_path.join("Cargo.lock").exists() {
            lock_files_present = true;
        }
    }

    // JavaScript/Node - package.json
    if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json")) {
        package_managers.push("npm".to_string());
        if let Ok(json_value) = serde_json::from_str::<JsonValue>(&content) {
            if let Some(deps) = json_value.get("dependencies").and_then(|v| v.as_object()) {
                direct_dependencies += deps.len() as u32;
                npm_deps = Some(deps.clone());
            }
            if let Some(dev_deps) = json_value
                .get("devDependencies")
                .and_then(|v| v.as_object())
            {
                dev_dependencies += dev_deps.len() as u32;
            }
        }
        if repo_path.join("package-lock.json").exists() || repo_path.join("yarn.lock").exists() {
            lock_files_present = true;
        }
    }

    // Python - requirements.txt or pyproject.toml
    if repo_path.join("requirements.txt").exists() {
        package_managers.push("pip".to_string());
        if let Ok(content) = std::fs::read_to_string(repo_path.join("requirements.txt")) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    direct_dependencies += 1;
                    python_reqs.push(trimmed.to_string());
                }
            }
        }
    }
    if repo_path.join("Pipfile").exists() || repo_path.join("poetry.lock").exists() {
        lock_files_present = true;
    }

    // Go - go.mod
    if repo_path.join("go.mod").exists() {
        package_managers.push("Go modules".to_string());
        if let Ok(content) = std::fs::read_to_string(repo_path.join("go.mod")) {
            let mut in_require_block = false;
            for line in content.lines() {
                let trimmed = line.trim();

                // Check for start of require block
                if trimmed.starts_with("require (") || trimmed == "require (" {
                    in_require_block = true;
                    continue;
                }

                // Check for end of require block
                if in_require_block && trimmed.starts_with(')') {
                    in_require_block = false;
                    continue;
                }

                // Count dependencies inside require block
                if in_require_block && !trimmed.is_empty() && !trimmed.starts_with("//") {
                    direct_dependencies += 1;
                }

                // Handle single-line require statements
                if !in_require_block && trimmed.starts_with("require ") && !trimmed.contains('(') {
                    direct_dependencies += 1;
                }
            }
        }
        if repo_path.join("go.sum").exists() {
            lock_files_present = true;
        }
    }

    let total_dependencies = direct_dependencies + dev_dependencies;

    // Calculate freshness score based on lock file modification timestamps
    let dependency_freshness_score = calculate_dependency_freshness(repo_path, &package_managers);

    // Fetch vulnerable dependencies from Dependabot API
    let vulnerable_dependencies = match octocrab
        .repos(owner, repo)
        .dependabot()
        .per_page(config.api_page_size)
        .get_alerts()
        .await
    {
        Ok(page) => page
            .items
            .iter()
            .filter(|alert| matches!(alert.state, State::Open))
            .count() as u32,
        Err(e) => {
            warn!("Failed to fetch Dependabot alerts for {owner}/{repo}: {e} - defaulting to 0");
            0
        }
    };

    // Check for outdated dependencies using registry APIs
    // Performance guard: Only check if total deps <= 50
    let outdated_dependencies = if total_dependencies > 50 {
        0 // Too many to check efficiently
    } else {
        // Create HTTP client with timeout
        let client = match Client::builder().timeout(Duration::from_secs(5)).build() {
            Ok(c) => c,
            Err(_) => {
                return Some(DependencyMetrics {
                    total_dependencies,
                    direct_dependencies,
                    dev_dependencies,
                    outdated_dependencies: 0,
                    vulnerable_dependencies,
                    dependency_freshness_score,
                    package_managers,
                    lock_files_present,
                });
            }
        };

        // Set overall timeout of 10 seconds for all checks
        let check_future = async {
            // Run all registry checks in parallel for fair timeout distribution
            let (cargo_count, npm_count, pypi_count) = tokio::join!(
                async {
                    if let Some(ref deps) = cargo_deps {
                        check_cargo_outdated(deps, &client).await
                    } else {
                        0u32
                    }
                },
                async {
                    if let Some(ref deps) = npm_deps {
                        check_npm_outdated(deps, &client).await
                    } else {
                        0u32
                    }
                },
                async {
                    if python_reqs.is_empty() {
                        0u32
                    } else {
                        check_pypi_outdated(&python_reqs, &client).await
                    }
                }
            );

            cargo_count + npm_count + pypi_count
        };

        if let Ok(count) = tokio::time::timeout(Duration::from_secs(10), check_future).await {
            count
        } else {
            warn!("Timeout checking outdated dependencies for {owner}/{repo}");
            0
        }
    };

    Some(DependencyMetrics {
        total_dependencies,
        direct_dependencies,
        dev_dependencies,
        outdated_dependencies,
        vulnerable_dependencies,
        dependency_freshness_score,
        package_managers,
        lock_files_present,
    })
}
