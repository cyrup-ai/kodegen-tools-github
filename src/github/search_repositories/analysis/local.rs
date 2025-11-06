//! Local repository analysis by cloning and scanning

use log::warn;
use octocrab::{Octocrab, models::Repository};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tempfile::TempDir;

use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::metrics::{
    MetricsCollectionContext, collect_local_metrics,
};
use crate::github::search_repositories::types::{
    LocalScores, SearchError, SearchResult, WikiInfo,
};

use super::activity::query_build_status;
use super::security::calculate_signed_commits_ratio;

/// Context for local repository analysis.
pub(crate) struct LocalAnalysisContext<'a> {
    pub repo_name: &'a str,
    pub url: &'a str,
    pub owner: &'a str,
    pub repo_name_str: &'a str,
    pub wiki_info: WikiInfo,
}

/// Performs local repository analysis by cloning and scanning
pub(crate) async fn local_analysis(
    context: LocalAnalysisContext<'_>,
    repo: &Repository,
    octocrab: Arc<Octocrab>,
    config: &SearchConfig,
) -> SearchResult<LocalScores> {
    // Check repository size before cloning
    let repo_size_kb = u64::from(repo.size.unwrap_or(0));
    let repo_size_bytes = repo_size_kb * 1024;

    if repo_size_bytes > config.max_repo_size {
        warn!(
            "Repository {} too large: {} bytes (max: {} bytes) - skipping clone",
            context.repo_name, repo_size_bytes, config.max_repo_size
        );

        // Return default metrics without local analysis
        return Ok(LocalScores {
            overall_local: 0.3, // Low score for oversized repos
            readme_score: 0.0,
            coverage_score: 0.0,
            metrics: None,
        });
    }

    // Create temp directory for cloning
    let temp_dir =
        TempDir::new().map_err(|e| SearchError::LocalAnalysisError(format!("Temp dir: {e}")))?;

    let repo_path = temp_dir.path();

    // Clone repository using gix with timeout protection
    let url_owned = context.url.to_string();
    let repo_path_owned = repo_path.to_path_buf();

    let clone_result = tokio::time::timeout(
        config.fetch_timeout,
        tokio::task::spawn_blocking(move || {
            // Parse URL to gix::Url type
            let parsed_url = gix::url::parse(url_owned.as_str().into())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let mut prep = gix::prepare_clone(parsed_url, &repo_path_owned)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let (checkout, outcome) = prep
                .fetch_then_checkout(gix::progress::Discard, &AtomicBool::new(false))
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((checkout, outcome))
        }),
    )
    .await;

    let gix_repo = match clone_result {
        // Timeout occurred
        Err(_) => {
            warn!(
                "Clone timeout for {} after {:?}",
                context.repo_name, config.fetch_timeout
            );
            return Err(SearchError::TimeoutError {
                operation: format!("git_clone_{}", context.repo_name),
                duration: config.fetch_timeout,
            });
        }
        // spawn_blocking panicked or was cancelled
        Ok(Err(e)) => {
            warn!("Clone task failed for {}: {:?}", context.repo_name, e);
            return Ok(LocalScores {
                overall_local: 0.3,
                readme_score: 0.0,
                coverage_score: 0.0,
                metrics: None,
            });
        }
        // Clone operation failed
        Ok(Ok(Err(e))) => {
            warn!("Clone failed for {}: {}", context.repo_name, e);
            return Ok(LocalScores {
                overall_local: 0.3,
                readme_score: 0.0,
                coverage_score: 0.0,
                metrics: None,
            });
        }
        // Success
        Ok(Ok(Ok((checkout, _)))) => Some(checkout.persist()),
    };

    // Calculate signed commits ratio synchronously to avoid Send issues
    let signed_commits_ratio = if let Some(repo) = gix_repo.as_ref() {
        calculate_signed_commits_ratio(repo)
    } else {
        0.0
    };

    // Query build status if GitHub Actions is configured
    let build_status = if repo_path.join(".github/workflows").exists() {
        query_build_status(&octocrab, context.owner, context.repo_name_str).await
    } else {
        "no_ci".to_string()
    };

    // Collect metrics with timeout protection (pass ratio instead of repo reference)
    let metrics_context = MetricsCollectionContext {
        signed_commits_ratio,
        build_status,
        owner: context.owner,
        repo: context.repo_name_str,
        wiki_info: context.wiki_info,
    };
    let local_metrics = match tokio::time::timeout(
        Duration::from_secs(10),
        collect_local_metrics(repo_path, metrics_context, config, &octocrab),
    )
    .await
    {
        Ok(Some(metrics)) => metrics,
        Ok(None) => {
            warn!("Metrics collection failed for {}", context.repo_name);
            return Ok(LocalScores {
                overall_local: 0.3,
                readme_score: 0.0,
                coverage_score: 0.0,
                metrics: None,
            });
        }
        Err(_) => {
            warn!("Timeout collecting metrics for {}", context.repo_name);
            return Ok(LocalScores {
                overall_local: 0.3,
                readme_score: 0.0,
                coverage_score: 0.0,
                metrics: None,
            });
        }
    };

    // Calculate scores from metrics
    let readme_score = local_metrics.readme_quality.quality_score / 100.0;
    let coverage_score = local_metrics.test_metrics.test_coverage_estimate;
    let overall_local = f32::midpoint(readme_score, coverage_score);

    Ok(LocalScores {
        overall_local,
        readme_score,
        coverage_score,
        metrics: Some(local_metrics),
    })
}
