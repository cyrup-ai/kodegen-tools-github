use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{CreateBranchArgs, CreateBranchPrompts, GITHUB_CREATE_BRANCH};

use crate::GitHubClient;

/// Tool for creating a new branch
pub struct CreateBranchTool;

impl Tool for CreateBranchTool {
    type Args = CreateBranchArgs;
    type Prompts = CreateBranchPrompts;

    fn name() -> &'static str {
        GITHUB_CREATE_BRANCH
    }

    fn description() -> &'static str {
        "Create a new branch from a commit SHA"
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
            .create_branch(args.owner.clone(), args.repo.clone(), args.branch_name.clone(), args.sha.clone())
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let reference =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Extract SHA from Object enum
        let sha = match &reference.object {
            octocrab::models::repos::Object::Commit { sha, .. } => sha.clone(),
            octocrab::models::repos::Object::Tag { sha, .. } => sha.clone(),
            _ => return Err(McpError::Other(anyhow::anyhow!("Unexpected object type"))),
        };

        let output = kodegen_mcp_schema::github::GitHubCreateBranchOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            branch_name: args.branch_name.clone(),
            sha: sha.clone(),
            message: format!("Branch '{}' created from {}", args.branch_name, args.sha),
        };

        let display = format!(
            "✅ Branch Created\n\n\
             Repository: {}/{}\n\
             Branch: {}\n\
             From: {}\n\
             SHA: {}",
            output.owner,
            output.repo,
            output.branch_name,
            args.sha,
            output.sha
        );

        Ok(ToolResponse::new(display, output))
    }
}
