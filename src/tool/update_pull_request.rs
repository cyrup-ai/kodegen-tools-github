use anyhow;
use kodegen_mcp_schema::github::{UpdatePullRequestArgs, UpdatePullRequestPromptArgs, GITHUB_UPDATE_PULL_REQUEST};
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;

/// Tool for updating an existing pull request
pub struct UpdatePullRequestTool;

impl Tool for UpdatePullRequestTool {
    type Args = UpdatePullRequestArgs;
    type PromptArgs = UpdatePullRequestPromptArgs;

    fn name() -> &'static str {
        GITHUB_UPDATE_PULL_REQUEST
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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

        // Format state
        let state_str = pr.state.as_ref()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "unknown".to_string());

        // Build 2-line ANSI yellow output with Nerd Font icons
        let summary = format!(
            "\x1b[33m PR Updated: #{}\x1b[0m\n\
              State: {} · Title: {}",
            pr.number,
            state_str,
            pr.title.as_deref().unwrap_or("Untitled")
        );

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&pr)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(vec![
            Content::text(summary),
            Content::text(json_str),
        ])
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        // Build content based on example_type customization
        let content_text = match args.example_type.as_deref() {
            Some("title") => {
                r#"# GitHub Pull Request Update: Title Examples

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

## Common Title Updates

- Remove "WIP:" prefix when ready for review
- Add scope/prefix for organization (e.g., "feat: ", "fix: ")
- Update to reflect final implementation vs. initial proposal
- Clarify the PR's purpose after discussion

## Requirements

- Title must not be empty (if provided)
- Maximum length: 255 characters
- Use clear, descriptive language
"#
            }
            Some("body") => {
                r#"# GitHub Pull Request Update: Body Examples

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

## Common Body Updates

- Add implementation details as work progresses
- Link to related issues using "Closes #123" or "Fixes #456"
- Update checklist items as features are completed
- Add screenshots or diagrams after initial submission
- Document breaking changes or migration steps

## Requirements

- Body supports Markdown formatting
- You can @mention users and reference issues
- Use line breaks and lists for clarity
- Keep descriptions up-to-date with current state
"#
            }
            Some("state") => {
                r#"# GitHub Pull Request Update: State Examples

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

## Requirements

- state must be either "open" or "closed"
- Only PR authors and repository maintainers can change state
- Closing a PR does not delete it - it can be reopened
- Reopen PR only if work will resume
"#
            }
            Some("base") => {
                r#"# GitHub Pull Request Update: Base Branch Examples

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

## Common Base Branch Changes

- Retarget from main to a release branch
- Move from develop to main when ready
- Change to a feature branch for related work
- Update when project branching strategy changes

## Requirements

- The target base branch must exist
- You cannot change base to the same branch as the head
- May cause merge conflicts - resolve after changing
- Both target and current base must have compatible history
- Only PR authors and maintainers can change the base
"#
            }
            Some("maintainer") => {
                r#"# GitHub Pull Request Update: Maintainer Access Examples

## Update Maintainer Modify Permission
To allow or disallow maintainers to modify the pull request:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "maintainer_can_modify": true
}
```

## Use Cases

- Set to `true` if maintainers should be able to push fixes directly
- Set to `false` if you want exclusive control over the PR commits
- Useful for collaborative code review workflows
- Help maintainers quickly address requested changes

## Requirements

- Only PR authors and repository admins can change this setting
- Default behavior depends on repository settings
- When true, maintainers with push access can add commits
"#
            }
            Some("combined") => {
                r#"# GitHub Pull Request Update: Multiple Fields Examples

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

## Advanced Combined Updates

Update title, body, and state together:
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "title": "Ready: Feature implementation complete",
  "body": "All requested changes have been made and tests pass.",
  "state": "open"
}
```

Change target and permissions:
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "base": "release-v1.0",
  "maintainer_can_modify": true
}
```

## Best Practices

- Only include fields that actually need to change
- Update title and body together for consistency
- Test changes before submitting
- Notify reviewers if making significant changes
"#
            }
            _ => {
                // Default: Show all examples
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
"#
            }
        };

        // Append gotchas section if requested
        let final_content = if args.show_gotchas.unwrap_or(false) {
            format!(
                "{}\n\n## Common Gotchas and Error Cases\n\n\
                 ### Cannot Update Non-Existent PR\n\
                 Error: 404 Not Found if PR number doesn't exist\n\
                 - Verify the owner, repo, and pr_number are correct\n\
                 - Check that the repository is accessible with GITHUB_TOKEN\n\n\
                 ### Insufficient Permissions\n\
                 Error: 403 Forbidden when lacking required permissions\n\
                 - PR authors can only update their own PRs (unless admin)\n\
                 - GITHUB_TOKEN must have 'repo' scope\n\
                 - Private repos require 'repo' scope (not 'public_repo')\n\n\
                 ### Invalid State Values\n\
                 Error: Unrecognized value for 'state'\n\
                 - Use only \"open\" or \"closed\" (lowercase)\n\
                 - Check for typos or case sensitivity issues\n\n\
                 ### Base Branch Conflicts\n\
                 Error: Cannot set base to the same as head branch\n\
                 - Head and base branches must be different\n\
                 - Target base must exist in the repository\n\
                 - May create merge conflicts - resolve them before/after update\n\n\
                 ### Invalid Base Branch\n\
                 Error: Invalid value for base branch\n\
                 - Verify the target branch exists\n\
                 - Check branch name spelling and case\n\
                 - Ensure you have access to the branch\n\n\
                 ### Race Conditions\n\
                 Error: Update fails unexpectedly after successful request\n\
                 - Another process may have updated the PR simultaneously\n\
                 - Consider retrying with fresh PR data\n\n\
                 ### Token Expiration\n\
                 Error: 401 Unauthorized after previously working\n\
                 - GITHUB_TOKEN may have expired\n\
                 - Regenerate personal access token if needed\n\
                 - Check token scopes and expiration date",
                content_text
            )
        } else {
            content_text.to_string()
        };

        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(final_content),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "example_type".to_string(),
                title: None,
                description: Some(
                    "Type of update example to focus on: 'title' for title-only updates, \
                     'body' for description/body updates, 'state' for open/close operations, \
                     'base' for branch retargeting, 'maintainer' for maintainer permissions, \
                     'combined' for multi-field updates, or omit for all examples together"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "show_gotchas".to_string(),
                title: None,
                description: Some(
                    "Set to true to include a comprehensive section on common gotchas, \
                     error cases, permission issues, and edge cases. Useful for deeper learning \
                     about what can go wrong and how to handle errors"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }
}
