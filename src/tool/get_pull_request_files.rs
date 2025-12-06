use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{GetPullRequestFilesArgs, GetPullRequestFilesPrompts, GITHUB_GET_PULL_REQUEST_FILES};
use serde_json;

use crate::GitHubClient;

/// Tool for getting all files changed in a pull request
pub struct GetPullRequestFilesTool;

impl Tool for GetPullRequestFilesTool {
    type Args = GetPullRequestFilesArgs;
    type Prompts = GetPullRequestFilesPrompts;

    fn name() -> &'static str {
        GITHUB_GET_PULL_REQUEST_FILES
    }

    fn description() -> &'static str {
        "Get all files changed in a pull request with their diff stats"
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
        -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Clone owner and repo once for reuse
        let owner = args.owner.clone();
        let repo = args.repo.clone();

        let mut file_stream = client.get_pull_request_files(owner.clone(), repo.clone(), args.pr_number);

        let mut files = Vec::new();
        while let Some(result) = file_stream.next().await {
            let file =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            files.push(file);
        }

        // Convert octocrab files to typed output
        let pr_files: Vec<kodegen_mcp_schema::github::GitHubPrFile> = files
            .iter()
            .map(|f| {
                // Convert DiffEntryStatus to string using serde serialization
                let status = serde_json::to_value(&f.status)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| format!("{:?}", f.status));
                
                kodegen_mcp_schema::github::GitHubPrFile {
                    filename: f.filename.clone(),
                    status,
                    additions: f.additions as u32,
                    deletions: f.deletions as u32,
                    changes: f.changes as u32,
                    patch: f.patch.clone(),
                }
            })
            .collect();

        let count = pr_files.len();
        let total_additions: u32 = pr_files.iter().map(|f| f.additions).sum();
        let total_deletions: u32 = pr_files.iter().map(|f| f.deletions).sum();

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubGetPrFilesOutput {
            success: true,
            owner,
            repo: repo.clone(),
            pr_number: args.pr_number,
            count,
            files: pr_files.clone(),
        };

        // Build human-readable display
        let display = format!(
            "📄 PR #{} Files: {}/{}\n\n\
             {} files changed\n\
             +{} additions / -{} deletions\n\n\
             {}",
            args.pr_number,
            output.owner,
            repo,
            count,
            total_additions,
            total_deletions,
            pr_files.iter()
                .map(|f| format!("  • {} [{}] (+{} -{})", 
                    f.filename, f.status, f.additions, f.deletions))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(ToolResponse::new(display, output))
    }
}
