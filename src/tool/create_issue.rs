//! GitHub issue creation tool

use anyhow;
use kodegen_mcp_schema::github::{
    CreateIssueArgs, CreateIssuePromptArgs, GitHubCreateIssueOutput, GITHUB_CREATE_ISSUE,
};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for creating GitHub issues
#[derive(Clone)]
pub struct CreateIssueTool;

impl Tool for CreateIssueTool {
    type Args = CreateIssueArgs;
    type PromptArgs = CreateIssuePromptArgs;

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

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "What aspect of create_issue to focus on: 'basic' for core functionality, \
                     'labels' for issue categorization, 'assignees' for team assignment, \
                     'authentication' for GITHUB_TOKEN setup, or 'team-collaboration' for team workflows"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "use_case".to_string(),
                title: None,
                description: Some(
                    "Repository context for examples: 'personal' for solo projects, \
                     'team' for organizational repos, or 'open-source' for community projects"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I create a GitHub issue with labels and assignees?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the create_issue tool to create a GitHub issue:\n\n\
                     Basic usage:\n\
                     create_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"title\": \"Bug report\"})\n\n\
                     With body and labels:\n\
                     create_issue({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"title\": \"Bug: Login fails\",\n\
                       \"body\": \"When I try to login, the form doesn't submit...\",\n\
                       \"labels\": [\"bug\", \"priority-high\"],\n\
                       \"assignees\": [\"octocat\"]\n\
                     })\n\n\
                     Returns GitHubCreateIssueOutput with:\n\
                     - success: boolean\n\
                     - owner, repo: repository info\n\
                     - issue_number: the created issue number\n\
                     - html_url: link to the issue\n\
                     - message: success message\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos, 'public_repo' for public\n\
                     - User must have write access to the repository\n\
                     - Labels must already exist in the repository\n\
                     - Assignees must be collaborators on the repository\n\n\
                     Tips:\n\
                     - Body supports Markdown formatting\n\
                     - You can @mention users in the body\n\
                     - Labels are case-sensitive\n\
                     - Multiple assignees can be specified",
                ),
            },
        ])
    }
}
