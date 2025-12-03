use anyhow;
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{ListBranchesArgs, GITHUB_LIST_BRANCHES};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;

/// Tool for listing repository branches
pub struct ListBranchesTool;

impl Tool for ListBranchesTool {
    type Args = ListBranchesArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        GITHUB_LIST_BRANCHES
    }

    fn description() -> &'static str {
        "List all branches in a repository"
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
            .list_branches(args.owner.clone(), args.repo.clone(), args.page, args.per_page)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let branches =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Convert octocrab branches to typed output
        let branch_list: Vec<kodegen_mcp_schema::github::GitHubBranch> = branches
            .iter()
            .map(|b| kodegen_mcp_schema::github::GitHubBranch {
                name: b.name.clone(),
                sha: b.commit.sha.clone(),
                protected: b.protected,
            })
            .collect();

        let count = branch_list.len();

        // Build human-readable display with emoji
        let branch_display = branch_list
            .iter()
            .map(|b| {
                let short_sha = &b.sha[..7];
                let protection = if b.protected { "🔒" } else { "  " };
                format!("  {} {} - {}", protection, b.name, short_sha)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let display = format!(
            "🌿 Branches: {}/{}\n\
             {} branches\n\n\
             {}",
            args.owner,
            args.repo,
            count,
            branch_display
        );

        let output = kodegen_mcp_schema::github::GitHubListBranchesOutput {
            success: true,
            owner: args.owner,
            repo: args.repo,
            count,
            branches: branch_list,
        };

        Ok(ToolResponse::new(display, output))
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub List Branches Examples

## List All Branches
To list all branches in a repository:

```json
{
  "owner": "octocat",
  "repo": "hello-world"
}
```

## List with Pagination
To list branches with pagination control:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "page": 1,
  "per_page": 50
}
```

## Response Information

Each branch object includes:
- **name**: Branch name
- **commit**: Object with SHA and URL of the latest commit
- **protected**: Whether the branch is protected

## Common Use Cases

1. **Branch Discovery**: See what branches exist in a repository
2. **Branch Management**: Identify old or stale branches for cleanup
3. **Development Workflow**: Check available feature branches
4. **Release Management**: Find release or hotfix branches
5. **Protected Branches**: Identify which branches have protection rules

## Best Practices

- Use pagination for repositories with many branches
- Check the default branch (usually "main" or "master")
- Look for branch naming patterns (feature/, hotfix/, release/)
- Identify protected branches to understand workflow constraints

## Pagination Notes

- Default per_page is 30 branches
- Maximum per_page is 100
- Use page parameter to navigate through results
- Check response headers for total count and next page

## Branch Naming Conventions

Common patterns you might find:
- **main** or **master**: Primary branch
- **develop**: Development integration branch
- **feature/**: Feature branches (e.g., feature/user-auth)
- **hotfix/**: Urgent fixes (e.g., hotfix/security-patch)
- **release/**: Release preparation (e.g., release/v1.0.0)
- **bugfix/**: Bug fixes (e.g., bugfix/login-error)
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "detail_level".to_string(),
                title: None,
                description: Some(
                    "Level of detail for examples: 'basic' (simple cases), 'advanced' (pagination, protected branches), or 'all' (comprehensive)".to_string()
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "focus".to_string(),
                title: None,
                description: Some(
                    "Specific aspect to focus on: 'pagination', 'protected_branches', 'naming_conventions', 'use_cases', or 'all'".to_string()
                ),
                required: Some(false),
            }
        ]
    }
}
