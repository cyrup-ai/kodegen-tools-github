use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{DeleteBranchArgs, DeleteBranchPrompts, GITHUB_DELETE_BRANCH};

use crate::GitHubClient;

/// Tool for deleting a branch
pub struct DeleteBranchTool;

impl Tool for DeleteBranchTool {
    type Args = DeleteBranchArgs;
    type Prompts = DeleteBranchPrompts;

    fn name() -> &'static str {
        GITHUB_DELETE_BRANCH
    }

    fn description() -> &'static str {
        "Delete a branch from a GitHub repository"
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true
    }

    fn idempotent() -> bool {
        false
    }

    fn open_world() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) 
        -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> 
    {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let task_result = client
            .delete_branch(args.owner.clone(), args.repo.clone(), args.branch_name.clone())
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        let output = kodegen_mcp_schema::github::GitHubDeleteBranchOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            branch_name: args.branch_name.clone(),
            message: format!("Branch '{}' deleted successfully", args.branch_name),
        };

        let display = format!(
            "🗑️  Branch Deleted\n\n\
             Repository: {}/{}\n\
             Branch: {}",
            output.owner,
            output.repo,
            output.branch_name
        );

        Ok(ToolResponse::new(display, output))
    }
}
