//! GitHub issues search tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{
    SearchIssuesArgs, SearchIssuesPromptArgs, GitHubSearchIssuesOutput, GitHubIssueSummary,
    GITHUB_SEARCH_ISSUES,
};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for searching GitHub issues using GitHub's search syntax
#[derive(Clone)]
pub struct SearchIssuesTool;

impl Tool for SearchIssuesTool {
    type Args = SearchIssuesArgs;
    type PromptArgs = SearchIssuesPromptArgs;

    fn name() -> &'static str {
        GITHUB_SEARCH_ISSUES
    }

    fn description() -> &'static str {
        "Search for issues across GitHub using GitHub's powerful search syntax. \
         Supports filtering by repository, state, labels, assignee, author, dates, and more. \
         Returns matching issues with relevance ranking. \
         Requires GITHUB_TOKEN environment variable. Note: Search API has stricter rate limits."
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

        // Convert per_page to u8 (GitHub API expects u8)
        let per_page = args.per_page.map(|p| p.min(100) as u8);

        // Clone query before moving it
        let query = args.query.clone();

        // Call API wrapper
        let mut issue_stream =
            client.search_issues(args.query, args.sort, args.order, args.page, per_page);

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

        let output = GitHubSearchIssuesOutput {
            success: true,
            query: query.clone(),
            total_count: issue_summaries.len() as u32,
            items: issue_summaries,
        };

        // Build user-friendly display string
        let display = format!(
            "GitHub Issues Search Results\n\nQuery: {}\nTotal Results: {}\nResults Returned: {}\n\nSearch completed successfully.",
            query,
            output.total_count,
            output.items.len()
        );

        Ok(ToolResponse::new(display, output))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "Teaching focus area: 'basic' (repo/state searches), 'filters' (labels/people), \
                     'advanced' (complex queries), 'pagination' (page navigation), or 'all' (comprehensive)"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "include_examples".to_string(),
                title: None,
                description: Some(
                    "Include comprehensive code examples (true) or concise explanations only (false)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        let focus_area = args.focus_area.as_deref().unwrap_or("all");

        let assistant_response = match focus_area {
            "basic" => {
                "The search_issues tool lets you search GitHub repositories. Basic examples:\n\n\
                 Search in specific repo:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world is:open\"})\n\n\
                 Search by state:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world is:closed\"})\n\n\
                 Returns GitHubSearchIssuesOutput with:\n\
                 - success: boolean\n\
                 - query: the search query used\n\
                 - total_count: number of results\n\
                 - items: array of GitHubIssueSummary objects"
            }
            "filters" => {
                "Filter issues by labels, assignees, authors, and dates:\n\n\
                 FILTER BY LABELS:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world label:bug\"})\n\n\
                 FILTER BY PEOPLE:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world assignee:octocat\"})\n\
                 search_issues({\"query\": \"repo:octocat/hello-world author:alice\"})\n\n\
                 DATE FILTERS:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world created:>=2024-01-01\"})\n\n\
                 Returns GitHubSearchIssuesOutput with typed results."
            }
            _ => {
                "The search_issues tool uses GitHub's powerful search syntax:\n\n\
                 BASIC SEARCHES:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world is:open\"})\n\n\
                 FILTER BY LABELS:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world label:bug\"})\n\n\
                 FILTER BY PEOPLE:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world assignee:octocat\"})\n\n\
                 DATE FILTERS:\n\
                 search_issues({\"query\": \"repo:octocat/hello-world created:>=2024-01-01\"})\n\n\
                 COMBINED FILTERS:\n\
                 search_issues({\n\
                   \"query\": \"repo:octocat/hello-world is:open label:bug assignee:alice\",\n\
                   \"sort\": \"created\",\n\
                   \"order\": \"desc\"\n\
                 })\n\n\
                 Returns GitHubSearchIssuesOutput with:\n\
                 - success: boolean\n\
                 - query: the search query used\n\
                 - total_count: number of results\n\
                 - items: array of GitHubIssueSummary (number, title, state, author, created_at, labels)\n\n\
                 IMPORTANT: Search API has stricter rate limits (30 requests/minute)"
            }
        };

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I search for GitHub issues?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(assistant_response),
            },
        ])
    }
}
