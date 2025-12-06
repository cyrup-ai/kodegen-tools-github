use anyhow;
use kodegen_mcp_schema::github::{SearchUsersArgs, SearchUsersPrompts, GITHUB_SEARCH_USERS};
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};

use crate::GitHubClient;

/// Tool for searching GitHub users
pub struct SearchUsersTool;

impl Tool for SearchUsersTool {
    type Args = SearchUsersArgs;
    type Prompts = SearchUsersPrompts;

    fn name() -> &'static str {
        GITHUB_SEARCH_USERS
    }

    fn description() -> &'static str {
        "Search GitHub users using GitHub's user search syntax"
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

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Convert sort string to UserSearchSort enum
        let sort_enum = if let Some(s) = args.sort.as_ref() {
            match s.as_str() {
                "followers" => Some(crate::github::UserSearchSort::Followers),
                "repositories" => Some(crate::github::UserSearchSort::Repositories),
                "joined" => Some(crate::github::UserSearchSort::Joined),
                _ => {
                    return Err(McpError::InvalidArguments(
                        "sort must be followers, repositories, or joined".into(),
                    ));
                }
            }
        } else {
            None
        };

        // Convert order string to SearchOrder enum
        let order_enum = if let Some(o) = args.order.as_ref() {
            match o.as_str() {
                "asc" => Some(crate::github::SearchOrder::Asc),
                "desc" => Some(crate::github::SearchOrder::Desc),
                _ => {
                    return Err(McpError::InvalidArguments(
                        "order must be asc or desc".into(),
                    ));
                }
            }
        } else {
            None
        };

        let task_result = client
            .search_users(args.query.clone(), sort_enum, order_enum, args.page, args.per_page)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let page =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Convert API response to typed output
        let total_count = page.total_count.unwrap_or(0);
        let items: Vec<kodegen_mcp_schema::github::GitHubUserSearchResult> = page.items
            .iter()
            .map(|user| kodegen_mcp_schema::github::GitHubUserSearchResult {
                login: user.login.clone(),
                id: user.id.0,
                avatar_url: user.avatar_url.to_string(),
                html_url: user.html_url.to_string(),
                user_type: user.r#type.clone(),
                name: None,  // Author type doesn't have name field
                bio: None,   // Author type doesn't have bio field
                location: None,  // Author type doesn't have location field
                followers: None,  // Author type doesn't have followers field
            })
            .collect();

        // Build human-readable display
        let results_text = if items.is_empty() {
            "  No users found".to_string()
        } else {
            items.iter()
                .map(|u| {
                    let name = u.name.as_deref().unwrap_or(&u.login);
                    let location = u.location.as_deref().unwrap_or("Unknown location");
                    let followers = u.followers.map(|f| format!("{} followers", f)).unwrap_or_else(|| "Unknown followers".to_string());
                    format!("  • {} (@{}) - {} - {}", name, u.login, location, followers)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let display = format!(
            "👥 GitHub User Search\n\n\
             Query: {}\n\
             Total Results: {}\n\
             Results Returned: {}\n\n\
             {}",
            args.query, total_count, items.len(), results_text
        );

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubSearchUsersOutput {
            success: true,
            query: args.query,
            total_count: total_count as u32,
            items,
        };

        // Return ToolResponse wrapper
        Ok(ToolResponse::new(display, output))
    }
}
