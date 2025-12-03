use anyhow;
use kodegen_mcp_schema::github::{
    GetPullRequestReviewsArgs, GetPullRequestReviewsPromptArgs, GitHubPrReviewsOutput, GitHubReview,
    GITHUB_GET_PULL_REQUEST_REVIEWS,
};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use octocrab::models::pulls::ReviewState;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use tokio_stream::StreamExt;

/// Tool for getting all reviews for a pull request
#[derive(Clone)]
pub struct GetPullRequestReviewsTool;

impl Tool for GetPullRequestReviewsTool {
    type Args = GetPullRequestReviewsArgs;
    type PromptArgs = GetPullRequestReviewsPromptArgs;

    fn name() -> &'static str {
        GITHUB_GET_PULL_REQUEST_REVIEWS
    }

    fn description() -> &'static str {
        "Get all reviews for a pull request. Shows approval status, requested changes, \
         and comments from reviewers. Requires GITHUB_TOKEN environment variable."
    }

    fn read_only() -> bool {
        true // Only reads data
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        true // Same result every time
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

        // Call API wrapper (returns AsyncStream<Result<Review, GitHubError>>)
        let mut review_stream =
            client.get_pull_request_reviews(args.owner.clone(), args.repo.clone(), args.pull_number);

        // Collect stream into vector
        let mut reviews = Vec::new();
        while let Some(result) = review_stream.next().await {
            let review =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            reviews.push(review);
        }

        // Convert to typed output
        let github_reviews: Vec<GitHubReview> = reviews
            .iter()
            .map(|r| {
                let state_str = match r.state {
                    Some(ReviewState::Approved) => "APPROVED",
                    Some(ReviewState::ChangesRequested) => "CHANGES_REQUESTED",
                    Some(ReviewState::Commented) => "COMMENTED",
                    Some(ReviewState::Dismissed) => "DISMISSED",
                    Some(ReviewState::Pending) => "PENDING",
                    Some(_) => "OTHER",
                    None => "UNKNOWN",
                };

                let author = r.user.as_ref()
                    .map(|u| u.login.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let submitted_at = r.submitted_at
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_default();

                GitHubReview {
                    id: r.id.into_inner(),
                    author,
                    state: state_str.to_string(),
                    body: r.body.clone(),
                    submitted_at,
                }
            })
            .collect();

        let output = GitHubPrReviewsOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pull_number,
            reviews: github_reviews,
        };

        // Build user-friendly display string
        let review_count = output.reviews.len();
        let display = format!(
            "Successfully retrieved {} review{} for PR #{} in {}/{}",
            review_count,
            if review_count == 1 { "" } else { "s" },
            args.pull_number,
            args.owner,
            args.repo
        );

        Ok(ToolResponse::new(display, output))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "Optional focus area for teaching prompt (e.g., 'approval_workflow', 'blocking_reviews', 'timeline', 'filtering')"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "use_case".to_string(),
                title: None,
                description: Some(
                    "Optional use case context for examples (e.g., 'merge_gates', 'permission_checks', 'ci_integration')"
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
                content: PromptMessageContent::text("How do I see all reviews on a pull request?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use get_pull_request_reviews to see all reviews:\n\n\
                     get_pull_request_reviews({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42\n\
                     })\n\n\
                     Returns GitHubPrReviewsOutput with:\n\
                     - success: boolean\n\
                     - owner, repo: repository info\n\
                     - pr_number: the PR number\n\
                     - reviews: array of GitHubReview objects\n\n\
                     Each GitHubReview contains:\n\
                     - id: Review ID\n\
                     - author: Reviewer username\n\
                     - state: \"APPROVED\", \"CHANGES_REQUESTED\", \"COMMENTED\", \"DISMISSED\", \"PENDING\"\n\
                     - body: Review comment text\n\
                     - submitted_at: When review was submitted\n\n\
                     Review states:\n\
                     - APPROVED: Reviewer approved the changes\n\
                     - CHANGES_REQUESTED: Reviewer wants changes before approval\n\
                     - COMMENTED: Reviewer left comments without approval/blocking\n\
                     - DISMISSED: Review was dismissed (no longer valid)\n\
                     - PENDING: Review is in progress but not submitted\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos",
                ),
            },
        ])
    }
}
