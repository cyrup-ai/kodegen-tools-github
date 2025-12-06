//! GitHub issue comments retrieval tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{
    GetIssueCommentsArgs, GetIssueCommentsPrompts, GitHubGetIssueCommentsOutput, GitHubComment,
    GITHUB_GET_ISSUE_COMMENTS,
};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for fetching all comments on a GitHub issue
#[derive(Clone)]
pub struct GetIssueCommentsTool;

impl Tool for GetIssueCommentsTool {
    type Args = GetIssueCommentsArgs;
    type Prompts = GetIssueCommentsPrompts;

    fn name() -> &'static str {
        GITHUB_GET_ISSUE_COMMENTS
    }

    fn description() -> &'static str {
        "Fetch all comments for a GitHub issue. Returns an array of comment objects \
         including author, body, timestamps, and metadata. Comments are returned in \
         chronological order. Requires GITHUB_TOKEN environment variable."
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
        true // Calls external GitHub API
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Call API wrapper (returns AsyncStream)
        let mut comment_stream =
            client.get_issue_comments(args.owner.clone(), args.repo.clone(), args.issue_number);

        // Collect stream results
        let mut comments = Vec::new();
        while let Some(result) = comment_stream.next().await {
            let comment =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            comments.push(comment);
        }

        // Convert to typed output
        let github_comments: Vec<GitHubComment> = comments
            .iter()
            .map(|c| GitHubComment {
                id: c.id.into_inner(),
                author: c.user.login.clone(),
                body: c.body.clone().unwrap_or_default(),
                created_at: c.created_at.to_rfc3339(),
                updated_at: c.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            })
            .collect();

        let output = GitHubGetIssueCommentsOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            issue_number: args.issue_number,
            count: github_comments.len(),
            comments: github_comments,
        };

        // Build user-friendly display string
        let display = format!(
            "Successfully retrieved {} comment{} for issue #{} in {}/{}",
            output.count,
            if output.count == 1 { "" } else { "s" },
            output.issue_number,
            args.owner,
            args.repo
        );

        Ok(ToolResponse::new(display, output))
    }
}
