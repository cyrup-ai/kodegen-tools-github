//! Activity metrics computation from commit history

use chrono::Utc;
use log::warn;
use octocrab::{
    Octocrab,
    models::repos::RepoCommit,
    params,
};
use std::collections::HashSet;

use crate::github::search_repositories::types::ActivityMetrics;

/// Computes activity metrics from commit history
pub(crate) async fn compute_activity(
    commits: &[RepoCommit],
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
) -> Option<ActivityMetrics> {
    if commits.is_empty() {
        return None;
    }

    let last_commit = &commits[0];
    let now = Utc::now();
    let thirty_days_ago = now - chrono::Duration::days(30);
    let ninety_days_ago = now - chrono::Duration::days(90);
    let six_months_ago = now - chrono::Duration::days(180);
    let one_year_ago = now - chrono::Duration::days(365);

    // Time-based commit counts
    let commits_last_month = commits
        .iter()
        .filter(|c| {
            c.commit
                .author
                .as_ref()
                .and_then(|a| a.date)
                .is_some_and(|date| date > thirty_days_ago)
        })
        .count() as u32;

    let commits_last_6_months = commits
        .iter()
        .filter(|c| {
            c.commit
                .author
                .as_ref()
                .and_then(|a| a.date)
                .is_some_and(|date| date > six_months_ago)
        })
        .count() as u32;

    let commits_last_year = commits
        .iter()
        .filter(|c| {
            c.commit
                .author
                .as_ref()
                .and_then(|a| a.date)
                .is_some_and(|date| date > one_year_ago)
        })
        .count() as u32;

    // Active contributors (unique authors in last 3 months)
    let active_authors: HashSet<String> = commits
        .iter()
        .filter(|c| {
            c.commit
                .author
                .as_ref()
                .and_then(|a| a.date)
                .is_some_and(|date| date > ninety_days_ago)
        })
        .filter_map(|c| c.commit.author.as_ref().and_then(|a| a.email.clone()))
        .collect();

    let active_contributors_last_3_months = active_authors.len() as u32;

    // Fetch contributors count
    let contributors_count = match octocrab
        .repos(owner, repo)
        .list_contributors()
        .per_page(100)
        .send()
        .await
    {
        Ok(contributors_page) => contributors_page.items.len() as u32,
        Err(e) => {
            warn!("Failed to fetch contributors for {owner}/{repo}: {e}");
            1
        }
    };

    // Fetch merged PRs in last month
    let pull_requests_merged_last_month = match octocrab
        .pulls(owner, repo)
        .list()
        .state(params::State::Closed)
        .per_page(100)
        .send()
        .await
    {
        Ok(prs_page) => prs_page
            .items
            .iter()
            .filter(|pr| {
                pr.merged_at
                    .is_some_and(|merged_at| merged_at > thirty_days_ago)
            })
            .count() as u32,
        Err(e) => {
            warn!("Failed to fetch pull requests for {owner}/{repo}: {e}");
            0
        }
    };

    // Fetch closed issues in last month (excluding PRs)
    let issues_closed_last_month = match octocrab
        .issues(owner, repo)
        .list()
        .state(params::State::Closed)
        .per_page(100)
        .send()
        .await
    {
        Ok(issues_page) => issues_page
            .items
            .iter()
            .filter(|issue| {
                issue.pull_request.is_none()
                    && issue
                        .closed_at
                        .is_some_and(|closed| closed > thirty_days_ago)
            })
            .count() as u32,
        Err(e) => {
            warn!("Failed to fetch issues for {owner}/{repo}: {e}");
            0
        }
    };

    // Fetch releases and calculate frequency
    let (release_frequency, latest_release) = match octocrab
        .repos(owner, repo)
        .releases()
        .list()
        .per_page(20)
        .send()
        .await
    {
        Ok(releases_page) => {
            let releases = releases_page.items;

            let latest = releases.first().map(|r| r.tag_name.clone());

            let frequency = if releases.len() >= 2 {
                let newest = releases[0]
                    .created_at
                    .or(releases[0].published_at)
                    .unwrap_or_else(Utc::now);

                let oldest = releases[releases.len() - 1]
                    .created_at
                    .or(releases[releases.len() - 1].published_at)
                    .unwrap_or_else(Utc::now);

                let days_between = (newest - oldest).num_days();
                let avg_days = days_between / (releases.len() as i64 - 1);

                if avg_days < 30 {
                    "monthly"
                } else if avg_days < 90 {
                    "quarterly"
                } else if avg_days < 180 {
                    "biannual"
                } else {
                    "annual"
                }
            } else if releases.len() == 1 {
                "single"
            } else {
                "none"
            };

            (frequency.to_string(), latest)
        }
        Err(e) => {
            warn!("Failed to fetch releases for {owner}/{repo}: {e}");
            ("unknown".to_string(), None)
        }
    };

    Some(ActivityMetrics {
        commits_last_month,
        commits_last_6_months,
        commits_last_year,
        last_commit: last_commit.sha.clone(),
        last_commit_date: last_commit
            .commit
            .author
            .as_ref()
            .and_then(|a| a.date)
            .unwrap_or_else(Utc::now),
        contributors_count,
        active_contributors_last_3_months,
        pull_requests_merged_last_month,
        issues_closed_last_month,
        release_frequency,
        latest_release,
    })
}

/// Queries the latest GitHub Actions workflow run status
pub(crate) async fn query_build_status(octocrab: &Octocrab, owner: &str, repo: &str) -> String {
    match octocrab
        .workflows(owner, repo)
        .list_all_runs()
        .per_page(1)
        .send()
        .await
    {
        Ok(runs_page) => {
            if let Some(run) = runs_page.items.first() {
                // If workflow completed, return conclusion
                if let Some(conclusion) = &run.conclusion {
                    return conclusion.to_lowercase();
                }
                // Still running/queued
                return "pending".to_string();
            }
            // Repository has GitHub Actions but no runs yet
            "no_runs".to_string()
        }
        Err(e) => {
            warn!("Failed to query build status for {owner}/{repo}: {e}");
            "unknown".to_string()
        }
    }
}
