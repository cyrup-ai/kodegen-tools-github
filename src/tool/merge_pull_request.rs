use anyhow;
use kodegen_mcp_schema::github::{
    MergePullRequestArgs, GitHubMergePrOutput, GITHUB_MERGE_PULL_REQUEST,
};
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext, ToolResponse};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

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

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                "# GitHub Pull Request Merge Examples\n\n\
                ## Basic Merge\n\
                To merge a pull request with default settings:\n\n\
                ```json\n\
                {\n\
                  \"owner\": \"octocat\",\n\
                  \"repo\": \"hello-world\",\n\
                  \"pr_number\": 42\n\
                }\n\
                ```\n\n\
                ## Squash Merge\n\
                To merge all commits into a single commit:\n\n\
                ```json\n\
                {\n\
                  \"owner\": \"octocat\",\n\
                  \"repo\": \"hello-world\",\n\
                  \"pr_number\": 42,\n\
                  \"merge_method\": \"squash\",\n\
                  \"commit_title\": \"Add authentication feature\"\n\
                }\n\
                ```\n\n\
                ## Rebase Merge\n\
                To rebase commits onto the base branch:\n\n\
                ```json\n\
                {\n\
                  \"owner\": \"octocat\",\n\
                  \"repo\": \"hello-world\",\n\
                  \"pr_number\": 42,\n\
                  \"merge_method\": \"rebase\"\n\
                }\n\
                ```\n\n\
                Returns GitHubMergePrOutput with:\n\
                - success: boolean\n\
                - owner, repo: repository info\n\
                - pr_number: the merged PR number\n\
                - merged: boolean indicating merge success\n\
                - sha: the merge commit SHA (if available)\n\
                - message: status message\n\n\
                Merge Methods:\n\
                - merge (default): Creates a merge commit\n\
                - squash: Combines all commits into one\n\
                - rebase: Rebases commits onto base branch\n\n\
                Safety Notes:\n\
                - This is a destructive operation\n\
                - Cannot be easily undone\n\
                - Use SHA parameter to prevent race conditions",
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "merge_strategy".to_string(),
                title: None,
                description: Some(
                    "Specific merge strategy to focus examples on: 'merge', 'squash', or 'rebase'".to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "Focus area for teaching: 'basic', 'advanced', 'safety', or 'all'".to_string(),
                ),
                required: Some(false),
            },
        ]
    }
}
