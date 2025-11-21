//! GitHub issues listing tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{ListIssuesArgs, ListIssuesPromptArgs, GITHUB_LIST_ISSUES};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;

use crate::github::ListIssuesRequest;

/// Tool for listing and filtering GitHub issues
#[derive(Clone)]
pub struct ListIssuesTool;

impl Tool for ListIssuesTool {
    type Args = ListIssuesArgs;
    type PromptArgs = ListIssuesPromptArgs;

    fn name() -> &'static str {
        GITHUB_LIST_ISSUES
    }

    fn description() -> &'static str {
        "List and filter issues in a GitHub repository. Supports filtering by state, labels, \
         assignee, and pagination. Returns an array of issue objects. \
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
        let request = ListIssuesRequest {
            owner: args.owner,
            repo: args.repo,
            state,
            labels: args.labels,
            sort: None,
            direction: None,
            since: None,
            page: args.page,
            per_page,
        };

        // Call API wrapper
        let mut issue_stream = client.list_issues(request);

        // Collect stream results
        let mut issues = Vec::new();
        while let Some(result) = issue_stream.next().await {
            let issue =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            issues.push(issue);
        }

        // Count open and closed issues
        let open_count = issues.iter()
            .filter(|i| matches!(i.state, octocrab::models::IssueState::Open))
            .count();
        let closed_count = issues.iter()
            .filter(|i| matches!(i.state, octocrab::models::IssueState::Closed))
            .count();
        let total_count = issues.len();

        // Build dual-content response
        let mut contents = Vec::new();

        // Content[0]: Human-Readable Summary
        // Line 1: Status Header with ANSI cyan color and Nerd Font icon
        // Line 2: Summary Statistics with info icon
        let summary = format!(
            "\x1b[36m📋 Issues: {}/{}\x1b[0m\n  ℹ️  Total: {} · Open: {} · Closed: {}",
            owner,
            repo,
            total_count,
            open_count,
            closed_count
        );
        contents.push(Content::text(summary));

        // Content[1]: Machine-Parseable JSON
        let metadata = json!({
            "issues": issues,
            "count": issues.len()
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I list and filter GitHub issues?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the list_issues tool to list and filter repository issues:\n\n\
                     List all open issues:\n\
                     list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\"})\n\n\
                     Filter by state:\n\
                     list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"state\": \"closed\"})\n\n\
                     Filter by labels (multiple labels = AND logic):\n\
                     list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"labels\": [\"bug\", \"priority-high\"]})\n\n\
                     Filter by assignee:\n\
                     list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"assignee\": \"octocat\"})\n\n\
                     With pagination:\n\
                     list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"per_page\": 50, \"page\": 2})\n\n\
                     Combined filters:\n\
                     list_issues({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\",\n\
                       \"labels\": [\"bug\"],\n\
                       \"per_page\": 20\n\
                     })\n\n\
                     Filter options:\n\
                     - state: \"open\" (default), \"closed\", or \"all\"\n\
                     - labels: Array of label names (matches issues with ALL labels)\n\
                     - assignee: Username of assigned user\n\
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
