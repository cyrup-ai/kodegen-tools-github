use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{CreateRepositoryArgs, CreateRepositoryPrompts, GITHUB_CREATE_REPOSITORY};

use crate::GitHubClient;

/// Tool for creating a new repository
pub struct CreateRepositoryTool;

impl Tool for CreateRepositoryTool {
    type Args = CreateRepositoryArgs;
    type Prompts = CreateRepositoryPrompts;

    fn name() -> &'static str {
        GITHUB_CREATE_REPOSITORY
    }

    fn description() -> &'static str {
        "Create a new repository under the authenticated user's account"
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
            .create_repository(args.name.clone(), args.description.clone(), args.private, args.auto_init)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let repository =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Extract fields from octocrab repository
        let owner = repository.owner.as_ref()
            .map(|o| o.login.clone())
            .unwrap_or_default();

        let full_name = repository.full_name
            .as_deref()
            .unwrap_or(&args.name)
            .to_string();

        let html_url = repository.html_url.as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();

        let clone_url = repository.clone_url.as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubCreateRepoOutput {
            success: true,
            owner: owner.clone(),
            name: args.name.clone(),
            full_name: full_name.clone(),
            html_url: html_url.clone(),
            clone_url: clone_url.clone(),
            message: format!("Repository '{}' created successfully", args.name),
        };

        // Build human-readable display
        let display = format!(
            "✅ Repository Created\n\n\
             Name: {}\n\
             Owner: {}\n\
             Full Name: {}\n\
             URL: {}\n\
             Clone: {}",
            output.name, output.owner, output.full_name, output.html_url, output.clone_url
        );

        // Return ToolResponse wrapper
        Ok(ToolResponse::new(display, output))
    }
}
