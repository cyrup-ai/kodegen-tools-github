//! GitHub issues search tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{
    SearchIssuesArgs, SearchIssuesPrompts, GitHubSearchIssuesOutput,
    GITHUB_SEARCH_ISSUES,
};
use kodegen_mcp_schema::github::search_issues::GitHubIssueSummary;
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for searching GitHub issues using GitHub's search syntax
#[derive(Clone)]
pub struct SearchIssuesTool;

impl Tool for SearchIssuesTool {
    type Args = SearchIssuesArgs;
    type Prompts = SearchIssuesPrompts;

    fn name() -> &'static str {
        GITHUB_SEARCH_ISSUES
    }

    fn description() -> &'static str {
        "Search for issues across GitHub using GitHub's powerful search syntax. \
         Supports filtering by repository, state, labels, assignee, author, dates, and more. \
         Returns matching issues with relevance ranking. \
         Requires GITHUB_TOKEN environment variable. Note: Search API has stricter rate limits."
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

        // Convert per_page to u8 (GitHub API expects u8)
        let per_page = args.per_page.map(|p| p.min(100) as u8);

        // Clone query before moving it
        let query = args.query.clone();

        // Call API wrapper
        let mut issue_stream =
            client.search_issues(args.query, args.sort, args.order, args.page, per_page);

        // Collect stream results
        let mut issues = Vec::new();
        while let Some(result) = issue_stream.next().await {
            let issue =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            issues.push(issue);
        }

        // Convert to typed output
        let issue_summaries: Vec<GitHubIssueSummary> = issues
            .iter()
            .map(|issue| {
                let state_str = match issue.state {
                    octocrab::models::IssueState::Open => "open",
                    octocrab::models::IssueState::Closed => "closed",
                    _ => "unknown",
                };
                let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();

                GitHubIssueSummary {
                    number: issue.number,
                    title: issue.title.clone(),
                    state: state_str.to_string(),
                    author: issue.user.login.clone(),
                    created_at: issue.created_at.to_rfc3339(),
                    labels,
                }
            })
            .collect();

        let output = GitHubSearchIssuesOutput {
            success: true,
            query: query.clone(),
            total_count: issue_summaries.len() as u32,
            items: issue_summaries,
        };

        // Build user-friendly display string
        let display = format!(
            "GitHub Issues Search Results\n\nQuery: {}\nTotal Results: {}\nResults Returned: {}\n\nSearch completed successfully.",
            query,
            output.total_count,
            output.items.len()
        );

        Ok(ToolResponse::new(display, output))
    }
}
