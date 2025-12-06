use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{ListCommitsArgs, ListCommitsPrompts, GITHUB_LIST_COMMITS};

use crate::GitHubClient;

/// Tool for listing repository commits
pub struct ListCommitsTool;

impl Tool for ListCommitsTool {
    type Args = ListCommitsArgs;
    type Prompts = ListCommitsPrompts;

    fn name() -> &'static str {
        GITHUB_LIST_COMMITS
    }

    fn description() -> &'static str {
        "List commits in a repository with filtering options"
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

        // Convert Args to ListCommitsOptions
        let options = crate::github::ListCommitsOptions {
            sha: args.sha.clone(),
            path: args.path.clone(),
            author: args.author.clone(),
            since: args.since.clone(),
            until: args.until.clone(),
            page: args.page,
            per_page: args.per_page,
        };

        let task_result = client.list_commits(args.owner.clone(), args.repo.clone(), options).await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let commits =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Convert octocrab commits to typed output
        let commit_summaries: Vec<kodegen_mcp_schema::github::GitHubCommitSummary> = commits
            .iter()
            .map(|c| {
                let author_name = c.commit.author.as_ref()
                    .map(|a| a.name.clone())
                    .unwrap_or_default();

                let author_email = c.commit.author.as_ref()
                    .and_then(|a| a.email.clone())
                    .unwrap_or_default();

                let date = c.commit.author.as_ref()
                    .and_then(|a| a.date.as_ref())
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_default();

                kodegen_mcp_schema::github::GitHubCommitSummary {
                    sha: c.sha.clone(),
                    message: c.commit.message.clone(),
                    author_name,
                    author_email,
                    date,
                    html_url: c.html_url.to_string(),
                }
            })
            .collect();

        let count = commit_summaries.len();

        // Build human-readable display with emoji
        let preview_commits = commit_summaries
            .iter()
            .take(10)
            .map(|c| {
                let short_sha = &c.sha[..7];
                let first_line = c.message.lines().next().unwrap_or("");
                format!("  • {} - {} by {}", short_sha, first_line, c.author_name)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let display = format!(
            "📜 Commit History: {}/{}\n\
             {} commits{}\n\n\
             {}",
            args.owner,
            args.repo,
            count,
            if count > 10 { " (showing first 10)" } else { "" },
            preview_commits
        );

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubListCommitsOutput {
            success: true,
            owner: args.owner,
            repo: args.repo,
            count,
            commits: commit_summaries,
        };

        // Return ToolResponse
        Ok(ToolResponse::new(display, output))
    }
}
