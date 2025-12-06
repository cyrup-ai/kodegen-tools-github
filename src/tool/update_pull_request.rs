use anyhow;
use kodegen_mcp_schema::github::{
    UpdatePullRequestArgs, UpdatePullRequestPrompts, GitHubUpdatePrOutput,
    GITHUB_UPDATE_PULL_REQUEST,
};
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};

use crate::GitHubClient;

/// Tool for updating an existing pull request
pub struct UpdatePullRequestTool;

impl Tool for UpdatePullRequestTool {
    type Args = UpdatePullRequestArgs;
    type Prompts = UpdatePullRequestPrompts;

    fn name() -> &'static str {
        GITHUB_UPDATE_PULL_REQUEST
    }

    fn description() -> &'static str {
        "Update an existing pull request in a GitHub repository"
    }

    fn read_only() -> bool {
        false
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Convert state string to octocrab State enum
        let state = args
            .state
            .as_ref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "open" => Some(octocrab::params::pulls::State::Open),
                "closed" => Some(octocrab::params::pulls::State::Closed),
                _ => None,
            });

        let options = crate::UpdatePullRequestOptions {
            title: args.title.clone(),
            body: args.body.clone(),
            state,
            base: args.base.clone(),
            maintainer_can_modify: args.maintainer_can_modify,
        };

        let task_result = client
            .update_pull_request(args.owner.clone(), args.repo.clone(), args.pr_number, options)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let pr =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Format state
        let state_str = pr.state.as_ref()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "unknown".to_string());

        let output = GitHubUpdatePrOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pr_number,
            message: format!("Pull request #{} updated successfully (state: {})", pr.number, state_str),
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
            updates.push("state");
        }
        if args.base.is_some() {
            updates.push("base branch");
        }
        if args.maintainer_can_modify.is_some() {
            updates.push("maintainer permissions");
        }

        let updates_str = if updates.is_empty() {
            "metadata".to_string()
        } else {
            updates.join(", ")
        };

        let display = format!(
            "Successfully updated pull request #{} in {}/{}\n\
             Updated: {}\n\
             Current state: {}",
            args.pr_number,
            args.owner,
            args.repo,
            updates_str,
            state_str
        );

        Ok(ToolResponse::new(display, output))
    }
}
