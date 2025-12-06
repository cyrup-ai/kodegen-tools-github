use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{
    GetFileContentsArgs, 
    GetFileContentsPrompts,
    GitHubGetFileContentsOutput,
    GitHubFileContent,
    GitHubDirectoryEntry,
    GITHUB_GET_FILE_CONTENTS
};
use anyhow;

use crate::GitHubClient;

/// Tool for getting file or directory contents from a GitHub repository
pub struct GetFileContentsTool;

impl Tool for GetFileContentsTool {
    type Args = GetFileContentsArgs;
    type Prompts = GetFileContentsPrompts;

    fn name() -> &'static str {
        GITHUB_GET_FILE_CONTENTS
    }

    fn description() -> &'static str {
        "Get file or directory contents from a GitHub repository"
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

        let task_result = client
            .get_file_contents(
                args.owner.clone(),
                args.repo.clone(),
                args.path.clone(),
                args.ref_name.clone(),
            )
            .await;

        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;

        let content_vec = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;

        // Determine if file or directory based on response structure
        if content_vec.len() == 1 && content_vec[0].r#type == "file" {
            // SINGLE FILE CASE
            let file = &content_vec[0];
            
            // Decode base64 content
            let content = file.decoded_content().unwrap_or_default();
            
            // Build display
            let content_preview = if content.len() > 500 {
                format!("{}...\n\n(Content truncated - {} bytes total)", 
                    &content[..500], content.len())
            } else {
                content.clone()
            };
            
            let display = format!(
                "📄 File: {}\n\
                 Repository: {}/{}\n\
                 Ref: {}\n\
                 Size: {} bytes\n\
                 SHA: {}\n\n\
                 Content:\n\
                 {}",
                args.path,
                args.owner,
                args.repo,
                args.ref_name.as_deref().unwrap_or("default branch"),
                file.size,
                &file.sha[..7],
                content_preview
            );
            
            // Build typed output
            let output = GitHubGetFileContentsOutput {
                success: true,
                owner: args.owner,
                repo: args.repo,
                path: args.path,
                ref_name: args.ref_name,
                content_type: "file".to_string(),
                file_content: Some(GitHubFileContent {
                    name: file.name.clone(),
                    path: file.path.clone(),
                    sha: file.sha.clone(),
                    size: file.size as u64,
                    content,
                    encoding: file.encoding.clone().unwrap_or_default(),
                    html_url: file.html_url.clone().unwrap_or_default(),
                    git_url: file.git_url.clone().unwrap_or_default(),
                    download_url: file.download_url.clone(),
                }),
                directory_contents: None,
            };
            
            Ok(ToolResponse::new(display, output))
            
        } else {
            // DIRECTORY CASE (multiple items)
            let entries: Vec<GitHubDirectoryEntry> = content_vec.iter().map(|entry| {
                GitHubDirectoryEntry {
                    name: entry.name.clone(),
                    path: entry.path.clone(),
                    sha: entry.sha.clone(),
                    size: entry.size as u64,
                    entry_type: entry.r#type.clone(),
                    html_url: entry.html_url.clone().unwrap_or_default(),
                }
            }).collect();
            
            // Build display
            let items_preview = entries.iter()
                .take(20)
                .map(|e| {
                    let icon = match e.entry_type.as_str() {
                        "dir" => "📁",
                        "file" => "📄",
                        _ => "🔗"
                    };
                    format!("  {} {}", icon, e.name)
                })
                .collect::<Vec<_>>()
                .join("\n");
            
            let more_indicator = if entries.len() > 20 {
                format!("\n  ... and {} more items", entries.len() - 20)
            } else {
                String::new()
            };
            
            let display = format!(
                "📁 Directory: {}\n\
                 Repository: {}/{}\n\
                 Ref: {}\n\
                 Total Items: {}\n\n\
                 Contents:\n\
                 {}{}",
                args.path,
                args.owner,
                args.repo,
                args.ref_name.as_deref().unwrap_or("default branch"),
                entries.len(),
                items_preview,
                more_indicator
            );
            
            // Build typed output
            let output = GitHubGetFileContentsOutput {
                success: true,
                owner: args.owner,
                repo: args.repo,
                path: args.path,
                ref_name: args.ref_name,
                content_type: "directory".to_string(),
                file_content: None,
                directory_contents: Some(entries),
            };
            
            Ok(ToolResponse::new(display, output))
        }
    }
}
