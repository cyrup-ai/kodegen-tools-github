use anyhow;
use kodegen_mcp_schema::github::{
    MergePullRequestArgs, MergePullRequestPrompts, GitHubMergePrOutput, GITHUB_MERGE_PULL_REQUEST,
};
use kodegen_mcp_schema::{McpError, Tool, ToolExecutionContext, ToolResponse};

use crate::GitHubClient;

/// Tool for merging a pull request
pub struct MergePullRequestTool;

impl Tool for MergePullRequestTool {
    type Args = MergePullRequestArgs;
    type Prompts = MergePullRequestPrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let options = crate::MergePullRequestOptions {
            commit_title: args.commit_title.clone(),
            commit_message: args.commit_message.clone(),
            sha: args.sha.clone(),
            merge_method: args.merge_method.clone(),
        };

        let task_result = client
            .merge_pull_request(args.owner.clone(), args.repo.clone(), args.pr_number, options)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let merge_result =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Extract SHA from merge result
        let sha = merge_result.get("sha")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        let merged = merge_result.get("merged")
            .and_then(|m| m.as_bool())
            .unwrap_or(true);

        let merge_method = args.merge_method.as_deref().unwrap_or("merge");

        let output = GitHubMergePrOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pr_number,
            merged,
            sha: sha.clone(),
            message: format!("Pull request #{} merged successfully (method: {})", args.pr_number, merge_method),
        };

        let display = format!(
            "Successfully merged PR #{} in {}/{} using {} method{}",
            args.pr_number,
            args.owner,
            args.repo,
            merge_method,
            sha.as_ref().map(|s| format!("\nMerge commit: {}", s)).unwrap_or_default()
        );

        Ok(ToolResponse::new(display, output))
    }
}
