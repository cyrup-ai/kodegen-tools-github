//! GitHub issue retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{
    GetIssueArgs, GetIssuePrompts, GitHubGetIssueOutput, GitHubIssue, GITHUB_GET_ISSUE,
};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for fetching a GitHub issue by number
#[derive(Clone)]
pub struct GetIssueTool;

impl Tool for GetIssueTool {
    type Args = GetIssueArgs;
    type Prompts = GetIssuePrompts;

    fn name() -> &'static str {
        GITHUB_GET_ISSUE
    }

    fn description() -> &'static str {
        "Fetch a single GitHub issue by number. Returns detailed issue information including \
         title, body, state, labels, assignees, comments count, and timestamps. \
         Requires GITHUB_TOKEN environment variable."
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

        // Call API wrapper (returns AsyncTask<Result<Issue, GitHubError>>)
        // The .await returns Result<Result<Issue, GitHubError>, RecvError>
        let task_result = client
            .get_issue(args.owner.clone(), args.repo.clone(), args.issue_number)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let issue =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Convert octocrab Issue to our typed output
        let state_str = match issue.state {
            octocrab::models::IssueState::Open => "open",
            octocrab::models::IssueState::Closed => "closed",
            _ => "unknown",
        };

        let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
        let assignees: Vec<String> = issue
            .assignees
            .iter()
            .map(|u| u.login.clone())
            .collect();

        let github_issue = GitHubIssue {
            number: issue.number,
            title: issue.title.clone(),
            body: issue.body.clone(),
            state: state_str.to_string(),
            author: issue.user.login.clone(),
            created_at: issue.created_at.to_rfc3339(),
            updated_at: issue.updated_at.to_rfc3339(),
            labels,
            assignees,
            closed_at: issue.closed_at.map(|d| d.to_rfc3339()),
            comments_count: issue.comments,
            html_url: issue.html_url.to_string(),
        };

        let output = GitHubGetIssueOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            issue: github_issue.clone(),
        };

        let display = format!(
            "Successfully fetched issue #{} from {}/{}\n\nTitle: {}\nState: {}\nAuthor: {}\nComments: {}\nURL: {}",
            github_issue.number,
            args.owner,
            args.repo,
            github_issue.title,
            github_issue.state,
            github_issue.author,
            github_issue.comments_count,
            github_issue.html_url
        );

        Ok(ToolResponse::new(display, output))
    }
}
