//! GitHub Pull Requests listing operation.

use crate::github::error::GitHubError;
use crate::runtime::{AsyncStream, EmitterBuilder};
use octocrab::models::IssueState;
use octocrab::models::pulls::PullRequest;
use octocrab::{Octocrab, Page, params};
use std::sync::Arc;

/// Request parameters for listing pull requests
#[derive(Debug, Clone)]
pub struct ListPullRequestsRequest {
    /// Repository owner (user or organization)
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Filter by pull request state (open, closed, all)
    pub state: Option<IssueState>,
    /// Filter by labels
    pub labels: Option<Vec<String>>,
    /// Sort field (created, updated, popularity, long-running)
    pub sort: Option<String>,
    /// Sort direction (asc, desc)
    pub direction: Option<String>,
    /// Page number for pagination
    pub page: Option<u32>,
    /// Results per page (max 100)
    pub per_page: Option<u8>,
}

/// List pull requests with optional filters. Uses a stream because the result can be large.
pub(crate) fn list_pull_requests(
    inner: Arc<Octocrab>,
    request: ListPullRequestsRequest,
) -> AsyncStream<Result<PullRequest, GitHubError>> {
    let builder = EmitterBuilder::new(Box::new(move || {
        let request = request.clone();
        Box::pin(async move {
            let mut pull_requests = Vec::new();
            let pulls_handler = inner.pulls(&request.owner, &request.repo);
            let mut req = pulls_handler.list();

            if let Some(state) = request.state {
                let param_state = match state {
                    IssueState::Open => params::State::Open,
                    IssueState::Closed => params::State::Closed,
                    _ => params::State::All,
                };
                req = req.state(param_state);
            }

            // Note: GitHub API for pull requests doesn't have a direct labels filter
            // Labels would need to be filtered client-side if needed

            if let Some(sort) = &request.sort {
                let sort_param = match sort.as_str() {
                    "created" => params::pulls::Sort::Created,
                    "updated" => params::pulls::Sort::Updated,
                    "popularity" => params::pulls::Sort::Popularity,
                    "long-running" => params::pulls::Sort::LongRunning,
                    _ => params::pulls::Sort::Created,
                };
                req = req.sort(sort_param);
            }

            if let Some(direction) = &request.direction {
                let dir_param = match direction.as_str() {
                    "asc" => params::Direction::Ascending,
                    "desc" => params::Direction::Descending,
                    _ => params::Direction::Descending,
                };
                req = req.direction(dir_param);
            }

            if let Some(page) = request.page {
                req = req.page(page);
            }

            if let Some(per_page) = request.per_page {
                req = req.per_page(per_page);
            }

            let mut page_res: Page<PullRequest> = req.send().await.map_err(GitHubError::from)?;
            let mut items = page_res.items;

            // Filter by labels client-side if labels were specified
            if let Some(labels) = &request.labels {
                items.retain(|pr| {
                    if let Some(pr_labels) = &pr.labels {
                        labels.iter().all(|label| {
                            pr_labels.iter().any(|pr_label| pr_label.name == *label)
                        })
                    } else {
                        false
                    }
                });
            }

            pull_requests.extend(items);

            while let Some(next_page) = inner.get_page::<PullRequest>(&page_res.next).await? {
                page_res = next_page;
                let mut items = page_res.items;

                // Filter by labels client-side if labels were specified
                if let Some(labels) = &request.labels {
                    items.retain(|pr| {
                        if let Some(pr_labels) = &pr.labels {
                            labels.iter().all(|label| {
                                pr_labels.iter().any(|pr_label| pr_label.name == *label)
                            })
                        } else {
                            false
                        }
                    });
                }

                pull_requests.extend(items);
            }
            Ok(pull_requests)
        })
    }));
    builder.emit(|v| v, |_| {})
}
