use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{
    CreateOrUpdateFileArgs, 
    CreateOrUpdateFilePrompts,
    GitHubCreateOrUpdateFileOutput,
    GITHUB_CREATE_OR_UPDATE_FILE
};
use anyhow;

use crate::GitHubClient;
use crate::github::CreateOrUpdateFileRequest;

/// Tool for creating a new file or updating an existing file
pub struct CreateOrUpdateFileTool;

impl Tool for CreateOrUpdateFileTool {
    type Args = CreateOrUpdateFileArgs;
    type Prompts = CreateOrUpdateFilePrompts;

    fn name() -> &'static str {
        GITHUB_CREATE_OR_UPDATE_FILE
    }

    fn description() -> &'static str {
        "Create a new file or update an existing file in a GitHub repository"
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
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| McpError::Other(anyhow::anyhow!(
                "GITHUB_TOKEN environment variable not set"
            )))?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {}", e)))?;

        let request = CreateOrUpdateFileRequest {
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            path: args.path.clone(),
            message: args.message.clone(),
            content: args.content.clone(),
            branch: args.branch.clone(),
            sha: args.sha.clone(),
        };

        let task_result = client
            .create_or_update_file(request)
            .await;

        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;

        let file_update = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;

        // Determine operation type
        let operation = if args.sha.is_some() { "updated" } else { "created" };
        let emoji = if args.sha.is_some() { "✏️" } else { "✨" };
        
        // Extract commit SHA
        let commit_sha = file_update.commit
            .as_ref()
            .and_then(|c| c.sha.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("N/A");
        
        // Extract file SHA
        let file_sha = file_update.content
            .sha
            .clone();
        
        // Extract HTML URL
        let html_url = file_update.content
            .html_url
            .clone()
            .unwrap_or_default();
        
        // Build display
        let content_preview = if args.content.len() > 200 {
            format!("{}...\n\n({} bytes total)", &args.content[..200], args.content.len())
        } else {
            args.content.clone()
        };
        
        let branch_info = args.branch
            .as_ref()
            .map(|b| format!("\nBranch: {}", b))
            .unwrap_or_else(|| "\nBranch: default".to_string());
        
        let display = format!(
            "{} File {}\n\n\
             Path: {}\n\
             Repository: {}/{}{}\n\
             Commit: \"{}\"\n\
             Commit SHA: {}\n\
             File SHA: {}\n\
             URL: {}\n\n\
             Content Preview:\n\
             {}",
            emoji,
            operation,
            args.path,
            args.owner,
            args.repo,
            branch_info,
            args.message,
            &commit_sha[..7],
            &file_sha[..7],
            html_url,
            content_preview
        );
        
        // Build typed output
        let output = GitHubCreateOrUpdateFileOutput {
            success: true,
            owner: args.owner,
            repo: args.repo,
            path: args.path,
            sha: file_sha,
            commit_sha: commit_sha.to_string(),
            commit_message: args.message,
            html_url,
            operation: operation.to_string(),
        };
        
        Ok(ToolResponse::new(display, output))
    }
}
