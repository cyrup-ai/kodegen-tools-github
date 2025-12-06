use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{SearchRepositoriesArgs, SearchRepositoriesPrompts, GITHUB_SEARCH_REPOSITORIES};
use octocrab::Octocrab;

/// Tool for searching GitHub repositories
pub struct SearchRepositoriesTool;

impl Tool for SearchRepositoriesTool {
    type Args = SearchRepositoriesArgs;
    type Prompts = SearchRepositoriesPrompts;

    fn name() -> &'static str {
        GITHUB_SEARCH_REPOSITORIES
    }

    fn description() -> &'static str {
        "Search GitHub repositories using GitHub's repository search syntax"
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Create octocrab instance directly
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let mut request = octocrab.search().repositories(&args.query);

        if let Some(sort_val) = &args.sort {
            request = request.sort(sort_val);
        }

        if let Some(order_val) = &args.order {
            request = request.order(order_val);
        }

        if let Some(p) = args.page {
            request = request.page(p);
        }

        if let Some(pp) = args.per_page {
            request = request.per_page(pp);
        }

        let page = request
            .send()
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Convert API response to typed output
        let total_count = page.total_count.unwrap_or(0);
        let items: Vec<kodegen_mcp_schema::github::GitHubRepoSearchResult> = page.items
            .iter()
            .map(|repo| kodegen_mcp_schema::github::GitHubRepoSearchResult {
                full_name: repo.full_name.clone().unwrap_or_default(),
                name: repo.name.clone(),
                owner: repo.owner.as_ref().map(|o| o.login.clone()).unwrap_or_default(),
                description: repo.description.clone(),
                html_url: repo.html_url.as_ref().map(|u| u.to_string()).unwrap_or_default(),
                language: repo.language.as_ref().and_then(|v| v.as_str()).map(|s| s.to_string()),
                stars: repo.stargazers_count.unwrap_or(0),
                forks: repo.forks_count.unwrap_or(0),
                watchers: repo.watchers_count.unwrap_or(0),
                open_issues: repo.open_issues_count.unwrap_or(0),
                created_at: repo.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                updated_at: repo.updated_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                pushed_at: repo.pushed_at.map(|dt| dt.to_rfc3339()),
                topics: repo.topics.clone().unwrap_or_default(),
                archived: repo.archived.unwrap_or(false),
                fork: repo.fork.unwrap_or(false),
            })
            .collect();

        // Build human-readable display
        let results_text = if items.is_empty() {
            "  No repositories found".to_string()
        } else {
            items.iter()
                .map(|r| {
                    let desc = r.description.as_deref().unwrap_or("No description");
                    let lang = r.language.as_deref().unwrap_or("Unknown");
                    format!("  • {} - ⭐ {} - {} - {}", r.full_name, r.stars, lang, desc)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let display = format!(
            "🔍 GitHub Repository Search\n\n\
             Query: {}\n\
             Total Results: {}\n\
             Results Returned: {}\n\n\
             {}",
            args.query, total_count, items.len(), results_text
        );

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubSearchReposOutput {
            success: true,
            query: args.query,
            total_count: total_count as u32,
            items,
        };

        // Return ToolResponse wrapper
        Ok(ToolResponse::new(display, output))
    }
}
