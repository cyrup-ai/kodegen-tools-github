//! GitHub issues search tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{SearchIssuesArgs, SearchIssuesPromptArgs, GITHUB_SEARCH_ISSUES};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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

        // Build dual-content response
        let mut contents = Vec::new();

        // Content[0]: Human-Readable Summary (2 lines with icons)
        // Line 1: Cyan with search icon
        // Line 2: Plain text with issue icon and first issue title
        let first_issue_title = issues.first()
            .map(|i| {
                let title = &i.title;
                if title.len() > 50 {
                    format!("{}...", &title[..47])
                } else {
                    title.to_string()
                }
            })
            .unwrap_or_else(|| "None".to_string());

        let summary = format!(
            "\x1b[36m\u{f002} Issue Search: {}\x1b[0m\n \u{f05a} Results: {} · Top: {}",
            query,
            issues.len(),
            first_issue_title
        );
        contents.push(Content::text(summary));

        // Content[1]: Machine-Parseable JSON
        let metadata = json!({
            "issues": issues,
            "count": issues.len()
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
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
        // Extract and default arguments
        let focus_area = args.focus_area.as_deref().unwrap_or("all");
        let include_examples = args.include_examples.unwrap_or(true);

        // Generate user question and assistant response based on focus_area
        let (user_question, assistant_response) = match focus_area {
            "basic" => {
                let question = "How do I search for GitHub issues in specific repositories?";
                let response = if include_examples {
                    "The search_issues tool lets you search GitHub repositories by state. Basic examples:\n\n\
                     Search in specific repo:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:open\"})\n\n\
                     Search by state:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:closed\"})\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - GITHUB_TOKEN environment variable must be set"
                } else {
                    "Use repo:owner/name to search a specific repository, is:open for open issues, is:closed for closed issues.\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - GITHUB_TOKEN environment variable must be set"
                };
                (question, response)
            }
            "filters" => {
                let question = "How do I filter GitHub issues by labels, assignees, and other criteria?";
                let response = if include_examples {
                    "Filter issues by labels, assignees, authors, and dates:\n\n\
                     FILTER BY LABELS:\n\
                     Single label:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world label:bug\"})\n\
                     Multiple labels (AND):\n\
                     search_issues({\"query\": \"repo:octocat/hello-world label:bug label:priority-high\"})\n\n\
                     FILTER BY PEOPLE:\n\
                     By assignee:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world assignee:octocat\"})\n\
                     By author:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world author:alice\"})\n\
                     By participant:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world involves:bob\"})\n\n\
                     DATE FILTERS:\n\
                     Created after date:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world created:>=2024-01-01\"})\n\
                     Updated recently:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world updated:>=2024-03-01\"})\n\
                     Date range:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world created:2024-01-01..2024-12-31\"})\n\n\
                     TEXT SEARCH:\n\
                     In title or body:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world authentication error\"})\n\
                     In title only:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world authentication in:title\"})\n\
                     In body only:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world error in:body\"})\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - Use quotes for multi-word searches: \"bug report\"\n\
                     - GITHUB_TOKEN environment variable must be set"
                } else {
                    "Use label:name for labels, assignee:name for assignees, author:name for authors, created:date for date ranges, and in:title/body for text location.\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - Use quotes for multi-word searches: \"bug report\"\n\
                     - GITHUB_TOKEN environment variable must be set"
                };
                (question, response)
            }
            "advanced" => {
                let question = "How do I combine multiple filters for complex GitHub issue searches?";
                let response = if include_examples {
                    "Combine multiple filters for powerful searches:\n\n\
                     COMBINED FILTERS:\n\
                     Complex query:\n\
                     search_issues({\n\
                       \"query\": \"repo:octocat/hello-world is:open label:bug assignee:alice created:>=2024-01-01\",\n\
                       \"sort\": \"created\",\n\
                       \"order\": \"desc\"\n\
                     })\n\n\
                     Multiple labels and states:\n\
                     search_issues({\n\
                       \"query\": \"repo:octocat/hello-world is:open label:bug label:critical author:alice\"\n\
                     })\n\n\
                     Date ranges with participants:\n\
                     search_issues({\n\
                       \"query\": \"repo:octocat/hello-world updated:2024-01-01..2024-12-31 involves:bob\"\n\
                     })\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - Use quotes for multi-word searches: \"bug report\"\n\
                     - GITHUB_TOKEN environment variable must be set"
                } else {
                    "Combine filters with spaces. Most filters can be combined: labels, assignees, authors, dates, and text search.\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - Use quotes for multi-word searches: \"bug report\"\n\
                     - GITHUB_TOKEN environment variable must be set"
                };
                (question, response)
            }
            "pagination" => {
                let question = "How do I paginate through search results in the search_issues tool?";
                let response = if include_examples {
                    "Navigate through large result sets using pagination parameters:\n\n\
                     PAGINATION:\n\
                     First page (default):\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:open\", \"per_page\": 50})\n\n\
                     Second page:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:open\", \"per_page\": 50, \"page\": 2})\n\n\
                     Different page sizes:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:open\", \"per_page\": 100})\n\n\
                     Combine with sorting for consistent results:\n\
                     search_issues({\n\
                       \"query\": \"repo:octocat/hello-world is:open\",\n\
                       \"sort\": \"created\",\n\
                       \"order\": \"desc\",\n\
                       \"per_page\": 50,\n\
                       \"page\": 2\n\
                     })\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - Use quotes for multi-word searches: \"bug report\"\n\
                     - GITHUB_TOKEN environment variable must be set"
                } else {
                    "Use per_page (1-100) and page parameters. Default is page 1, per_page 30. Combine with sort and order for consistent pagination.\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - Use quotes for multi-word searches: \"bug report\"\n\
                     - GITHUB_TOKEN environment variable must be set"
                };
                (question, response)
            }
            _ => {
                // "all" or default: comprehensive content
                let question = "How do I search for GitHub issues using the search_issues tool?";
                let response = "The search_issues tool uses GitHub's powerful search syntax. Here are comprehensive examples:\n\n\
                     BASIC SEARCHES:\n\
                     Search in specific repo:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:open\"})\n\n\
                     Search by state:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:closed\"})\n\n\
                     FILTER BY LABELS:\n\
                     Single label:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world label:bug\"})\n\
                     Multiple labels (AND):\n\
                     search_issues({\"query\": \"repo:octocat/hello-world label:bug label:priority-high\"})\n\n\
                     FILTER BY PEOPLE:\n\
                     By assignee:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world assignee:octocat\"})\n\
                     By author:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world author:alice\"})\n\
                     By participant:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world involves:bob\"})\n\n\
                     DATE FILTERS:\n\
                     Created after date:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world created:>=2024-01-01\"})\n\
                     Updated recently:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world updated:>=2024-03-01\"})\n\
                     Date range:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world created:2024-01-01..2024-12-31\"})\n\n\
                     TEXT SEARCH:\n\
                     In title or body:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world authentication error\"})\n\
                     In title only:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world authentication in:title\"})\n\
                     In body only:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world error in:body\"})\n\n\
                     COMBINED FILTERS:\n\
                     Complex query:\n\
                     search_issues({\n\
                       \"query\": \"repo:octocat/hello-world is:open label:bug assignee:alice created:>=2024-01-01\",\n\
                       \"sort\": \"created\",\n\
                       \"order\": \"desc\"\n\
                     })\n\n\
                     SORTING:\n\
                     - sort: \"created\", \"updated\", \"comments\", \"reactions\"\n\
                     - order: \"asc\" (ascending) or \"desc\" (descending)\n\n\
                     PAGINATION:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:open\", \"per_page\": 50, \"page\": 2})\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - Use quotes for multi-word searches: \"bug report\"\n\
                     - GITHUB_TOKEN environment variable must be set";
                (question, response)
            }
        };

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(user_question),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(assistant_response),
            },
        ])
    }
}
