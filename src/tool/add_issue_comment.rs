//! GitHub issue comment addition tool

use anyhow;
use kodegen_mcp_schema::github::{
    AddIssueCommentArgs, AddIssueCommentPrompts, GitHubAddIssueCommentOutput,
    GITHUB_ADD_ISSUE_COMMENT,
};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for adding comments to GitHub issues
#[derive(Clone)]
pub struct AddIssueCommentTool;

impl Tool for AddIssueCommentTool {
    type Args = AddIssueCommentArgs;
    type Prompts = AddIssueCommentPrompts;

    fn name() -> &'static str {
        GITHUB_ADD_ISSUE_COMMENT
    }

    fn description() -> &'static str {
        "Add a comment to an existing GitHub issue. Supports Markdown formatting in the comment body. \
         Requires GITHUB_TOKEN environment variable with write access to the repository."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Creates new comment each time
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

        // Call API wrapper (returns AsyncTask<Result<Comment, GitHubError>>)
        // The .await returns Result<Result<Comment, GitHubError>, RecvError>
        let task_result = client
            .add_issue_comment(args.owner.clone(), args.repo.clone(), args.issue_number, args.body)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let comment =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        let display = format!(
            "💬 Comment Added to Issue #{}\n\n\
             Repository: {}/{}\n\
             Comment ID: {}\n\
             ✅ Comment added successfully",
            args.issue_number,
            args.owner,
            args.repo,
            comment.id
        );

        let output = GitHubAddIssueCommentOutput {
            success: true,
            owner: args.owner,
            repo: args.repo,
            issue_number: args.issue_number,
            comment_id: comment.id.into_inner(),
            message: format!("Comment added successfully (ID: {})", comment.id),
        };

        Ok(ToolResponse::new(display, output))
    }
}
