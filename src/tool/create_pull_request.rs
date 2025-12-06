use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{
    CreatePullRequestArgs, CreatePullRequestPrompts, GitHubCreatePrOutput, GITHUB_CREATE_PULL_REQUEST,
};

use crate::GitHubClient;
use crate::github::CreatePullRequestRequest;

/// Tool for creating a new pull request in a GitHub repository
pub struct CreatePullRequestTool;

impl Tool for CreatePullRequestTool {
    type Args = CreatePullRequestArgs;
    type Prompts = CreatePullRequestPrompts;

    fn name() -> &'static str {
        GITHUB_CREATE_PULL_REQUEST
    }

    fn description() -> &'static str {
        "Create a new pull request in a GitHub repository"
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false
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

        let request = CreatePullRequestRequest {
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            title: args.title.clone(),
            body: args.body.clone(),
            head: args.head.clone(),
            base: args.base.clone(),
            draft: args.draft,
            maintainer_can_modify: args.maintainer_can_modify,
        };

        let task_result = client.create_pull_request(request).await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let pr =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        let html_url = pr.html_url
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();

        let output = GitHubCreatePrOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: pr.number,
            html_url: html_url.clone(),
            message: format!("Pull request #{} created successfully", pr.number),
        };

        let display = format!(
            "Successfully created Pull Request #{} in {}/{}\n\
            Title: {}\n\
            Base: {} <- Head: {}\n\
            URL: {}\n\
            Status: {}",
            pr.number,
            args.owner,
            args.repo,
            args.title,
            args.base,
            args.head,
            html_url,
            if args.draft.unwrap_or(false) { "Draft" } else { "Ready for review" }
        );

        Ok(ToolResponse::new(display, output))
    }
}
