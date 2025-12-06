//! GitHub pull requests listing tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{
    ListPullRequestsArgs, ListPullRequestsPrompts, GitHubListPrsOutput, GitHubPrSummary,
    GITHUB_LIST_PULL_REQUESTS,
};
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};

use crate::github::ListPullRequestsRequest;

/// Tool for listing and filtering GitHub pull requests
#[derive(Clone)]
pub struct ListPullRequestsTool;

impl Tool for ListPullRequestsTool {
    type Args = ListPullRequestsArgs;
    type Prompts = ListPullRequestsPrompts;

    fn name() -> &'static str {
        GITHUB_LIST_PULL_REQUESTS
    }

    fn description() -> &'static str {
        "List and filter pull requests in a GitHub repository. Supports filtering by state, labels, \
         and pagination. Returns an array of pull request objects. \
         Requires GITHUB_TOKEN environment variable."
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true
    }

    fn open_world() -> bool {
        true // Calls external GitHub API
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Convert state string to IssueState enum
        // Note: "all" is handled by passing None (no state filter)
        let state = args
            .state
            .as_ref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "open" => Some(octocrab::models::IssueState::Open),
                "closed" => Some(octocrab::models::IssueState::Closed),
                "all" => None,
                _ => None,
            });

        // Convert per_page to u8 (GitHub API expects u8)
        let per_page = args.per_page.map(|p| p.min(100) as u8);

        // Build request
        let request = ListPullRequestsRequest {
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            state,
            labels: args.labels.clone(),
            sort: None,
            direction: None,
            page: args.page,
            per_page,
        };

        // Call API wrapper
        let mut pr_stream = client.list_pull_requests(request);

        // Collect stream results
        let mut pull_requests = Vec::new();
        while let Some(result) = pr_stream.next().await {
            let pr = result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            pull_requests.push(pr);
        }

        // Convert to typed output
        let pr_summaries: Vec<GitHubPrSummary> = pull_requests
            .iter()
            .map(|pr| {
                let state_str = match pr.state {
                    Some(octocrab::models::IssueState::Open) => "open",
                    Some(octocrab::models::IssueState::Closed) => "closed",
                    _ => "unknown",
                };

                let author = pr.user.as_ref()
                    .map(|u| u.login.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let head_ref = pr.head.ref_field.clone();
                let base_ref = pr.base.ref_field.clone();

                let created_at = pr.created_at
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_default();

                GitHubPrSummary {
                    number: pr.number,
                    title: pr.title.clone().unwrap_or_default(),
                    state: state_str.to_string(),
                    author,
                    head_ref,
                    base_ref,
                    created_at,
                    draft: pr.draft.unwrap_or(false),
                }
            })
            .collect();

        let output = GitHubListPrsOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            count: pr_summaries.len(),
            pull_requests: pr_summaries,
        };

        // Build display string
        let state_filter = args.state.as_deref().unwrap_or("all");
        let display = format!(
            "Successfully listed {} pull request(s) from {}/{} (state: {})",
            output.count,
            args.owner,
            args.repo,
            state_filter
        );

        Ok(ToolResponse::new(display, output))
    }
}
