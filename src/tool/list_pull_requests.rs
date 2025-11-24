//! GitHub pull requests listing tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{
    ListPullRequestsArgs, ListPullRequestsPromptArgs, GITHUB_LIST_PULL_REQUESTS,
};
use kodegen_mcp_tool::{error::McpError, Tool, ToolExecutionContext};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;

use crate::github::ListPullRequestsRequest;

/// Tool for listing and filtering GitHub pull requests
#[derive(Clone)]
pub struct ListPullRequestsTool;

impl Tool for ListPullRequestsTool {
    type Args = ListPullRequestsArgs;
    type PromptArgs = ListPullRequestsPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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

        // Clone values before moving them
        let owner = args.owner.clone();
        let repo = args.repo.clone();

        // Build request
        let request = ListPullRequestsRequest {
            owner: args.owner,
            repo: args.repo,
            state,
            labels: args.labels,
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

        // Count open and closed pull requests
        let open_count = pull_requests
            .iter()
            .filter(|pr| matches!(pr.state, Some(octocrab::models::IssueState::Open)))
            .count();
        let closed_count = pull_requests
            .iter()
            .filter(|pr| matches!(pr.state, Some(octocrab::models::IssueState::Closed)))
            .count();
        let total_count = pull_requests.len();

        // Build dual-content response
        let mut contents = Vec::new();

        // Content[0]: Human-Readable Summary
        // Line 1: Status Header with ANSI cyan color and Nerd Font icon
        // Line 2: Summary Statistics with info icon
        let summary = format!(
            "\x1b[36m 🔀 Pull Requests: {}/{}\x1b[0m\n  ℹ️  Total: {} · Open: {} · Closed: {}",
            owner, repo, total_count, open_count, closed_count
        );
        contents.push(Content::text(summary));

        // Content[1]: Machine-Parseable JSON
        let metadata = json!({
            "pull_requests": pull_requests,
            "count": pull_requests.len()
        });
        let json_str =
            serde_json::to_string_pretty(&metadata).unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "focus_area".to_string(),
            title: None,
            description: Some(
                "Optional focus area: 'overview', 'filtering', 'pagination', or 'advanced'"
                    .to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I list and filter GitHub pull requests?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the list_pull_requests tool to list and filter repository pull requests:\n\n\
                     List all open pull requests:\n\
                     list_pull_requests({\"owner\": \"octocat\", \"repo\": \"hello-world\"})\n\n\
                     Filter by state:\n\
                     list_pull_requests({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"state\": \"closed\"})\n\n\
                     Filter by labels (multiple labels = AND logic):\n\
                     list_pull_requests({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"labels\": [\"bug\", \"priority-high\"]})\n\n\
                     With pagination:\n\
                     list_pull_requests({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"per_page\": 50, \"page\": 2})\n\n\
                     Combined filters:\n\
                     list_pull_requests({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\",\n\
                       \"labels\": [\"bug\"],\n\
                       \"per_page\": 20\n\
                     })\n\n\
                     Filter options:\n\
                     - state: \"open\" (default), \"closed\", or \"all\"\n\
                     - labels: Array of label names (matches PRs with ALL labels)\n\
                     - per_page: Results per page (max 100, default 30)\n\
                     - page: Page number for pagination\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos",
                ),
            },
        ])
    }
}
