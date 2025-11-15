use anyhow;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::{CreatePullRequestArgs, GITHUB_CREATE_PULL_REQUEST};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;
use crate::github::CreatePullRequestRequest;

/// Tool for creating a new pull request in a GitHub repository
pub struct CreatePullRequestTool;

impl Tool for CreatePullRequestTool {
    type Args = CreatePullRequestArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        GITHUB_CREATE_PULL_REQUEST
    }

    fn description() -> &'static str {
        "Create a new pull request in a GitHub repository"
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

        let request = CreatePullRequestRequest {
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            title: args.title,
            body: args.body,
            head: args.head.clone(),
            base: args.base.clone(),
            draft: args.draft,
            maintainer_can_modify: args.maintainer_can_modify,
        };

        let task_result = client.create_pull_request(request).await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let pr =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build dual-content response
        let mut contents = Vec::new();

        // Content[0]: Human-Readable Summary
        let summary = format!(
            "✓ Created pull request #{}\n\n\
             Repository: {}/{}\n\
             Title: {}\n\
             Head: {} → Base: {}\n\
             State: {}\n\
             Draft: {}\n\n\
             View on GitHub: {}",
            pr.number,
            args.owner,
            args.repo,
            pr.title.as_ref().map_or("(no title)", |t| t.as_str()),
            args.head,
            args.base,
            pr.state.as_ref().map(|s| format!("{:?}", s)).unwrap_or_else(|| "unknown".to_string()),
            pr.draft.unwrap_or(false),
            pr.html_url.as_ref().map_or("", |url| url.as_str())
        );
        contents.push(Content::text(summary));

        // Content[1]: Machine-Parseable JSON
        let json_str = serde_json::to_string_pretty(&pr)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Pull Request Creation Examples

## Basic Pull Request
To create a simple pull request:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "title": "Add new feature",
  "body": "This PR adds a new feature that...",
  "head": "feature-branch",
  "base": "main"
}
```

## Draft Pull Request
To create a draft pull request:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "title": "WIP: Experimental feature",
  "body": "This is still in progress...",
  "head": "experimental",
  "base": "develop",
  "draft": true
}
```

## Cross-Fork Pull Request
When creating a PR from a fork:

```json
{
  "owner": "upstream-owner",
  "repo": "project",
  "title": "Fix bug in authentication",
  "body": "Fixes #123\n\nThis PR resolves the authentication issue by...",
  "head": "fork-owner:fix-auth-bug",
  "base": "main",
  "maintainer_can_modify": true
}
```

## Common Use Cases

1. **Feature Development**: Create a PR from a feature branch to main/develop
2. **Bug Fixes**: Create a PR with fixes and link to issues using "Fixes #123"
3. **Documentation**: Create PRs for documentation updates
4. **Draft PRs**: Use draft mode for work-in-progress that needs early feedback
5. **Cross-Fork Contributions**: Contribute to upstream repositories from your fork

## Best Practices

- Write clear, descriptive titles
- Include detailed descriptions explaining the changes
- Reference related issues using "Fixes #123" or "Closes #456"
- Use draft mode for incomplete work
- Enable maintainer modifications for easier collaboration
- Follow the repository's contribution guidelines
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
