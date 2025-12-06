use anyhow;
use kodegen_mcp_schema::github::{RequestCopilotReviewArgs, RequestCopilotReviewPrompts, GITHUB_REQUEST_COPILOT_REVIEW};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for requesting GitHub Copilot to review a pull request
#[derive(Clone)]
pub struct RequestCopilotReviewTool;

impl Tool for RequestCopilotReviewTool {
    type Args = RequestCopilotReviewArgs;
    type Prompts = RequestCopilotReviewPrompts;

    fn name() -> &'static str {
        GITHUB_REQUEST_COPILOT_REVIEW
    }

    fn description() -> &'static str {
        "Request GitHub Copilot to review a pull request (experimental feature). \
         Triggers automated code review from Copilot. Requires GITHUB_TOKEN and Copilot access."
    }

    fn read_only() -> bool {
        false // Triggers an action
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        true // Can be called multiple times safely
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

        // Call API wrapper (returns AsyncTask<Result<(), GitHubError>>)
        let task_result = client
            .request_copilot_review(args.owner.clone(), args.repo.clone(), args.pull_number)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubRequestCopilotReviewOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pull_number,
            message: format!("Copilot review requested for PR #{}", args.pull_number),
        };

        // Build human-readable display
        let display = format!(
            "🤖 Copilot Review Requested\n\n\
             Repository: {}/{}\n\
             PR: #{}\n\
             Status: Pending",
            output.owner,
            output.repo,
            output.pr_number
        );

        Ok(ToolResponse::new(display, output))
    }
}
