//! Repository analysis logic

mod activity;
mod api_metrics;
mod local;
mod security;

use chrono::Utc;
use log::info;
use octocrab::{Octocrab, models::Repository};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::github::search_repositories::cache::SearchCache;
use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::rate_limiter::RateLimiter;
use crate::github::search_repositories::types::{
    QualityMetrics, RepositoryResult, SearchError, SearchResult, WikiInfo,
};

// Re-export public API
pub(crate) use activity::compute_activity;
pub(crate) use api_metrics::compute_api_metrics;
pub(crate) use local::{LocalAnalysisContext, local_analysis};
pub(crate) use security::calculate_security_score;

/// Analyzes a single repository with caching
pub(crate) async fn analyze_repo(
    octocrab: Arc<Octocrab>,
    cache: Arc<Mutex<SearchCache>>,
    repo: Repository,
    _token: String,
    config: SearchConfig,
    _rate_limiter: Arc<RwLock<RateLimiter>>,
) -> SearchResult<RepositoryResult> {
    let repo_name = repo.full_name.as_deref().unwrap_or("unknown");
    let url = repo.clone_url.as_ref().map_or("", reqwest::Url::as_str);
    let stars = repo.stargazers_count.unwrap_or(0);

    // Extract owner safely - repository must have owner to fetch commits
    let owner_login = repo
        .owner
        .as_ref()
        .ok_or_else(|| SearchError::InvalidQuery {
            details: format!(
                "Repository '{repo_name}' has no owner information - cannot fetch commits"
            ),
        })?
        .login
        .as_str();

    // Get latest commit
    let commits_resp = octocrab
        .repos(owner_login, &repo.name)
        .list_commits()
        .per_page(1)
        .send()
        .await
        .map_err(|e| SearchError::ApiError(e.to_string()))?;

    let latest_sha = commits_resp
        .items
        .first()
        .map(|c| c.sha.clone())
        .unwrap_or_default();

    // Check cache
    {
        let mut c = cache.lock().await;
        if let Some(found) = c.get_if_valid(repo_name, &latest_sha) {
            info!("Cache hit for {repo_name}");
            return Ok(found);
        }
    }

    // Compute activity metrics
    let activity = compute_activity(&commits_resp.items, &octocrab, owner_login, &repo.name).await;

    // Compute API metrics
    let (api_score, ()) = compute_api_metrics(&repo, &activity);

    // Extract wiki information
    let wiki_info = WikiInfo {
        has_wiki: repo.has_wiki.unwrap_or(false),
        clone_url: url.to_string(),
    };

    // Perform local analysis
    let context = LocalAnalysisContext {
        repo_name,
        url,
        owner: owner_login,
        repo_name_str: &repo.name,
        wiki_info,
    };
    let local_scores = local_analysis(context, &repo, octocrab.clone(), &config).await?;

    // Combine scores
    let overall_score = 0.7 * api_score + 0.3 * local_scores.overall_local;

    let result = RepositoryResult {
        name: repo.name.clone(),
        full_name: repo_name.to_string(),
        url: repo
            .html_url
            .as_ref()
            .map_or("", reqwest::Url::as_str)
            .to_string(),
        clone_url: url.to_string(),
        description: repo.description,
        stars,
        forks: repo.forks_count.unwrap_or(0),
        watchers: repo.watchers_count.unwrap_or(0),
        language: repo
            .language
            .as_ref()
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string),
        topics: repo.topics.unwrap_or_default(),
        license: repo.license.map(|l| l.name),
        created_at: repo.created_at.unwrap_or_else(Utc::now),
        updated_at: repo.updated_at.unwrap_or_else(Utc::now),
        pushed_at: repo.pushed_at.unwrap_or_else(Utc::now),
        size_kb: repo.size.unwrap_or(0),
        quality_metrics: QualityMetrics {
            overall_score,
            api_score,
            local_score: local_scores.overall_local,
            popularity_score: local_scores.readme_score,
            maintenance_score: local_scores.coverage_score,
            documentation_score: local_scores.readme_score,
            security_score: local_scores
                .metrics
                .as_ref()
                .map_or(0.5, |m| calculate_security_score(&m.security_metrics)),
        },
        activity_metrics: activity,
        local_metrics: local_scores.metrics,
        errors: vec![],
    };

    Ok(result)
}
