//! GitHub issue retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{GetIssueArgs, GetIssuePromptArgs, GITHUB_GET_ISSUE};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for fetching a GitHub issue by number
#[derive(Clone)]
pub struct GetIssueTool;

impl Tool for GetIssueTool {
    type Args = GetIssueArgs;
    type PromptArgs = GetIssuePromptArgs;

    fn name() -> &'static str {
        GITHUB_GET_ISSUE
    }

    fn description() -> &'static str {
        "Fetch a single GitHub issue by number. Returns detailed issue information including \
         title, body, state, labels, assignees, comments count, and timestamps. \
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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Clone values before moving them
        let owner = args.owner.clone();
        let repo = args.repo.clone();

        // Call API wrapper (returns AsyncTask<Result<Issue, GitHubError>>)
        // The .await returns Result<Result<Issue, GitHubError>, RecvError>
        let task_result = client
            .get_issue(args.owner, args.repo, args.issue_number)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let issue =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build dual-content response
        let mut contents = Vec::new();

        // Content[0]: Human-Readable Summary
        let labels_str = issue.labels.iter()
            .map(|l| l.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        
        let assignees_str = issue.assignees.iter()
            .map(|a| format!("@{}", a.login))
            .collect::<Vec<_>>()
            .join(", ");
        
        let state_emoji = match issue.state {
            octocrab::models::IssueState::Open => "🟢",
            octocrab::models::IssueState::Closed => "🔴",
            _ => "⚪",
        };
        
        let state_str = match issue.state {
            octocrab::models::IssueState::Open => "open",
            octocrab::models::IssueState::Closed => "closed",
            _ => "unknown",
        };

        let summary = format!(
            "🔍 Issue #{}: {}\n\n\
             Repository: {}/{}\n\
             State: {} {}\n\
             Author: @{}\n\
             Created: {}\n\
             Comments: {}\n\n\
             Labels: {}\n\
             Assignees: {}\n\n\
             View on GitHub: {}",
            issue.number,
            issue.title,
            owner,
            repo,
            state_emoji,
            state_str,
            issue.user.login,
            issue.created_at.format("%Y-%m-%d"),
            issue.comments,
            if labels_str.is_empty() { "none" } else { &labels_str },
            if assignees_str.is_empty() { "none" } else { &assignees_str },
            issue.html_url
        );
        contents.push(Content::text(summary));

        // Content[1]: Machine-Parseable JSON
        let json_str = serde_json::to_string_pretty(&issue)
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
                content: PromptMessageContent::text("How do I fetch a specific GitHub issue?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the get_issue tool to fetch a GitHub issue by its number:\n\n\
                     Basic usage:\n\
                     get_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42})\n\n\
                     The returned issue object includes:\n\
                     - number: Issue number\n\
                     - title: Issue title\n\
                     - body: Issue description\n\
                     - state: \"open\" or \"closed\"\n\
                     - labels: Array of label objects\n\
                     - assignees: Array of assigned users\n\
                     - created_at: Creation timestamp\n\
                     - updated_at: Last update timestamp\n\
                     - comments: Number of comments\n\
                     - html_url: Link to issue on GitHub\n\n\
                     Important notes:\n\
                     - issue_number is the issue number (e.g., #42), NOT the internal ID\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos, 'public_repo' for public\n\
                     - Works for both issues and pull requests (PRs are issues with pull_request field)",
                ),
            },
        ])
    }
}
