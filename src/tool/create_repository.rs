use anyhow;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::CreateRepositoryArgs;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for creating a new repository
pub struct CreateRepositoryTool;

impl Tool for CreateRepositoryTool {
    type Args = CreateRepositoryArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "github_create_repository"
    }

    fn description() -> &'static str {
        "Create a new repository under the authenticated user's account"
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false
    }

    fn open_world() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let task_result = client
            .create_repository(args.name.clone(), args.description.clone(), args.private, args.auto_init)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let repository =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build human-readable summary
        let visibility = if args.private.unwrap_or(false) { "🔒 Private" } else { "🌍 Public" };
        let initialized = if args.auto_init.unwrap_or(false) { "✓ Initialized with README" } else { "Empty (no initial commit)" };
        
        let description_text = args.description
            .as_ref()
            .map(|d| format!("\nDescription: {}", d))
            .unwrap_or_default();

        let summary = format!(
            "✨ Created repository: {}\n\n\
             Visibility: {}\n\
             Status: {}{}\n\n\
             Clone URLs:\n\
             • HTTPS: {}\n\
             • SSH: {}\n\n\
             View on GitHub: {}",
            repository.full_name.as_deref().unwrap_or(&args.name),
            visibility,
            initialized,
            description_text,
            repository.clone_url.as_ref().map(|u| u.as_str()).unwrap_or("N/A"),
            repository.ssh_url.as_ref().map(|u| u.as_str()).unwrap_or("N/A"),
            repository.html_url.as_ref().map(|u| u.as_str()).unwrap_or("N/A")
        );

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&repository)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(vec![
            Content::text(summary),
            Content::text(json_str),
        ])
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Repository Creation Examples

## Basic Repository
To create a simple public repository:

```json
{
  "name": "my-new-project"
}
```

## Repository with Description
To create a repository with a description:

```json
{
  "name": "awesome-project",
  "description": "An awesome project that does amazing things"
}
```

## Private Repository
To create a private repository:

```json
{
  "name": "secret-project",
  "description": "A private project",
  "private": true
}
```

## Initialize with README
To create a repository with an initial README file:

```json
{
  "name": "quick-start",
  "description": "A project with README",
  "auto_init": true
}
```

## Complete Example
To create a fully configured repository:

```json
{
  "name": "my-awesome-library",
  "description": "A comprehensive library for doing X",
  "private": false,
  "auto_init": true
}
```

## Repository Naming Rules

**Valid names:**
- Alphanumeric characters, hyphens, and underscores
- Cannot start with a hyphen
- Maximum 100 characters
- Examples: `my-project`, `awesome_lib`, `Project123`

**Invalid names:**
- Names with spaces: "my project" ❌
- Names starting with hyphen: "-project" ❌
- Special characters: "my@project" ❌

## Common Use Cases

1. **Quick Prototype**: Create public repo without README for pushing existing code
2. **New Project**: Create with README to have an initial commit
3. **Private Work**: Create private repo for confidential projects
4. **Open Source**: Create public repo with description for community discovery

## Best Practices

- **Descriptive Names**: Use clear, descriptive repository names
- **Add Description**: Always provide a description to help others understand the purpose
- **Private by Default**: Consider starting private and making public later
- **Initialize with README**: Use auto_init if you want to clone immediately
- **Consistent Naming**: Follow your organization's naming conventions

## What Happens After Creation

- Repository is created under your GitHub account
- If auto_init is true, a README.md file is created
- You can immediately clone the repository
- You can push existing code if not initialized
- Repository URL: `https://github.com/YOUR_USERNAME/REPO_NAME`

## Response Information

The response includes:
- **id**: Unique repository ID
- **name**: Repository name
- **full_name**: Owner/repo format
- **html_url**: Web URL to view the repository
- **clone_url**: HTTPS clone URL
- **ssh_url**: SSH clone URL
- **private**: Whether repository is private
- **created_at**: Creation timestamp
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
