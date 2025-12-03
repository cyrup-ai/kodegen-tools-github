use anyhow;
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::github::{
    CreatePullRequestArgs, GitHubCreatePrOutput, GITHUB_CREATE_PULL_REQUEST,
};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
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
            title: args.title.clone(),
            body: args.body.clone(),
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

        let html_url = pr.html_url
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();

        let output = GitHubCreatePrOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: pr.number,
            html_url: html_url.clone(),
            message: format!("Pull request #{} created successfully", pr.number),
        };

        let display = format!(
            "Successfully created Pull Request #{} in {}/{}\n\
            Title: {}\n\
            Base: {} <- Head: {}\n\
            URL: {}\n\
            Status: {}",
            pr.number,
            args.owner,
            args.repo,
            args.title,
            args.base,
            args.head,
            html_url,
            if args.draft.unwrap_or(false) { "Draft" } else { "Ready for review" }
        );

        Ok(ToolResponse::new(display, output))
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                "# GitHub Pull Request Creation Examples\n\n\
                ## Basic Pull Request\n\
                To create a simple pull request:\n\n\
                ```json\n\
                {\n\
                  \"owner\": \"octocat\",\n\
                  \"repo\": \"hello-world\",\n\
                  \"title\": \"Add new feature\",\n\
                  \"body\": \"This PR adds a new feature that...\",\n\
                  \"head\": \"feature-branch\",\n\
                  \"base\": \"main\"\n\
                }\n\
                ```\n\n\
                ## Draft Pull Request\n\
                To create a draft pull request:\n\n\
                ```json\n\
                {\n\
                  \"owner\": \"octocat\",\n\
                  \"repo\": \"hello-world\",\n\
                  \"title\": \"WIP: Experimental feature\",\n\
                  \"head\": \"experimental\",\n\
                  \"base\": \"develop\",\n\
                  \"draft\": true\n\
                }\n\
                ```\n\n\
                Returns GitHubCreatePrOutput with:\n\
                - success: boolean\n\
                - owner, repo: repository info\n\
                - pr_number: the created PR number\n\
                - html_url: link to the PR\n\
                - message: success message",
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "scenario_focus".to_string(),
            title: None,
            description: Some(
                "Which PR scenario to focus on: 'basic' (simple feature branches), 'draft' (work-in-progress), 'cross-fork' (upstream contributions), or 'all' for comprehensive examples".to_string(),
            ),
            required: Some(false),
        }]
    }
}
