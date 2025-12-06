//! GitHub authenticated user retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{GetMeArgs, GetMePrompts, GITHUB_GET_ME};
use kodegen_mcp_schema::ToolArgs;
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for getting information about the authenticated GitHub user
#[derive(Clone)]
pub struct GetMeTool;

impl Tool for GetMeTool {
    type Args = GetMeArgs;
    type Prompts = GetMePrompts;

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
}
