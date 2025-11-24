use anyhow;
use kodegen_mcp_schema::github::{MergePullRequestArgs, GITHUB_MERGE_PULL_REQUEST};
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;

/// Tool for merging a pull request
pub struct MergePullRequestTool;

impl Tool for MergePullRequestTool {
    type Args = MergePullRequestArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        GITHUB_MERGE_PULL_REQUEST
    }

    fn description() -> &'static str {
        "Merge a pull request in a GitHub repository"
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true
    }

    fn idempotent() -> bool {
        false
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

        let options = crate::MergePullRequestOptions {
            commit_title: args.commit_title,
            commit_message: args.commit_message,
            sha: args.sha,
            merge_method: args.merge_method.clone(),
        };

        let task_result = client
            .merge_pull_request(args.owner.clone(), args.repo.clone(), args.pr_number, options)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let merge_result =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build dual-content response
        let mut contents = Vec::new();

        // Content[0]: Human-Readable Summary
        let sha = merge_result.get("sha")
            .and_then(|s| s.as_str())
            .map(|s| if s.len() > 7 { &s[..7] } else { s })
            .unwrap_or("(unknown)");

        let merge_method = args.merge_method.as_ref().map_or("merge", |m| m.as_str());

        let summary = format!(
            "\x1b[33m PR Merged: #{}\x1b[0m\n\
             ✓ Method: {} · SHA: {}",
            args.pr_number,
            merge_method,
            sha
        );
        contents.push(Content::text(summary));

        // Content[1]: Machine-Parseable JSON
        let json_str = serde_json::to_string_pretty(&merge_result)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Pull Request Merge Examples

## Basic Merge
To merge a pull request with default settings:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42
}
```

## Merge with Custom Commit Message
To merge with a custom commit title and message:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "commit_title": "Feature: Add user authentication",
  "commit_message": "This commit adds OAuth2 authentication support.\n\nCloses #123\nCloses #124"
}
```

## Squash Merge
To merge all commits into a single commit:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "merge_method": "squash",
  "commit_title": "Add authentication feature"
}
```

## Rebase Merge
To rebase and merge commits onto the base branch:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "merge_method": "rebase"
}
```

## Safe Merge with SHA Check
To ensure the PR hasn't changed since you last reviewed it:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "sha": "6dcb09b5b57875f334f61aebed695e2e4193db5e",
  "commit_title": "Merge feature after review"
}
```

## Merge Methods

- **merge** (default): Creates a merge commit, preserving all commits from the PR
- **squash**: Combines all commits into a single commit
- **rebase**: Rebases commits onto the base branch without a merge commit

## Common Use Cases

1. **Standard Merge**: Merge approved PRs with default settings
2. **Clean History**: Use squash merge for feature branches with many small commits
3. **Linear History**: Use rebase merge to maintain a linear commit history
4. **Custom Messages**: Provide detailed commit messages for important merges
5. **Safe Merging**: Use SHA verification to prevent merging outdated code

## Best Practices

- **Review First**: Always review and approve PRs before merging
- **Check CI**: Ensure all checks pass before merging
- **Choose Method**: Select merge method based on project conventions
- **Update Message**: Provide clear commit messages, especially for squash merges
- **Use SHA Check**: For critical merges, verify the exact commit being merged
- **Clean Up**: Delete the branch after merging (done automatically in many repos)

## Safety Notes

- This is a **destructive operation** - merged code becomes part of the base branch
- Cannot be easily undone (requires revert commits)
- Ensure proper testing and review before merging
- Use SHA parameter to prevent race conditions
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "merge_strategy".to_string(),
                title: None,
                description: Some(
                    "Specific merge strategy to focus examples on: 'merge' (default, creates merge commit), \
                     'squash' (combines commits), or 'rebase' (linear history)".to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "Focus area for teaching: 'basic' (simple merges), 'advanced' (custom messages and SHA \
                     verification), 'safety' (best practices and verification), or 'all' (comprehensive)".to_string(),
                ),
                required: Some(false),
            },
        ]
    }
}
