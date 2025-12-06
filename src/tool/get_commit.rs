use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{GetCommitArgs, GetCommitPrompts, GITHUB_GET_COMMIT};

use crate::GitHubClient;

/// Tool for getting detailed commit information
pub struct GetCommitTool;

impl Tool for GetCommitTool {
    type Args = GetCommitArgs;
    type Prompts = GetCommitPrompts;

    fn name() -> &'static str {
        GITHUB_GET_COMMIT
    }

    fn description() -> &'static str {
        "Get detailed information about a specific commit"
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
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let task_result = client
            .get_commit(
                args.owner.clone(),
                args.repo.clone(),
                args.commit_sha.clone(),
                args.page,
                args.per_page,
            )
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let commit =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Convert octocrab commit to typed output
        let author_name = commit.commit.author.as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default();

        let author_email = commit.commit.author.as_ref()
            .and_then(|a| a.email.clone())
            .unwrap_or_default();

        let committer_name = commit.commit.committer.as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_default();

        let committer_email = commit.commit.committer.as_ref()
            .and_then(|c| c.email.clone())
            .unwrap_or_default();

        let author_date = commit.commit.author.as_ref()
            .and_then(|a| a.date.as_ref())
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();

        let commit_date = commit.commit.committer.as_ref()
            .and_then(|c| c.date.as_ref())
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();

        let parents: Vec<String> = commit.parents
            .iter()
            .filter_map(|p| p.sha.clone())
            .collect();

        let stats = commit.stats.as_ref().map(|s| {
            kodegen_mcp_schema::github::GitHubCommitStats {
                additions: s.additions.unwrap_or(0) as u32,
                deletions: s.deletions.unwrap_or(0) as u32,
                total: s.total.unwrap_or(0) as u32,
            }
        });

        let files: Vec<kodegen_mcp_schema::github::GitHubCommitFile> = commit.files
            .as_ref()
            .map(|files| {
                files.iter().map(|f| {
                    // Convert DiffEntryStatus to string using serde serialization
                    let status = serde_json::to_value(&f.status)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| format!("{:?}", f.status));
                    
                    kodegen_mcp_schema::github::GitHubCommitFile {
                        filename: f.filename.clone(),
                        status,
                        additions: f.additions as u32,
                        deletions: f.deletions as u32,
                        changes: f.changes as u32,
                        patch: f.patch.clone(),
                    }
                }).collect()
            })
            .unwrap_or_default();

        let commit_detail = kodegen_mcp_schema::github::GitHubCommitDetail {
            sha: commit.sha.clone(),
            message: commit.commit.message.clone(),
            author_name,
            author_email,
            committer_name,
            committer_email,
            author_date,
            commit_date,
            parents,
            html_url: commit.html_url.to_string(),
            stats,
            files,
        };

        let files_changed = commit_detail.files.len();
        let additions = commit_detail.stats.as_ref().map(|s| s.additions).unwrap_or(0);
        let deletions = commit_detail.stats.as_ref().map(|s| s.deletions).unwrap_or(0);

        let display = format!(
            "📝 Commit Details\n\n\
             SHA: {}\n\
             Author: {} <{}>\n\
             Date: {}\n\
             Message: {}\n\n\
             Files Changed: {}\n\
             +{} -{} total changes",
            commit_detail.sha,
            commit_detail.author_name,
            commit_detail.author_email,
            commit_detail.author_date,
            commit_detail.message,
            files_changed,
            additions,
            deletions
        );

        let output = kodegen_mcp_schema::github::GitHubGetCommitOutput {
            success: true,
            owner: args.owner,
            repo: args.repo,
            commit: commit_detail,
        };

        Ok(ToolResponse::new(display, output))
    }
}
