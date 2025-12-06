use anyhow;
use kodegen_mcp_schema::github::{AddPullRequestReviewCommentArgs, AddPullRequestReviewCommentPrompts, GITHUB_ADD_PULL_REQUEST_REVIEW_COMMENT};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for adding inline review comments to a pull request
#[derive(Clone)]
pub struct AddPullRequestReviewCommentTool;

impl Tool for AddPullRequestReviewCommentTool {
    type Args = AddPullRequestReviewCommentArgs;
    type Prompts = AddPullRequestReviewCommentPrompts;

    fn name() -> &'static str {
        GITHUB_ADD_PULL_REQUEST_REVIEW_COMMENT
    }

    fn description() -> &'static str {
        "Add an inline review comment to a pull request (comment on specific lines of code). \
         Supports single-line, multi-line, and threaded comments. Requires GITHUB_TOKEN."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        false // Multiple comments can be created
    }

    fn open_world() -> bool {
        true // Calls external GitHub API
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) 
        -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Build request
        let request = crate::github::AddPullRequestReviewCommentRequest {
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pull_number,
            body: args.body.clone(),
            commit_id: args.commit_id.clone(),
            path: args.path.clone(),
            line: args.line,
            side: args.side.clone(),
            start_line: args.start_line,
            start_side: args.start_side.clone(),
            subject_type: args.subject_type.clone(),
            in_reply_to: args.in_reply_to,
        };

        // Call API wrapper (returns AsyncTask<Result<ReviewComment, GitHubError>>)
        let task_result = client.add_pull_request_review_comment(request).await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let comment =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build location string for display
        let location_str = if let Some(in_reply_to) = args.in_reply_to {
            format!("Reply to comment #{}", in_reply_to)
        } else if let Some(path) = &args.path {
            if let Some(start_line) = args.start_line {
                // Multi-line comment: show range
                format!("{}:Lines {}-{}", path, start_line, args.line.unwrap_or(0))
            } else {
                // Single-line comment: show single line
                format!("{}:Line {}", path, args.line.unwrap_or(0))
            }
        } else {
            "N/A".to_string()
        };

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubAddPrReviewCommentOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pull_number,
            comment_id: comment.id.0, // Convert octocrab::models::CommentId to u64
            message: format!("Added review comment to PR #{}", args.pull_number),
        };

        // Build human-readable display
        let display = format!(
            "💬 Review Comment Added\n\n\
             Repository: {}/{}\n\
             PR: #{}\n\
             Comment ID: {}\n\
             Location: {}",
            output.owner,
            output.repo,
            output.pr_number,
            output.comment_id,
            location_str
        );

        Ok(ToolResponse::new(display, output))
    }
}
