use anyhow;
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext};
use kodegen_mcp_schema::github::{GetCommitArgs, GetCommitPromptArgs, GITHUB_GET_COMMIT};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;

/// Tool for getting detailed commit information
pub struct GetCommitTool;

impl Tool for GetCommitTool {
    type Args = GetCommitArgs;
    type PromptArgs = GetCommitPromptArgs;

    fn name() -> &'static str {
        GITHUB_GET_COMMIT
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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
        let author_name = commit.commit.author.as_ref()
            .map(|a| a.name.as_str())
            .unwrap_or("Unknown");
        
        let additions = commit.stats.as_ref()
            .and_then(|s| s.additions)
            .unwrap_or(0);
        
        let deletions = commit.stats.as_ref()
            .and_then(|s| s.deletions)
            .unwrap_or(0);
        
        let files_count = commit.files.as_ref()
            .map(|f| f.len())
            .unwrap_or(0);

        let sha_short = &args.commit_sha[..7.min(args.commit_sha.len())];
        let summary = format!(
            "\x1b[36m Commit: {}\x1b[0m\n  Author: {} · Files: {} · +{} -{}",
            sha_short,
            author_name,
            files_count,
            additions,
            deletions
        );

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&commit)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(vec![
            Content::text(summary),
            Content::text(json_str),
        ])
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        // Determine what to include based on args
        let include_response = args.explain_response.unwrap_or(true);
        let include_pagination = args.explain_pagination.unwrap_or(true);
        let include_diffs = args.explain_diffs.unwrap_or(true);
        let include_use_cases = args.explain_use_cases.unwrap_or(true);

        let mut content = String::from("# GitHub Get Commit Examples\n\n");

        // Basic usage (always included)
        content.push_str("## Get Commit Details\n");
        content.push_str("To get detailed information about a specific commit:\n\n");
        content.push_str("```json\n");
        content.push_str("{\n");
        content.push_str("  \"owner\": \"octocat\",\n");
        content.push_str("  \"repo\": \"hello-world\",\n");
        content.push_str("  \"commit_sha\": \"abc123def456789abc123def456789abc123def4\"\n");
        content.push_str("}\n");
        content.push_str("```\n\n");

        if include_pagination {
            content.push_str("## Get Commit with Pagination for Files\n");
            content.push_str("For commits with many changed files, use pagination:\n\n");
            content.push_str("```json\n");
            content.push_str("{\n");
            content.push_str("  \"owner\": \"octocat\",\n");
            content.push_str("  \"repo\": \"hello-world\",\n");
            content.push_str("  \"commit_sha\": \"abc123def456\",\n");
            content.push_str("  \"page\": 1,\n");
            content.push_str("  \"per_page\": 100\n");
            content.push_str("}\n");
            content.push_str("```\n\n");
        }

        if include_response {
            content.push_str("## Response Information\n\n");
            content.push_str("The response includes comprehensive commit details:\n\n");
            content.push_str("**Basic Information:**\n");
            content.push_str("- **sha**: Full commit SHA\n");
            content.push_str("- **commit**: Commit object with message, author, committer, tree\n");
            content.push_str("- **author**: GitHub user object (may be null for external commits)\n");
            content.push_str("- **committer**: GitHub user object\n");
            content.push_str("- **parents**: Array of parent commit SHAs\n");
            content.push_str("- **html_url**: Web URL to view the commit\n\n");
            content.push_str("**Change Statistics:**\n");
            content.push_str("- **stats**: Object with total additions, deletions, and changes\n");
            content.push_str("- **files**: Array of changed files with patches\n\n");
            content.push_str("**File Details (for each file):**\n");
            content.push_str("- **filename**: Path to the file\n");
            content.push_str("- **status**: Change type (added, modified, removed, renamed)\n");
            content.push_str("- **additions**: Lines added\n");
            content.push_str("- **deletions**: Lines deleted\n");
            content.push_str("- **changes**: Total changes\n");
            content.push_str("- **patch**: The actual diff content (if available)\n\n");
        }

        if include_use_cases {
            content.push_str("## Common Use Cases\n\n");
            content.push_str("1. **Code Review**: Examine specific commit changes in detail\n");
            content.push_str("2. **Debugging**: Investigate when and how a bug was introduced\n");
            content.push_str("3. **Audit Trail**: Review security-sensitive changes\n");
            content.push_str("4. **Documentation**: Generate change logs with detailed diffs\n");
            content.push_str("5. **Analysis**: Calculate code churn metrics\n");
            content.push_str("6. **Verification**: Confirm specific changes were made\n");
            content.push_str("7. **Integration**: Trigger workflows based on commit content\n\n");
        }

        // Commit SHA guidance (always included)
        content.push_str("## Understanding Commit SHAs\n\n");
        content.push_str("**Full SHA:**\n");
        content.push_str("- 40 hexadecimal characters\n");
        content.push_str("- Example: `abc123def456789abc123def456789abc123def4`\n");
        content.push_str("- Uniquely identifies a commit\n\n");
        content.push_str("**Short SHA:**\n");
        content.push_str("- First 7-10 characters\n");
        content.push_str("- Example: `abc123d`\n");
        content.push_str("- Can be used in many GitHub APIs\n");
        content.push_str("- This tool accepts both full and short SHAs\n\n");
        content.push_str("**Getting SHAs:**\n");
        content.push_str("- Use `list_commits` to get recent commit SHAs\n");
        content.push_str("- From PR file changes in pull request APIs\n");
        content.push_str("- From branch information in `list_branches`\n");
        content.push_str("- From GitHub web UI commit history\n\n");

        if include_diffs {
            content.push_str("## Working with Diffs\n\n");
            content.push_str("The patch field contains standard unified diff format:\n");
            content.push_str("- Lines starting with `-` are removed\n");
            content.push_str("- Lines starting with `+` are added\n");
            content.push_str("- Lines starting with `@@` show line numbers\n");
            content.push_str("- Context lines show surrounding code\n\n");
        }

        if include_pagination {
            content.push_str("## Pagination for Large Commits\n\n");
            content.push_str("Some commits change many files:\n");
            content.push_str("- Use page and per_page to paginate through files\n");
            content.push_str("- Default is 30 files per page\n");
            content.push_str("- Maximum is 100 files per page\n");
            content.push_str("- Useful for merge commits or large refactorings\n\n");
        }

        // Best practices (always included)
        content.push_str("## Best Practices\n\n");
        content.push_str("- Cache commit information to avoid repeated API calls\n");
        content.push_str("- Use short SHAs when displaying to users\n");
        content.push_str("- Check the stats object for commit size before processing files\n");
        content.push_str("- Handle null author/committer (can occur for old or external commits)\n");
        content.push_str("- Be aware of rate limits when fetching many commits\n");

        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(content),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "explain_response".to_string(),
                title: Some("Response Structure".to_string()),
                description: Some("Include detailed explanation of response structure and fields (sha, commit, stats, files)".to_string()),
                required: Some(false),
            },
            PromptArgument {
                name: "explain_pagination".to_string(),
                title: Some("Pagination".to_string()),
                description: Some("Include guidance on handling commits with many files using page/per_page parameters".to_string()),
                required: Some(false),
            },
            PromptArgument {
                name: "explain_diffs".to_string(),
                title: Some("Diff Format".to_string()),
                description: Some("Include explanation of unified diff format and how to interpret patch content".to_string()),
                required: Some(false),
            },
            PromptArgument {
                name: "explain_use_cases".to_string(),
                title: Some("Use Cases".to_string()),
                description: Some("Include common use cases like code review, debugging, and audit trails".to_string()),
                required: Some(false),
            },
        ]
    }
}
