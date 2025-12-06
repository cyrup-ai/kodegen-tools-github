use anyhow;
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{ListBranchesArgs, ListBranchesPrompts, GITHUB_LIST_BRANCHES};

use crate::GitHubClient;

/// Tool for listing repository branches
pub struct ListBranchesTool;

impl Tool for ListBranchesTool {
    type Args = ListBranchesArgs;
    type Prompts = ListBranchesPrompts;

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
}
