//! GitHub authenticated user retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{GetMeArgs, GetMePromptArgs, GITHUB_GET_ME};
use kodegen_mcp_schema::ToolArgs;
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};

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

    async fn execute(&self, _args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as ToolArgs>::Output>, McpError> {
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

        // Extract fields from JSON Value
        let login = user.get("login")
            .and_then(|l| l.as_str())
            .unwrap_or_default()
            .to_string();

        let id = user.get("id")
            .and_then(|i| i.as_u64())
            .unwrap_or(0);

        let name = user.get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        let email = user.get("email")
            .and_then(|e| e.as_str())
            .map(|s| s.to_string());

        let avatar_url = user.get("avatar_url")
            .and_then(|a| a.as_str())
            .unwrap_or_default()
            .to_string();

        let html_url = user.get("html_url")
            .and_then(|u| u.as_str())
            .unwrap_or_default()
            .to_string();

        let bio = user.get("bio")
            .and_then(|b| b.as_str())
            .map(|s| s.to_string());

        let location = user.get("location")
            .and_then(|l| l.as_str())
            .map(|s| s.to_string());

        let company = user.get("company")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let followers = user.get("followers")
            .and_then(|f| f.as_u64())
            .unwrap_or(0) as u32;

        let following = user.get("following")
            .and_then(|f| f.as_u64())
            .unwrap_or(0) as u32;

        let public_repos = user.get("public_repos")
            .and_then(|r| r.as_u64())
            .unwrap_or(0) as u32;

        let created_at = user.get("created_at")
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string();

        let output = kodegen_mcp_schema::github::GitHubGetMeOutput {
            success: true,
            login: login.clone(),
            id,
            name: name.clone(),
            email: email.clone(),
            avatar_url: avatar_url.clone(),
            html_url: html_url.clone(),
            bio: bio.clone(),
            location: location.clone(),
            company: company.clone(),
            followers,
            following,
            public_repos,
            created_at: created_at.clone(),
        };

        let display = format!(
            "👤 GitHub Profile\n\n\
             Username: @{}\n\
             Name: {}\n\
             Email: {}\n\
             Bio: {}\n\
             Location: {}\n\
             Company: {}\n\
             Followers: {} | Following: {}\n\
             Public Repos: {}\n\
             Created: {}\n\
             Profile: {}",
            output.login,
            output.name.as_deref().unwrap_or("N/A"),
            output.email.as_deref().unwrap_or("N/A"),
            output.bio.as_deref().unwrap_or("No bio"),
            output.location.as_deref().unwrap_or("Unknown"),
            output.company.as_deref().unwrap_or("N/A"),
            output.followers,
            output.following,
            output.public_repos,
            output.created_at,
            output.html_url
        );

        Ok(ToolResponse::new(display, output))
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
