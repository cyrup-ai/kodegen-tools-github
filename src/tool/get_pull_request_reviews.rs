use anyhow;
use kodegen_mcp_schema::github::{
    GetPullRequestReviewsArgs, GetPullRequestReviewsPrompts, GitHubPrReviewsOutput, GitHubReview,
    GITHUB_GET_PULL_REQUEST_REVIEWS,
};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use octocrab::models::pulls::ReviewState;
use tokio_stream::StreamExt;

/// Tool for getting all reviews for a pull request
#[derive(Clone)]
pub struct GetPullRequestReviewsTool;

impl Tool for GetPullRequestReviewsTool {
    type Args = GetPullRequestReviewsArgs;
    type Prompts = GetPullRequestReviewsPrompts;

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
}
