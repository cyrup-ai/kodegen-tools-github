//! API-based quality metrics computation from repository metadata

use octocrab::models::Repository;

use crate::github::search_repositories::types::ActivityMetrics;

/// Computes API-based quality metrics from repository metadata
pub(crate) fn compute_api_metrics(
    repo: &Repository,
    activity: &Option<ActivityMetrics>,
) -> (f32, ()) {
    let mut score: f32 = 0.0;

    // Stars contribute to score
    let stars = repo.stargazers_count.unwrap_or(0);
    if stars > 1000 {
        score += 0.3;
    } else if stars > 100 {
        score += 0.2;
    } else if stars > 10 {
        score += 0.1;
    }

    // Forks contribute to score
    let forks = repo.forks_count.unwrap_or(0);
    if forks > 100 {
        score += 0.2;
    } else if forks > 10 {
        score += 0.1;
    }

    // Recent activity contributes to score
    if let Some(act) = activity
        && act.commits_last_month > 0
    {
        score += 0.2;
    }

    // License presence
    if repo.license.is_some() {
        score += 0.1;
    }

    // Description presence
    if repo.description.is_some() {
        score += 0.1;
    }

    // Topics presence
    if let Some(topics) = &repo.topics
        && !topics.is_empty()
    {
        score += 0.1;
    }

    (score.min(1.0), ())
}
