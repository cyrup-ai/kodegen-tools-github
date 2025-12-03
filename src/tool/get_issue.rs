//! GitHub issue retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{
    GetIssueArgs, GetIssuePromptArgs, GitHubGetIssueOutput, GitHubIssue, GITHUB_GET_ISSUE,
};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for fetching a GitHub issue by number
#[derive(Clone)]
pub struct GetIssueTool;

impl Tool for GetIssueTool {
    type Args = GetIssueArgs;
    type PromptArgs = GetIssuePromptArgs;

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

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "detail_focus".to_string(),
                title: None,
                description: Some(
                    "Focus teaching on: 'basic' (minimal usage), 'advanced' (complex patterns, response interpretation), or 'pr' (pull request specific usage)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            // Exchange 1: Basic Usage
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I fetch a specific GitHub issue?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the get_issue tool to fetch a GitHub issue by its number:\n\n\
                     BASIC USAGE:\n\
                     get_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42})\n\n\
                     REQUIRED PARAMETERS:\n\
                     - owner: Repository owner (user or organization name)\n\
                     - repo: Repository name\n\
                     - issue_number: The issue NUMBER (e.g., 42 from #42), NOT the internal ID\n\n\
                     RESPONSE FORMAT (Typed Output):\n\
                     Returns a GitHubGetIssueOutput with:\n\
                     - success: boolean indicating success\n\
                     - owner: repository owner\n\
                     - repo: repository name\n\
                     - issue: GitHubIssue object with all details\n\n\
                     ISSUE FIELDS:\n\
                     - number: Issue number\n\
                     - title: Issue title\n\
                     - body: Issue description (Markdown)\n\
                     - state: \"open\" or \"closed\"\n\
                     - author: Issue creator username\n\
                     - labels: Array of label names\n\
                     - assignees: Array of assigned usernames\n\
                     - comments_count: Number of comments\n\
                     - created_at, updated_at: ISO timestamps\n\
                     - html_url: Link to issue on GitHub.com"
                ),
            },
            // Exchange 2: Important Distinction - Issues vs PRs
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "Can I use get_issue to fetch pull requests?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Yes! On GitHub, pull requests ARE treated as issues internally, so get_issue works for both.\n\n\
                     TO FETCH A PULL REQUEST:\n\
                     - Use the same syntax: get_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 123})\n\
                     - It doesn't matter if #123 is a PR or issue - the endpoint returns both\n\n\
                     WHY THIS MATTERS:\n\
                     - Use get_issue to check if a number refers to a PR before working with it\n\
                     - For PR-specific operations (reviews, merge), use dedicated tools\n\
                     - This tool returns the same data structure for both"
                ),
            },
        ])
    }
}
