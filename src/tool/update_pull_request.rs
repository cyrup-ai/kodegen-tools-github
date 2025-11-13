use anyhow;
use kodegen_mcp_schema::github::UpdatePullRequestArgs;
use kodegen_mcp_tool::{McpError, Tool};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for updating an existing pull request
pub struct UpdatePullRequestTool;

impl Tool for UpdatePullRequestTool {
    type Args = UpdatePullRequestArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "github_update_pull_request"
    }

    fn description() -> &'static str {
        "Update an existing pull request in a GitHub repository"
    }

    fn read_only() -> bool {
        false
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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Convert state string to octocrab State enum
        let state = args
            .state
            .as_ref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "open" => Some(octocrab::params::pulls::State::Open),
                "closed" => Some(octocrab::params::pulls::State::Closed),
                _ => None,
            });

        let options = crate::UpdatePullRequestOptions {
            title: args.title.clone(),
            body: args.body.clone(),
            state,
            base: args.base.clone(),
            maintainer_can_modify: args.maintainer_can_modify,
        };

        let task_result = client
            .update_pull_request(args.owner.clone(), args.repo.clone(), args.pr_number, options)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let pr =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build human-readable summary
        let mut changes = Vec::new();
        if args.title.is_some() {
            changes.push("title");
        }
        if args.body.is_some() {
            changes.push("description");
        }
        if args.state.is_some() {
            changes.push("state");
        }
        if args.base.is_some() {
            changes.push("base branch");
        }
        if args.maintainer_can_modify.is_some() {
            changes.push("maintainer access");
        }

        let changes_str = if changes.is_empty() {
            "no changes".to_string()
        } else {
            changes.join(", ")
        };

        let state_str = pr.state.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
        let state_emoji = if state_str == "open" { "🟢" } else { "⚫" };

        let summary = format!(
            "✏️ Updated PR #{}: {}\n\n\
             Repository: {}/{}\n\
             State: {} {}\n\
             Changes: {}\n\n\
             View on GitHub: {}",
            pr.number,
            pr.title.as_deref().unwrap_or("Untitled"),
            args.owner,
            args.repo,
            state_emoji,
            state_str,
            changes_str,
            pr.html_url.as_ref().map(|u| u.as_str()).unwrap_or("N/A")
        );

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&pr)
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
                r#"# GitHub Pull Request Update Examples

## Update Title
To update a pull request's title:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "title": "Updated: Add new feature with improvements"
}
```

## Update Body/Description
To update the pull request description:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "body": "Updated description:\n\n- Added feature X\n- Fixed bug Y\n- Improved performance\n\nCloses #123"
}
```

## Close a Pull Request
To close a pull request without merging:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "state": "closed"
}
```

## Reopen a Closed Pull Request
To reopen a previously closed pull request:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "state": "open"
}
```

## Change Base Branch
To retarget a pull request to a different base branch:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "base": "develop"
}
```

## Update Multiple Fields
To update several fields at once:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "title": "Complete: New authentication system",
  "body": "Fully implemented authentication with OAuth2 support",
  "maintainer_can_modify": false
}
```

## Common Use Cases

1. **Update Description**: Add more details or link to issues as work progresses
2. **Change Title**: Update to reflect current state (e.g., remove "WIP:")
3. **Close PRs**: Close pull requests that are no longer needed
4. **Reopen PRs**: Reopen closed PRs if work needs to continue
5. **Retarget Base**: Change the target branch if project structure changes
6. **Toggle Maintainer Access**: Enable/disable maintainer modifications

## Best Practices

- Only update fields that need to change (all fields except owner, repo, and pr_number are optional)
- Use clear, descriptive titles
- Update descriptions to keep them current with the changes
- Close PRs with clear explanations if they won't be merged
- Be cautious when changing the base branch
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
