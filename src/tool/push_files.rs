//! GitHub multiple files push tool

use anyhow;
use kodegen_mcp_schema::github::{
    PushFilesArgs,
    PushFilesPromptArgs,
    GitHubPushFilesOutput,
    GITHUB_PUSH_FILES
};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};

/// Tool for pushing multiple files to a GitHub repository in a single commit
#[derive(Clone)]
pub struct PushFilesTool;

impl Tool for PushFilesTool {
    type Args = PushFilesArgs;
    type PromptArgs = PushFilesPromptArgs;
    
    fn name() -> &'static str {
        GITHUB_PUSH_FILES
    }
    
    fn description() -> &'static str {
        "Push multiple files to a GitHub repository in a single commit. All files \
         are added atomically (creates tree, commit, and updates ref). File content \
         must be base64-encoded. Requires GITHUB_TOKEN environment variable."
    }
    
    fn read_only() -> bool {
        false  // Modifies data
    }
    
    fn destructive() -> bool {
        false  // Creates, doesn't delete
    }
    
    fn idempotent() -> bool {
        false  // Multiple calls create multiple commits
    }
    
    fn open_world() -> bool {
        true  // Calls external GitHub API
    }
    
    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| McpError::Other(anyhow::anyhow!(
                "GITHUB_TOKEN environment variable not set"
            )))?;
        
        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {}", e)))?;
        
        let file_count = args.files.len();
        let file_paths: Vec<String> = args.files.keys().cloned().collect();
        
        // Note: The API wrapper expects base64-encoded content in the HashMap
        // The args.files should already be base64-encoded by the caller
        // Call API wrapper (returns AsyncTask<Result<Commit, GitHubError>>)
        let task_result = client.push_files(
            args.owner.clone(),
            args.repo.clone(),
            args.branch.clone(),
            args.files,
            args.message.clone(),
        ).await;
        
        // Handle outer Result (channel error)
        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;
        
        // Handle inner Result (GitHub API error)
        let commit = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;

        // Build human-readable summary
        let file_list_preview = file_paths
            .iter()
            .take(5)
            .map(|p| format!("  📄 {}", p))
            .collect::<Vec<_>>()
            .join("\n");

        let more_indicator = if file_paths.len() > 5 {
            format!("\n  ... and {} more files", file_paths.len() - 5)
        } else {
            String::new()
        };

        let commit_sha = commit.sha.as_deref().unwrap_or("N/A");

        let summary = format!(
            "📦 Pushed {} file(s) to {}\n\n\
             Repository: {}/{}\n\
             Branch: {}\n\
             Commit: \"{}\"\n\
             Commit SHA: {}\n\n\
             Files:\n{}{}",
            file_count,
            args.branch,
            args.owner,
            args.repo,
            args.branch,
            args.message,
            commit_sha,
            file_list_preview,
            more_indicator
        );

        // Build typed output
        let output = GitHubPushFilesOutput {
            success: true,
            owner: args.owner,
            repo: args.repo,
            branch: args.branch,
            message: args.message,
            file_count,
            file_paths,
            commit_sha: commit.sha.unwrap_or_default(),
            commit_url: commit.html_url.unwrap_or_default(),
        };

        Ok(ToolResponse::new(summary, output))
    }
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "use_case".to_string(),
                title: None,
                description: Some(
                    "Optional use case to focus examples on: 'bulk_setup' (initial repo setup), \
                     'binary_files' (images, binaries), or 'encoding' (base64 workflows)"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "focus".to_string(),
                title: None,
                description: Some(
                    "Optional feature to focus on: 'atomicity' (transaction guarantees), \
                     'base64' (encoding/decoding), or 'permissions' (GitHub token scopes)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }
    
    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I push multiple files to GitHub at once?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use push_files to commit multiple files atomically:\n\n\
                     push_files({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"branch\": \"main\",\n\
                       \"message\": \"Add multiple files\",\n\
                       \"files\": {\n\
                         \"src/file1.rs\": \"ZnVuIG1haW4oKSB7fQ==\",  // base64 of content\n\
                         \"src/file2.rs\": \"ZnVuIGhlbHBlcigpIHt9\",\n\
                         \"README.md\": \"IyBQcm9qZWN0\"\n\
                       }\n\
                     })\n\n\
                     Important:\n\
                     - All file content MUST be base64-encoded\n\
                     - All files are added in a SINGLE commit\n\
                     - Creates tree, commit, and updates ref atomically\n\
                     - More efficient than multiple create_or_update_file calls\n\
                     - Good for bulk operations or initial repo setup\n\n\
                     To encode content to base64:\n\
                     - In JavaScript: Buffer.from(text).toString('base64')\n\
                     - In Python: base64.b64encode(text.encode()).decode()\n\
                     - In Rust: use base64 crate\n\n\
                     Workflow:\n\
                     1. Prepare all file contents\n\
                     2. Base64-encode each file's content\n\
                     3. Create a map of {path: base64_content}\n\
                     4. Call push_files with the map\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos, 'public_repo' for public\n\
                     - User must have write access to the repository\n\
                     - Branch must already exist"
                ),
            },
        ])
    }
}
