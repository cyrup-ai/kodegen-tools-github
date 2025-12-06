use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{SearchCodeArgs, SearchCodePrompts, GITHUB_SEARCH_CODE};

use crate::GitHubClient;

/// Tool for searching code across GitHub
pub struct SearchCodeTool;

impl Tool for SearchCodeTool {
    type Args = SearchCodeArgs;
    type Prompts = SearchCodePrompts;

    fn name() -> &'static str {
        GITHUB_SEARCH_CODE
    }

    fn description() -> &'static str {
        "Search code across GitHub repositories using GitHub's code search syntax"
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
        true
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) 
        -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError>
    {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let task_result = client
            .search_code(
                args.query.clone(),
                args.sort.clone(),
                args.order.clone(),
                args.page,
                args.per_page,
                args.enrich_stars,
            )
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let page =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Convert API response to typed output
        let total_count = page.total_count.unwrap_or(0);
        let items: Vec<kodegen_mcp_schema::github::GitHubCodeSearchResult> = page.items
            .iter()
            .map(|item| kodegen_mcp_schema::github::GitHubCodeSearchResult {
                name: item.name.clone(),
                path: item.path.clone(),
                sha: item.sha.clone(),
                repository_full_name: item.repository.full_name.clone().unwrap_or_default(),
                repository_owner: item.repository.owner.as_ref().map(|o| o.login.clone()).unwrap_or_default(),
                repository_name: item.repository.name.clone(),
                html_url: item.html_url.to_string(),
                git_url: item.git_url.to_string(),
                star_count: if args.enrich_stars {
                    item.repository.stargazers_count
                } else {
                    None
                },
            })
            .collect();

        // Build human-readable display
        let results_text = if items.is_empty() {
            "  No results found".to_string()
        } else {
            items.iter()
                .enumerate()
                .take(10)
                .map(|(i, item)| format!(
                    "  {}. {} - {}/{}\n     {}", 
                    i + 1,
                    item.path,
                    item.repository_owner,
                    item.repository_name,
                    item.html_url
                ))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let more_indicator = if items.len() > 10 {
            format!("\n  ... and {} more results", items.len() - 10)
        } else {
            String::new()
        };

        let display = format!(
            "🔍 GitHub Code Search\n\n\
             Query: {}\n\
             Total Results: {}\n\
             Results Returned: {}\n\n\
             {}{}",
            args.query, total_count, items.len(), results_text, more_indicator
        );

        let output = kodegen_mcp_schema::github::GitHubSearchCodeOutput {
            success: true,
            query: args.query,
            total_count: total_count as u32,
            items,
        };

        Ok(ToolResponse::new(display, output))
    }
}
