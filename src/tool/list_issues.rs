//! GitHub issues listing tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{
    ListIssuesArgs, ListIssuesPromptArgs, GitHubListIssuesOutput, GitHubIssueSummary,
    GITHUB_LIST_ISSUES,
};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::github::ListIssuesRequest;

/// Tool for listing and filtering GitHub issues
#[derive(Clone)]
pub struct ListIssuesTool;

impl Tool for ListIssuesTool {
    type Args = ListIssuesArgs;
    type PromptArgs = ListIssuesPromptArgs;

    fn name() -> &'static str {
        GITHUB_LIST_ISSUES
    }

    fn description() -> &'static str {
        "List and filter issues in a GitHub repository. Supports filtering by state, labels, \
         assignee, and pagination. Returns an array of issue objects. \
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

        // Convert state string to IssueState enum
        // Note: "all" is handled by passing None (no state filter)
        let state = args
            .state
            .as_ref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "open" => Some(octocrab::models::IssueState::Open),
                "closed" => Some(octocrab::models::IssueState::Closed),
                "all" => None,
                _ => None,
            });

        // Convert per_page to u8 (GitHub API expects u8)
        let per_page = args.per_page.map(|p| p.min(100) as u8);

        // Build request
        let request = ListIssuesRequest {
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            state,
            labels: args.labels.clone(),
            sort: None,
            direction: None,
            since: None,
            page: args.page,
            per_page,
        };

        // Call API wrapper
        let mut issue_stream = client.list_issues(request);

        // Collect stream results
        let mut issues = Vec::new();
        while let Some(result) = issue_stream.next().await {
            let issue =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            issues.push(issue);
        }

        // Convert to typed output
        let issue_summaries: Vec<GitHubIssueSummary> = issues
            .iter()
            .map(|issue| {
                let state_str = match issue.state {
                    octocrab::models::IssueState::Open => "open",
                    octocrab::models::IssueState::Closed => "closed",
                    _ => "unknown",
                };
                let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();

                GitHubIssueSummary {
                    number: issue.number,
                    title: issue.title.clone(),
                    state: state_str.to_string(),
                    author: issue.user.login.clone(),
                    created_at: issue.created_at.to_rfc3339(),
                    labels,
                }
            })
            .collect();

        let output = GitHubListIssuesOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            count: issue_summaries.len(),
            issues: issue_summaries,
        };

        // Build user-friendly display string
        let state_filter = args.state.as_deref().unwrap_or("open");
        let display = format!(
            "Successfully retrieved {} issue(s) from {}/{} (state: {})",
            output.count, args.owner, args.repo, state_filter
        );

        Ok(ToolResponse::new(display, output))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "Optional focus area for examples: 'filtering' (state/labels/assignee), \
                     'pagination' (page/per_page), 'advanced' (combined filters), or 'all' (comprehensive)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        let assistant_response = match args.focus_area.as_deref() {
            Some("filtering") => {
                "Use the list_issues tool to filter repository issues by state, labels, and assignee:\n\n\
                 Filter by state:\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"state\": \"open\"})\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"state\": \"closed\"})\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"state\": \"all\"})\n\n\
                 Filter by labels (multiple labels = AND logic):\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"labels\": [\"bug\"]})\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"labels\": [\"bug\", \"priority-high\"]})\n\n\
                 Returns GitHubListIssuesOutput with:\n\
                 - success: boolean\n\
                 - owner, repo: repository info\n\
                 - count: number of issues returned\n\
                 - issues: array of GitHubIssueSummary objects"
            }
            Some("pagination") => {
                "Use the list_issues tool to paginate through repository issues:\n\n\
                 Customize results per page:\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"per_page\": 50})\n\n\
                 Navigate to specific pages:\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"page\": 2, \"per_page\": 20})\n\n\
                 Returns GitHubListIssuesOutput with paginated results."
            }
            _ => {
                "Use the list_issues tool to list and filter repository issues:\n\n\
                 List all open issues:\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\"})\n\n\
                 Filter by state:\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"state\": \"closed\"})\n\n\
                 Filter by labels:\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"labels\": [\"bug\"]})\n\n\
                 With pagination:\n\
                 list_issues({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"per_page\": 50, \"page\": 2})\n\n\
                 Returns GitHubListIssuesOutput with:\n\
                 - success: boolean\n\
                 - owner, repo: repository info\n\
                 - count: number of issues returned\n\
                 - issues: array of GitHubIssueSummary (number, title, state, author, created_at, labels)"
            }
        };

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I list and filter GitHub issues?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(assistant_response),
            },
        ])
    }
}
