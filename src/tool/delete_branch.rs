use anyhow;
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext};
use kodegen_mcp_schema::github::{DeleteBranchArgs, GITHUB_DELETE_BRANCH};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;

/// Tool for deleting a branch
pub struct DeleteBranchTool;

impl Tool for DeleteBranchTool {
    type Args = DeleteBranchArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        GITHUB_DELETE_BRANCH
    }

    fn description() -> &'static str {
        "Delete a branch from a GitHub repository"
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

        let task_result = client
            .delete_branch(args.owner.clone(), args.repo.clone(), args.branch_name.clone())
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build human-readable summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[31m Branch Deleted: {}\x1b[0m\n\
             󰋼 Repo: {}/{} · Destructive operation completed",
            args.branch_name,
            args.owner,
            args.repo
        );

        // Create success metadata
        let metadata = serde_json::json!({
            "branch": args.branch_name,
            "owner": args.owner,
            "repo": args.repo,
            "deleted": true
        });

        let json_str = serde_json::to_string_pretty(&metadata)
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
                r#"# GitHub Delete Branch Examples

## Delete a Feature Branch
To delete a branch that is no longer needed:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "branch_name": "feature/old-feature"
}
```

## Delete After Merge
After merging a pull request, clean up the branch:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "branch_name": "feature/completed-feature"
}
```

## Common Use Cases

### 1. Cleanup After Pull Request Merge
```
1. Merge pull request to main
2. Delete the feature branch
3. Keep repository clean
```

### 2. Remove Abandoned Branches
```
1. List all branches
2. Identify stale/abandoned branches
3. Delete branches no longer in use
```

### 3. Release Branch Cleanup
```
1. Complete release process
2. Tag the release
3. Delete the release branch
```

## Important Safety Notes

**DESTRUCTIVE OPERATION:**
- This permanently deletes the branch from the remote repository
- Once deleted, the branch reference is gone
- Commits are not deleted (they remain in git history if referenced elsewhere)
- This action cannot be undone through the API

**Cannot Delete:**
- The default branch (usually `main` or `master`)
- Protected branches (as configured in repository settings)
- Branches you don't have permission to delete

**Best Practices:**
- Verify the branch name before deletion
- Ensure the branch has been merged if it contains important work
- Check that no open pull requests reference the branch
- Consider branch protection rules for important branches

## Permissions Required

- **Write access** to the repository
- **Admin access** may be required for certain branches
- Must not be the default branch
- Must not be protected (unless you have override permissions)

## Workflow Integration

### After PR Merge Workflow
```
1. Get pull request status (verify it's merged)
2. Get the head branch name from PR
3. Delete the head branch
4. Confirm deletion
```

### Batch Cleanup Workflow
```
1. List all branches
2. Filter for merged/stale branches
3. Review list carefully
4. Delete branches one by one
```

## Response Information

The response confirms:
- **branch**: Name of the deleted branch
- **owner**: Repository owner
- **repo**: Repository name
- **deleted**: Boolean confirmation (true)

## Troubleshooting

- **"Reference does not exist"**: Branch name is incorrect or already deleted
- **"Not Found"**: Repository doesn't exist or no write access
- **"Cannot delete protected branch"**: Branch has protection rules enabled
- **"Cannot delete default branch"**: Attempting to delete main/master branch
- **"Validation Failed"**: Branch name format is invalid

## Alternative: Local Branch Deletion

This tool deletes **remote** branches only. For local branches, use git commands:
```bash
git branch -d branch-name    # Safe delete (merged only)
git branch -D branch-name    # Force delete (any state)
```

## Recovery

If you accidentally delete a branch:
1. Commits are still in git history
2. Find the commit SHA from reflog or PR history
3. Use `create_branch` with the SHA to recreate
4. Branch protection can prevent accidental deletion
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
