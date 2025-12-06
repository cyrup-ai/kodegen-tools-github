use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{ForkRepositoryArgs, ForkRepositoryPrompts, GITHUB_FORK_REPOSITORY};

use crate::GitHubClient;

/// Tool for forking a repository
pub struct ForkRepositoryTool;

impl Tool for ForkRepositoryTool {
    type Args = ForkRepositoryArgs;
    type Prompts = ForkRepositoryPrompts;

    fn name() -> &'static str {
        GITHUB_FORK_REPOSITORY
    }

    fn description() -> &'static str {
        "Fork a repository to your account or an organization"
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

        let task_result = client
            .fork_repository(args.owner.clone(), args.repo.clone(), args.organization.clone())
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let repository =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Extract forked repository details
        let forked_owner = repository.owner.as_ref()
            .map(|o| o.login.clone())
            .unwrap_or_default();

        let forked_name = repository.name.clone();

        let forked_full_name = repository.full_name
            .as_deref()
            .unwrap_or_default()
            .to_string();

        let html_url = repository.html_url.as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubForkRepoOutput {
            success: true,
            source_owner: args.owner.clone(),
            source_repo: args.repo.clone(),
            forked_owner: forked_owner.clone(),
            forked_name: forked_name.clone(),
            forked_full_name: forked_full_name.clone(),
            html_url: html_url.clone(),
            message: format!("Forked {}/{} successfully", args.owner, args.repo),
        };

        // Build human-readable display
        let display = format!(
            "🍴 Repository Forked\n\n\
             Source: {}/{}\n\
             Forked To: {}\n\
             URL: {}",
            output.source_owner, output.source_repo, output.forked_full_name, output.html_url
        );

        // Return ToolResponse wrapper
        Ok(ToolResponse::new(display, output))
    }
}
