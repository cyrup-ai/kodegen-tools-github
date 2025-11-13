use anyhow;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::GetCommitArgs;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for getting detailed commit information
pub struct GetCommitTool;

impl Tool for GetCommitTool {
    type Args = GetCommitArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "github_get_commit"
    }

    fn description() -> &'static str {
        "Get detailed information about a specific commit"
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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let task_result = client
            .get_commit(
                args.owner.clone(),
                args.repo.clone(),
                args.commit_sha.clone(),
                args.page,
                args.per_page,
            )
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let commit =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build human-readable summary
        let commit_message = commit.get("commit")
            .and_then(|c| c.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("No message");
        
        let author_name = commit.get("commit")
            .and_then(|c| c.get("author"))
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown");
        
        let author_date = commit.get("commit")
            .and_then(|c| c.get("author"))
            .and_then(|a| a.get("date"))
            .and_then(|d| d.as_str())
            .unwrap_or("Unknown");
        
        let additions = commit.get("stats")
            .and_then(|s| s.get("additions"))
            .and_then(|a| a.as_u64())
            .unwrap_or(0);
        
        let deletions = commit.get("stats")
            .and_then(|s| s.get("deletions"))
            .and_then(|d| d.as_u64())
            .unwrap_or(0);
        
        let files_count = commit.get("files")
            .and_then(|f| f.as_array())
            .map(|f| f.len())
            .unwrap_or(0);

        let message_preview = if commit_message.len() > 100 {
            format!("{}...", &commit_message[..100])
        } else {
            commit_message.to_string()
        };

        let summary = format!(
            "📝 Commit: {}\n\n\
             Repository: {}/{}\n\
             Author: {}\n\
             Date: {}\n\n\
             Message:\n{}\n\n\
             Changes:\n\
             • Files changed: {}\n\
             • Additions: +{}\n\
             • Deletions: -{}\n\
             • Total: {} lines",
            &args.commit_sha[..7.min(args.commit_sha.len())],
            args.owner,
            args.repo,
            author_name,
            author_date,
            message_preview,
            files_count,
            additions,
            deletions,
            additions + deletions
        );

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&commit)
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
                r#"# GitHub Get Commit Examples

## Get Commit Details
To get detailed information about a specific commit:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "commit_sha": "abc123def456789abc123def456789abc123def4"
}
```

## Get Commit with Pagination for Files
For commits with many changed files, use pagination:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "commit_sha": "abc123def456",
  "page": 1,
  "per_page": 100
}
```

## Response Information

The response includes comprehensive commit details:

**Basic Information:**
- **sha**: Full commit SHA
- **commit**: Commit object with message, author, committer, tree
- **author**: GitHub user object (may be null for external commits)
- **committer**: GitHub user object
- **parents**: Array of parent commit SHAs
- **html_url**: Web URL to view the commit

**Change Statistics:**
- **stats**: Object with total additions, deletions, and changes
- **files**: Array of changed files with patches

**File Details (for each file):**
- **filename**: Path to the file
- **status**: Change type (added, modified, removed, renamed)
- **additions**: Lines added
- **deletions**: Lines deleted
- **changes**: Total changes
- **patch**: The actual diff content (if available)

## Common Use Cases

1. **Code Review**: Examine specific commit changes in detail
2. **Debugging**: Investigate when and how a bug was introduced
3. **Audit Trail**: Review security-sensitive changes
4. **Documentation**: Generate change logs with detailed diffs
5. **Analysis**: Calculate code churn metrics
6. **Verification**: Confirm specific changes were made
7. **Integration**: Trigger workflows based on commit content

## Understanding Commit SHAs

**Full SHA:**
- 40 hexadecimal characters
- Example: `abc123def456789abc123def456789abc123def4`
- Uniquely identifies a commit

**Short SHA:**
- First 7-10 characters
- Example: `abc123d`
- Can be used in many GitHub APIs
- This tool accepts both full and short SHAs

**Getting SHAs:**
- Use `list_commits` to get recent commit SHAs
- From PR file changes in pull request APIs
- From branch information in `list_branches`
- From GitHub web UI commit history

## Working with Diffs

The patch field contains standard unified diff format:
- Lines starting with `-` are removed
- Lines starting with `+` are added
- Lines starting with `@@` show line numbers
- Context lines show surrounding code

## Pagination for Large Commits

Some commits change many files:
- Use page and per_page to paginate through files
- Default is 30 files per page
- Maximum is 100 files per page
- Useful for merge commits or large refactorings

## Best Practices

- Cache commit information to avoid repeated API calls
- Use short SHAs when displaying to users
- Check the stats object for commit size before processing files
- Handle null author/committer (can occur for old or external commits)
- Be aware of rate limits when fetching many commits
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
