use anyhow;
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{DeleteBranchArgs, DeleteBranchPromptArgs, GITHUB_DELETE_BRANCH};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;

/// Tool for deleting a branch
pub struct DeleteBranchTool;

impl Tool for DeleteBranchTool {
    type Args = DeleteBranchArgs;
    type PromptArgs = DeleteBranchPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) 
        -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> 
    {
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

        let output = kodegen_mcp_schema::github::GitHubDeleteBranchOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            branch_name: args.branch_name.clone(),
            message: format!("Branch '{}' deleted successfully", args.branch_name),
        };

        let display = format!(
            "🗑️  Branch Deleted\n\n\
             Repository: {}/{}\n\
             Branch: {}",
            output.owner,
            output.repo,
            output.branch_name
        );

        Ok(ToolResponse::new(display, output))
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        // Extract arguments with defaults
        let scenario = args.scenario.as_deref().unwrap_or("all").to_lowercase();
        let include_permissions = args.include_permissions.unwrap_or(true);
        let include_recovery = args.include_recovery.unwrap_or(true);

        let mut content = String::from("# GitHub Delete Branch Guide\n\n");

        // Basic usage (always included)
        content.push_str("## Basic Usage\n\n");
        content.push_str("Delete a branch from a repository:\n\n");
        content.push_str("```json\n{\n");
        content.push_str("  \"owner\": \"octocat\",\n");
        content.push_str("  \"repo\": \"hello-world\",\n");
        content.push_str("  \"branch_name\": \"feature/old-feature\"\n");
        content.push_str("}\n```\n\n");

        // Scenario-specific content
        if scenario == "all" || scenario == "cleanup" {
            content.push_str("## Cleanup After Pull Request Merge\n\n");
            content.push_str("After merging a PR, clean up the feature branch:\n\n");
            content.push_str("**Workflow:**\n");
            content.push_str("1. Merge pull request to main\n");
            content.push_str("2. Get the head branch name from PR\n");
            content.push_str("3. Delete the feature branch\n");
            content.push_str("4. Confirm deletion\n\n");
            content.push_str("```json\n{\n");
            content.push_str("  \"owner\": \"octocat\",\n");
            content.push_str("  \"repo\": \"hello-world\",\n");
            content.push_str("  \"branch_name\": \"feature/completed-feature\"\n");
            content.push_str("}\n```\n\n");
        }

        if scenario == "all" || scenario == "workflow" {
            content.push_str("## Batch Cleanup Workflow\n\n");
            content.push_str("**Process:**\n");
            content.push_str("1. List all branches in repository\n");
            content.push_str("2. Filter for merged/stale branches\n");
            content.push_str("3. Review list carefully\n");
            content.push_str("4. Delete branches one by one\n\n");
            content.push_str("**Common targets:**\n");
            content.push_str("- Merged feature branches\n");
            content.push_str("- Abandoned experiment branches\n");
            content.push_str("- Old release branches (after tagging)\n\n");
        }

        if scenario == "all" || scenario == "protection" {
            content.push_str("## Important Safety Notes\n\n");
            content.push_str("**DESTRUCTIVE OPERATION:**\n");
            content.push_str("- Permanently deletes the branch from remote repository\n");
            content.push_str("- Branch reference is removed\n");
            content.push_str("- Commits remain in git history if referenced elsewhere\n");
            content.push_str("- Cannot be undone through the API\n\n");
            content.push_str("**Cannot Delete:**\n");
            content.push_str("- Default branch (usually `main` or `master`)\n");
            content.push_str("- Protected branches (unless you have override permissions)\n");
            content.push_str("- Branches without proper access\n\n");
            content.push_str("**Best Practices:**\n");
            content.push_str("- Verify branch name before deletion\n");
            content.push_str("- Ensure branch is merged if it contains important work\n");
            content.push_str("- Check no open PRs reference the branch\n");
            content.push_str("- Use branch protection rules for important branches\n\n");
        }

        // Permissions section (conditional)
        if include_permissions {
            content.push_str("## Permissions Required\n\n");
            content.push_str("- **Write access** to the repository\n");
            content.push_str("- **Admin access** may be required for certain branches\n");
            content.push_str("- Must not be the default branch\n");
            content.push_str("- Must not be protected (unless override permissions granted)\n");
            content.push_str("- Token needs `repo` scope for private repos\n\n");
        }

        // Response information (always included)
        content.push_str("## Response Information\n\n");
        content.push_str("Confirms deletion with:\n");
        content.push_str("- **branch**: Name of deleted branch\n");
        content.push_str("- **owner**: Repository owner\n");
        content.push_str("- **repo**: Repository name\n");
        content.push_str("- **deleted**: Boolean confirmation (true)\n\n");

        // Troubleshooting (always included)
        content.push_str("## Troubleshooting\n\n");
        content.push_str("**\"Reference does not exist\"**\n");
        content.push_str("- Branch name is incorrect or already deleted\n\n");
        content.push_str("**\"Not Found\"**\n");
        content.push_str("- Repository doesn't exist or no write access\n\n");
        content.push_str("**\"Cannot delete protected branch\"**\n");
        content.push_str("- Branch has protection rules enabled\n\n");
        content.push_str("**\"Cannot delete default branch\"**\n");
        content.push_str("- Attempting to delete main/master branch\n\n");
        content.push_str("**\"Validation Failed\"**\n");
        content.push_str("- Branch name format is invalid\n\n");

        // Recovery section (conditional)
        if include_recovery || scenario == "recovery" {
            content.push_str("## Recovery from Accidental Deletion\n\n");
            content.push_str("If you accidentally delete a branch:\n\n");
            content.push_str("1. **Commits are preserved**: They remain in git history\n");
            content.push_str("2. **Find the commit SHA**:\n");
            content.push_str("   - Check reflog: `git reflog`\n");
            content.push_str("   - View PR history on GitHub\n");
            content.push_str("   - Check recent commit emails\n");
            content.push_str("3. **Recreate the branch**:\n");
            content.push_str("   - Use `create_branch` tool with the SHA\n");
            content.push_str("   - Or: `git push origin <SHA>:refs/heads/branch-name`\n");
            content.push_str("4. **Prevention**:\n");
            content.push_str("   - Enable branch protection for important branches\n");
            content.push_str("   - Require reviews before deletion\n\n");
            content.push_str("**Note:** Local branch deletion uses git commands:\n");
            content.push_str("```bash\n");
            content.push_str("git branch -d branch-name    # Safe delete (merged only)\n");
            content.push_str("git branch -D branch-name    # Force delete (any state)\n");
            content.push_str("```\n\n");
        }

        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(content),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "scenario".to_string(),
                title: Some("Scenario".to_string()),
                description: Some(
                    "Focus on a specific scenario: 'cleanup' (after PR merge), 'recovery' (accidental deletion), \
                     'protection' (safety measures), 'workflow' (batch cleanup), or 'all' (comprehensive). Default: 'all'"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "include_permissions".to_string(),
                title: Some("Include Permissions".to_string()),
                description: Some(
                    "Include detailed permissions and access requirements section (true/false). Default: true"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "include_recovery".to_string(),
                title: Some("Include Recovery".to_string()),
                description: Some(
                    "Include branch recovery and restoration guidance (true/false). Default: true"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }
}
