//! GitHub authenticated user retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{GetMeArgs, GetMePromptArgs, GITHUB_GET_ME};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use serde_json::Value;

/// Tool for getting information about the authenticated GitHub user
#[derive(Clone)]
pub struct GetMeTool;

impl Tool for GetMeTool {
    type Args = GetMeArgs;
    type PromptArgs = GetMePromptArgs;
    
    fn name() -> &'static str {
        GITHUB_GET_ME
    }
    
    fn description() -> &'static str {
        "Get information about the authenticated GitHub user. Returns user profile \
         details including login, name, email, avatar, bio, company, location, repos, \
         followers, etc. Uses GITHUB_TOKEN for authentication. No arguments needed."
    }
    
    fn read_only() -> bool {
        true  // Only reads data
    }
    
    fn destructive() -> bool {
        false  // No destructive operations
    }
    
    fn idempotent() -> bool {
        true  // Same request returns same result
    }
    
    fn open_world() -> bool {
        true  // Calls external GitHub API
    }
    
    async fn execute(&self, _args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| McpError::Other(anyhow::anyhow!(
                "GITHUB_TOKEN environment variable not set"
            )))?;
        
        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {}", e)))?;
        
        // Call API wrapper (returns AsyncTask<Result<Author, GitHubError>>)
        let task_result = client.get_me().await;
        
        // Handle outer Result (channel error)
        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;
        
        // Handle inner Result (GitHub API error)
        let user = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;
        
        // Build human-readable summary
        let login = user.get("login").and_then(|l| l.as_str()).unwrap_or("Unknown");
        let name = user.get("name").and_then(|n| n.as_str()).unwrap_or("No name");
        let email = user.get("email")
            .and_then(|e| e.as_str())
            .map(|e| format!("\nEmail: {}", e))
            .unwrap_or_default();
        let bio = user.get("bio")
            .and_then(|b| b.as_str())
            .map(|b| format!("\nBio: {}", b))
            .unwrap_or_default();
        let company = user.get("company")
            .and_then(|c| c.as_str())
            .map(|c| format!("\nCompany: {}", c))
            .unwrap_or_default();
        let location = user.get("location")
            .and_then(|l| l.as_str())
            .map(|l| format!("\nLocation: {}", l))
            .unwrap_or_default();
        let blog = user.get("blog")
            .and_then(|b| b.as_str())
            .filter(|b| !b.is_empty())
            .map(|b| format!("\nWebsite: {}", b))
            .unwrap_or_default();
        
        let public_repos = user.get("public_repos").and_then(|r| r.as_u64()).unwrap_or(0);
        let followers = user.get("followers").and_then(|f| f.as_u64()).unwrap_or(0);
        let following = user.get("following").and_then(|f| f.as_u64()).unwrap_or(0);
        let created_at = user.get("created_at").and_then(|c| c.as_str()).unwrap_or("Unknown");
        let html_url = user.get("html_url").and_then(|u| u.as_str()).unwrap_or("N/A");

        let summary = format!(
            "👤 Authenticated as: @{}\n\n\
             Name: {}{}{}{}{}{}\n\n\
             Stats:\n\
             • Public repos: {}\n\
             • Followers: {}\n\
             • Following: {}\n\
             • Account created: {}\n\n\
             Profile: {}",
            login,
            name,
            email,
            bio,
            company,
            location,
            blog,
            public_repos,
            followers,
            following,
            created_at,
            html_url
        );
        
        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&user)
            .unwrap_or_else(|_| "{}".to_string());
        
        Ok(vec![
            Content::text(summary),
            Content::text(json_str),
        ])
    }
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "include_use_cases".to_string(),
                title: None,
                description: Some(
                    "Include practical use cases and examples of when to use get_me (default: true)"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "include_fields".to_string(),
                title: None,
                description: Some(
                    "Include detailed explanation of all returned user fields (default: true)"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "include_auth_details".to_string(),
                title: None,
                description: Some(
                    "Include authentication requirements and token setup details (default: true)"
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
                    "How do I get information about the authenticated user?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use get_me to retrieve your GitHub user details:\n\n\
                     get_me({})\n\n\
                     Returns information about the user associated with the GITHUB_TOKEN:\n\
                     - login: Username\n\
                     - id: User ID\n\
                     - name: Display name\n\
                     - email: Email address\n\
                     - avatar_url: Profile image\n\
                     - bio: User bio\n\
                     - company: Company name\n\
                     - location: Location\n\
                     - blog: Website URL\n\
                     - public_repos: Number of public repositories\n\
                     - followers: Follower count\n\
                     - following: Following count\n\
                     - created_at: Account creation date\n\n\
                     Use this to:\n\
                     - Verify authentication is working\n\
                     - Get your username for other operations\n\
                     - Display user information\n\
                     - Check account details\n\n\
                     No arguments needed - automatically uses GITHUB_TOKEN.\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token must be valid and not expired"
                ),
            },
        ])
    }
}
