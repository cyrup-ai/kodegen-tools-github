use anyhow;
use kodegen_mcp_schema::github::{GetPullRequestStatusArgs, GetPullRequestStatusPrompts, GITHUB_GET_PULL_REQUEST_STATUS};
use kodegen_mcp_schema::ToolArgs;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};

use crate::GitHubClient;

/// Tool for getting detailed status information about a pull request
pub struct GetPullRequestStatusTool;

impl Tool for GetPullRequestStatusTool {
    type Args = GetPullRequestStatusArgs;
    type Prompts = GetPullRequestStatusPrompts;

    fn name() -> &'static str {
        GITHUB_GET_PULL_REQUEST_STATUS
    }

    fn description() -> &'static str {
        "Get detailed status information about a pull request including merge status, checks, and review state"
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
        true
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as ToolArgs>::Output>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;
        let task_result = client
            .get_pull_request_status(args.owner.clone(), args.repo.clone(), args.pr_number)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let status =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Map state to lowercase string
        let state_str = match status.pr.state {
            Some(octocrab::models::IssueState::Open) => "open",
            Some(octocrab::models::IssueState::Closed) => "closed",
            _ => "unknown",
        }.to_string();

        // Get mergeable status
        let mergeable = status.pr.mergeable;

        // Map mergeable_state to check status
        let checks_status = match &status.pr.mergeable_state {
            Some(octocrab::models::pulls::MergeableState::Clean) => "pass",
            Some(octocrab::models::pulls::MergeableState::Unstable) => "pass",
            Some(octocrab::models::pulls::MergeableState::HasHooks) => "pass",
            Some(octocrab::models::pulls::MergeableState::Dirty) => "fail",
            Some(octocrab::models::pulls::MergeableState::Blocked) => "pending",
            Some(octocrab::models::pulls::MergeableState::Behind) => "pending",
            Some(octocrab::models::pulls::MergeableState::Draft) => "pending",
            _ => "pending",
        }.to_string();

        // Calculate check counts from combined_status
        let statuses = &status.combined_status.statuses;
        let checks_passed = statuses.iter()
            .filter(|s| s.state == octocrab::models::StatusState::Success)
            .count() as u32;
        let checks_failed = statuses.iter()
            .filter(|s| s.state == octocrab::models::StatusState::Failure || s.state == octocrab::models::StatusState::Error)
            .count() as u32;
        let checks_count = statuses.len() as u32;

        let output = kodegen_mcp_schema::github::GitHubGetPrStatusOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pr_number,
            state: state_str.clone(),
            mergeable,
            checks_status: checks_status.clone(),
            checks_count,
            checks_passed,
            checks_failed,
        };

        let display = format!(
            "🔄 PR #{} Status: {}/{}\n\n\
             State: {}\n\
             Mergeable: {}\n\
             Checks: {} total ({} ✅ / {} ❌)\n\
             Overall Status: {}",
            output.pr_number,
            output.owner,
            output.repo,
            output.state,
            output.mergeable.map(|m| if m { "Yes" } else { "No" }).unwrap_or("Unknown"),
            output.checks_count,
            output.checks_passed,
            output.checks_failed,
            output.checks_status
        );

        Ok(ToolResponse::new(display, output))
    }
}
