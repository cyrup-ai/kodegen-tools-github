//! GitHub issue update tool

use anyhow;
use kodegen_mcp_schema::github::{
    UpdateIssueArgs, UpdateIssuePrompts, GitHubUpdateIssueOutput, GITHUB_UPDATE_ISSUE,
};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

use crate::github::UpdateIssueRequest;

/// Tool for updating GitHub issues
#[derive(Clone)]
pub struct UpdateIssueTool;

impl Tool for UpdateIssueTool {
    type Args = UpdateIssueArgs;
    type Prompts = UpdateIssuePrompts;

    fn name() -> &'static str {
        GITHUB_UPDATE_ISSUE
    }

    fn description() -> &'static str {
        "Update an existing GitHub issue. Supports partial updates - only specified fields \
         will be modified. Can update title, body, state (open/closed), labels, and assignees. \
         Requires GITHUB_TOKEN environment variable with write access."
    }

    fn read_only() -> bool {
        false // Modifies data
    }

    fn destructive() -> bool {
        false // Modifies, doesn't delete
    }

    fn idempotent() -> bool {
        false // Multiple updates may differ
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
        let state = args
            .state
            .as_ref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "open" => Some(octocrab::models::IssueState::Open),
                "closed" => Some(octocrab::models::IssueState::Closed),
                _ => None,
            });

        // Build request
        let request = UpdateIssueRequest {
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            issue_number: args.issue_number,
            title: args.title.clone(),
            body: args.body.clone(),
            state,
            labels: args.labels.clone(),
            assignees: args.assignees.clone(),
            milestone: None,
        };

        // Call API wrapper (returns AsyncTask<Result<Issue, GitHubError>>)
        // The .await returns Result<Result<Issue, GitHubError>, RecvError>
        let task_result = client.update_issue(request).await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let issue =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build message based on what was updated
        let state_str = match issue.state {
            octocrab::models::IssueState::Open => "open",
            octocrab::models::IssueState::Closed => "closed",
            _ => "unknown",
        };

        let message = format!("Issue #{} updated successfully (state: {})", issue.number, state_str);

        let output = GitHubUpdateIssueOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            issue_number: args.issue_number,
            message,
        };

        // Build display string
        let mut updates = Vec::new();
        if args.title.is_some() {
            updates.push("title");
        }
        if args.body.is_some() {
            updates.push("body");
        }
        if args.state.is_some() {
            updates.push(format!("state ({})", state_str).leak() as &str);
        }
        if args.labels.is_some() {
            updates.push("labels");
        }
        if args.assignees.is_some() {
            updates.push("assignees");
        }

        let updates_str = if updates.is_empty() {
            "no changes".to_string()
        } else {
            updates.join(", ")
        };

        let display = format!(
            "Successfully updated issue #{} in {}/{}\nUpdated: {}\nCurrent state: {}",
            args.issue_number, args.owner, args.repo, updates_str, state_str
        );

        Ok(ToolResponse::new(display, output))
    }
}
