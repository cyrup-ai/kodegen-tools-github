use anyhow;
use kodegen_mcp_schema::github::{CreatePullRequestReviewArgs, CreatePullRequestReviewPrompts, GITHUB_CREATE_PULL_REQUEST_REVIEW};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use octocrab::models::pulls::ReviewAction;

/// Tool for creating a review on a pull request
#[derive(Clone)]
pub struct CreatePullRequestReviewTool;

impl Tool for CreatePullRequestReviewTool {
    type Args = CreatePullRequestReviewArgs;
    type Prompts = CreatePullRequestReviewPrompts;

    fn name() -> &'static str {
        GITHUB_CREATE_PULL_REQUEST_REVIEW
    }

    fn description() -> &'static str {
        "Create a review on a pull request (approve, request changes, or comment). \
         Requires GITHUB_TOKEN environment variable with repo permissions."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        false // Multiple reviews can be submitted
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

        // Convert string event to ReviewAction enum
        let event = match args.event.to_uppercase().as_str() {
            "APPROVE" => ReviewAction::Approve,
            "REQUEST_CHANGES" => ReviewAction::RequestChanges,
            "COMMENT" => ReviewAction::Comment,
            _ => {
                return Err(McpError::InvalidArguments(format!(
                    "Invalid event '{}'. Must be APPROVE, REQUEST_CHANGES, or COMMENT",
                    args.event
                )));
            }
        };

        // Build options struct
        let options = crate::CreatePullRequestReviewOptions {
            event,
            body: args.body.clone(),
            commit_id: args.commit_id.clone(),
            comments: None, // Inline comments not supported in this tool
        };

        // Call API wrapper (returns AsyncTask<Result<Review, GitHubError>>)
        let task_result = client
            .create_pull_request_review(args.owner.clone(), args.repo.clone(), args.pull_number, options)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let review =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubCreatePrReviewOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pull_number,
            review_id: review.id.0, // Convert octocrab::models::ReviewId to u64
            event: args.event.to_uppercase(),
            message: format!("Created {} review on PR #{}", args.event.to_uppercase(), args.pull_number),
        };

        // Build human-readable display
        let display = format!(
            "✅ PR Review Created\n\n\
             Repository: {}/{}\n\
             PR: #{}\n\
             Review ID: {}\n\
             Event: {}\n\
             Body: {}",
            output.owner,
            output.repo,
            output.pr_number,
            output.review_id,
            output.event,
            args.body.as_deref().unwrap_or("(no comment)")
        );

        Ok(ToolResponse::new(display, output))
    }
}
