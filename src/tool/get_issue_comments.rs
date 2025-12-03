//! GitHub issue comments retrieval tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{
    GetIssueCommentsArgs, GetIssueCommentsPromptArgs, GitHubGetIssueCommentsOutput, GitHubComment,
    GITHUB_GET_ISSUE_COMMENTS,
};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for fetching all comments on a GitHub issue
#[derive(Clone)]
pub struct GetIssueCommentsTool;

impl Tool for GetIssueCommentsTool {
    type Args = GetIssueCommentsArgs;
    type PromptArgs = GetIssueCommentsPromptArgs;

    fn name() -> &'static str {
        GITHUB_GET_ISSUE_COMMENTS
    }

    fn description() -> &'static str {
        "Fetch all comments for a GitHub issue. Returns an array of comment objects \
         including author, body, timestamps, and metadata. Comments are returned in \
         chronological order. Requires GITHUB_TOKEN environment variable."
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
        true // Calls external GitHub API
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Call API wrapper (returns AsyncStream)
        let mut comment_stream =
            client.get_issue_comments(args.owner.clone(), args.repo.clone(), args.issue_number);

        // Collect stream results
        let mut comments = Vec::new();
        while let Some(result) = comment_stream.next().await {
            let comment =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            comments.push(comment);
        }

        // Convert to typed output
        let github_comments: Vec<GitHubComment> = comments
            .iter()
            .map(|c| GitHubComment {
                id: c.id.into_inner(),
                author: c.user.login.clone(),
                body: c.body.clone().unwrap_or_default(),
                created_at: c.created_at.to_rfc3339(),
                updated_at: c.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            })
            .collect();

        let output = GitHubGetIssueCommentsOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            issue_number: args.issue_number,
            count: github_comments.len(),
            comments: github_comments,
        };

        // Build user-friendly display string
        let display = format!(
            "Successfully retrieved {} comment{} for issue #{} in {}/{}",
            output.count,
            if output.count == 1 { "" } else { "s" },
            output.issue_number,
            args.owner,
            args.repo
        );

        Ok(ToolResponse::new(display, output))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "aspect".to_string(),
                title: Some("Learning Focus".to_string()),
                description: Some(
                    "Which aspect of the tool to focus on: 'basic_usage' (simple examples), \
                     'response_structure' (fields and data layout), 'filtering_and_sorting' (advanced queries), \
                     'author_analysis' (working with comment authors), or 'discussion_tracking' (conversation patterns)"
                        .to_string(),
                ),
                required: Some(true),
            },
            PromptArgument {
                name: "depth".to_string(),
                title: Some("Learning Depth".to_string()),
                description: Some(
                    "Complexity level of examples: 'beginner' (simple, step-by-step), \
                     'intermediate' (common patterns and best practices), \
                     or 'advanced' (optimization, edge cases, performance)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I fetch all comments for a GitHub issue?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the get_issue_comments tool to fetch all comments for an issue:\n\n\
                     Basic usage:\n\
                     get_issue_comments({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42})\n\n\
                     Returns GitHubGetIssueCommentsOutput with:\n\
                     - success: boolean\n\
                     - owner, repo: repository info\n\
                     - issue_number: the issue number\n\
                     - count: number of comments\n\
                     - comments: array of GitHubComment objects\n\n\
                     Each GitHubComment contains:\n\
                     - id: Comment ID\n\
                     - author: Comment author username\n\
                     - body: Comment text (Markdown)\n\
                     - created_at: When comment was created\n\
                     - updated_at: When comment was last edited\n\n\
                     Comment ordering:\n\
                     - Comments are returned in chronological order (oldest first)\n\
                     - Use the created_at timestamp to determine comment age\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos\n\
                     - Works for both issues and pull requests",
                ),
            },
        ])
    }
}
