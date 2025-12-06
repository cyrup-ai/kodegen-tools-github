//! GitHub issue creation tool

use anyhow;
use kodegen_mcp_schema::github::{
    CreateIssueArgs, CreateIssuePrompts, GitHubCreateIssueOutput, GITHUB_CREATE_ISSUE,
};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for creating GitHub issues
#[derive(Clone)]
pub struct CreateIssueTool;

impl Tool for CreateIssueTool {
    type Args = CreateIssueArgs;
    type Prompts = CreateIssuePrompts;

    fn name() -> &'static str {
        GITHUB_CREATE_ISSUE
    }

    fn description() -> &'static str {
        "Create a new issue in a GitHub repository. Supports setting title, body, \
         labels, and assignees. Requires GITHUB_TOKEN environment variable with appropriate permissions."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Multiple calls create multiple issues
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

        // Call API wrapper (returns AsyncTask<Result<Issue, GitHubError>>)
        // The .await returns Result<Result<Issue, GitHubError>, RecvError>
        let task_result = client
            .create_issue(
                args.owner.clone(),
                args.repo.clone(),
                args.title.clone(),
                args.body.clone(),
                args.assignees.clone(),
                args.labels.clone(),
            )
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let issue =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        let output = GitHubCreateIssueOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            issue_number: issue.number,
            html_url: issue.html_url.to_string(),
            message: format!("Issue #{} created successfully", issue.number),
        };

        let display = format!(
            "Successfully created issue #{} in {}/{}:\n  Title: {}\n  URL: {}",
            issue.number,
            args.owner,
            args.repo,
            args.title,
            issue.html_url
        );

        Ok(ToolResponse::new(display, output))
    }
}
