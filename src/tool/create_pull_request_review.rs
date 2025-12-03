use anyhow;
use kodegen_mcp_schema::github::{CreatePullRequestReviewArgs, CreatePullRequestReviewPromptArgs, GITHUB_CREATE_PULL_REQUEST_REVIEW};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use octocrab::models::pulls::ReviewAction;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for creating a review on a pull request
#[derive(Clone)]
pub struct CreatePullRequestReviewTool;

impl Tool for CreatePullRequestReviewTool {
    type Args = CreatePullRequestReviewArgs;
    type PromptArgs = CreatePullRequestReviewPromptArgs;

    fn name() -> &'static str {
        GITHUB_CREATE_PULL_REQUEST_REVIEW
    }

    fn description() -> &'static str {
        "Create a review on a pull request (approve, request changes, or comment). \
         Requires GITHUB_TOKEN environment variable with repo permissions."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        false // Multiple reviews can be submitted
    }

    fn open_world() -> bool {
        true // Calls external GitHub API
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) 
        -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Convert string event to ReviewAction enum
        let event = match args.event.to_uppercase().as_str() {
            "APPROVE" => ReviewAction::Approve,
            "REQUEST_CHANGES" => ReviewAction::RequestChanges,
            "COMMENT" => ReviewAction::Comment,
            _ => {
                return Err(McpError::InvalidArguments(format!(
                    "Invalid event '{}'. Must be APPROVE, REQUEST_CHANGES, or COMMENT",
                    args.event
                )));
            }
        };

        // Build options struct
        let options = crate::CreatePullRequestReviewOptions {
            event,
            body: args.body.clone(),
            commit_id: args.commit_id.clone(),
            comments: None, // Inline comments not supported in this tool
        };

        // Call API wrapper (returns AsyncTask<Result<Review, GitHubError>>)
        let task_result = client
            .create_pull_request_review(args.owner.clone(), args.repo.clone(), args.pull_number, options)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let review =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubCreatePrReviewOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pull_number,
            review_id: review.id.0, // Convert octocrab::models::ReviewId to u64
            event: args.event.to_uppercase(),
            message: format!("Created {} review on PR #{}", args.event.to_uppercase(), args.pull_number),
        };

        // Build human-readable display
        let display = format!(
            "✅ PR Review Created\n\n\
             Repository: {}/{}\n\
             PR: #{}\n\
             Review ID: {}\n\
             Event: {}\n\
             Body: {}",
            output.owner,
            output.repo,
            output.pr_number,
            output.review_id,
            output.event,
            args.body.as_deref().unwrap_or("(no comment)")
        );

        Ok(ToolResponse::new(display, output))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "focus_area".to_string(),
                title: Some("Review Type Focus".to_string()),
                description: Some(
                    "Optional focus area for teaching: 'approve' (approval workflows), \
                    'request_changes' (feedback & blocking), 'comment' (discussions only), \
                    or 'general' (all aspects). Customizes examples to relevant workflows."
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "skill_level".to_string(),
                title: Some("Explanation Depth".to_string()),
                description: Some(
                    "Optional skill level for explanation: 'beginner' (basic workflows, common cases), \
                    'intermediate' (standard workflows, best practices), or 'advanced' \
                    (edge cases, automation patterns, integration scenarios). Defaults to 'intermediate'."
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        // Extract and normalize arguments
        let focus_area = args
            .focus_area
            .as_deref()
            .unwrap_or("general")
            .to_lowercase();
        let skill_level = args
            .skill_level
            .as_deref()
            .unwrap_or("intermediate")
            .to_lowercase();

        // Build user question based on focus area
        let user_question = match focus_area.as_str() {
            "approve" => "How do I approve a pull request?",
            "request_changes" => "How do I request changes on a pull request?",
            "comment" => "How do I leave review comments without blocking?",
            _ => "How do I review a pull request?",
        };

        // Build assistant response with focus-specific content
        let base_response = match focus_area.as_str() {
            "approve" => {
                format!(
                    "Use create_pull_request_review with event \"APPROVE\" to approve a PR:\n\n\
                     create_pull_request_review({{\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"APPROVE\",\n\
                       \"body\": \"LGTM! Great work.\"\n\
                     }})\n\n\
                     The APPROVE event indicates the PR is ready to merge if required reviews are met.\n\
                     {}",
                    match skill_level.as_str() {
                        "beginner" => {
                            "Basic workflow:\n\
                             1. Review the code changes\n\
                             2. Run tests locally if available\n\
                             3. Approve with a positive comment\n\
                             4. PR can now be merged if all conditions are met"
                                .to_string()
                        }
                        "advanced" => {
                            "Advanced patterns:\n\
                             - Always include a substantive body explaining what was reviewed\n\
                             - Use commit_id to review specific commits in multi-commit PRs\n\
                             - Consider stale review status if new commits are pushed\n\
                             - Combine with add_pull_request_review_comment for inline feedback\n\
                             - Automate approvals for trusted authors (requires careful auth checks)"
                                .to_string()
                        }
                        _ => {
                            "Key points:\n\
                             - Always include a descriptive body with your approval\n\
                             - Specify commit_id for complex multi-commit reviews\n\
                             - Your approval may be required depending on branch protection rules"
                                .to_string()
                        }
                    }
                )
            }
            "request_changes" => {
                format!(
                    "Use create_pull_request_review with event \"REQUEST_CHANGES\" to block a PR:\n\n\
                     create_pull_request_review({{\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"REQUEST_CHANGES\",\n\
                       \"body\": \"Please address the comments before merging.\"\n\
                     }})\n\n\
                     The REQUEST_CHANGES event blocks the PR from merging until changes are made.\n\
                     {}",
                    match skill_level.as_str() {
                        "beginner" => {
                            "Basic workflow:\n\
                             1. Identify issues or improvements needed\n\
                             2. Request changes with clear explanation\n\
                             3. Provide actionable feedback in the body\n\
                             4. Wait for author to address comments and request re-review"
                                .to_string()
                        }
                        "advanced" => {
                            "Advanced patterns:\n\
                             - Use REQUEST_CHANGES sparingly - prefer COMMENT for suggestions\n\
                             - REQUEST_CHANGES should be for blocking issues (security, breaking API, etc.)\n\
                             - Combine with add_pull_request_review_comment for specific line feedback\n\
                             - Consider timing of request relative to PR lifecycle\n\
                             - Monitor for re-review requests after author makes changes"
                                .to_string()
                        }
                        _ => {
                            "Key points:\n\
                             - Use REQUEST_CHANGES for blocking issues only\n\
                             - Provide clear, actionable feedback in the body\n\
                             - Use inline comments for specific code locations\n\
                             - Wait for author to push changes before re-approving"
                                .to_string()
                        }
                    }
                )
            }
            "comment" => {
                format!(
                    "Use create_pull_request_review with event \"COMMENT\" to provide feedback without blocking:\n\n\
                     create_pull_request_review({{\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"COMMENT\",\n\
                       \"body\": \"Some suggestions for improvement.\"\n\
                     }})\n\n\
                     The COMMENT event leaves a review without approval or blocking the PR.\n\
                     {}",
                    match skill_level.as_str() {
                        "beginner" => {
                            "Basic workflow:\n\
                             1. Provide constructive feedback\n\
                             2. Leave suggestions without blocking\n\
                             3. Use for questions or non-critical improvements\n\
                             4. Author can merge regardless of your comments"
                                .to_string()
                        }
                        "advanced" => {
                            "Advanced patterns:\n\
                             - Use COMMENT for design discussions and questions\n\
                             - Combine with inline comments via add_pull_request_review_comment\n\
                             - COMMENT doesn't require author response before merge\n\
                             - Useful for knowledge sharing and mentoring feedback\n\
                             - Consider thread discussions with multiple comments"
                                .to_string()
                        }
                        _ => {
                            "Key points:\n\
                             - COMMENT is for suggestions and non-blocking feedback\n\
                             - Does not prevent PR from merging\n\
                             - Use for questions, design discussion, or optional improvements\n\
                             - Combine with inline comments for specific code locations"
                                .to_string()
                        }
                    }
                )
            }
            _ => {
                // General comprehensive response covering all types
                format!(
                    "create_pull_request_review creates a review on a pull request. The event type determines the impact:\n\n\
                     Event types:\n\
                     - \"APPROVE\" - Approve the PR (allows merging if required reviews are met)\n\
                     - \"REQUEST_CHANGES\" - Block PR until changes are made\n\
                     - \"COMMENT\" - Leave review comments without approval/blocking\n\
                     \n\
                     Basic usage:\n\
                     create_pull_request_review({{\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"APPROVE\",\n\
                       \"body\": \"Looks good!\"\n\
                     }})\n\
                     \n\
                     Optional fields:\n\
                     - body: Overall review comment (recommended for context)\n\
                     - commit_id: Specific commit SHA to review (defaults to latest)\n\
                     \n\
                     {}",
                    match skill_level.as_str() {
                        "beginner" => {
                            "Examples by use case:\n\n\
                             # Approve a simple PR\n\
                             create_pull_request_review({{\n\
                               \"owner\": \"octocat\",\n\
                               \"repo\": \"hello-world\",\n\
                               \"pull_number\": 42,\n\
                               \"event\": \"APPROVE\",\n\
                               \"body\": \"Tests pass, code looks good!\"\n\
                             }})\n\n\
                             # Request changes for issues\n\
                             create_pull_request_review({{\n\
                               \"owner\": \"octocat\",\n\
                               \"repo\": \"hello-world\",\n\
                               \"pull_number\": 42,\n\
                               \"event\": \"REQUEST_CHANGES\",\n\
                               \"body\": \"Please fix the failing tests before merging.\"\n\
                             }})\n\n\
                             # Leave suggestions without blocking\n\
                             create_pull_request_review({{\n\
                               \"owner\": \"octocat\",\n\
                               \"repo\": \"hello-world\",\n\
                               \"pull_number\": 42,\n\
                               \"event\": \"COMMENT\",\n\
                               \"body\": \"Consider renaming this variable for clarity.\"\n\
                             }})\n\n\
                             Requirements:\n\
                             - GITHUB_TOKEN environment variable must be set\n\
                             - Token needs 'repo' scope\n\
                             - User must have write access to the repository"
                                .to_string()
                        }
                        "advanced" => {
                            "Advanced usage:\n\n\
                             # Review specific commit in multi-commit PR\n\
                             create_pull_request_review({{\n\
                               \"owner\": \"octocat\",\n\
                               \"repo\": \"hello-world\",\n\
                               \"pull_number\": 42,\n\
                               \"event\": \"APPROVE\",\n\
                               \"body\": \"Reviewed the refactoring commit\",\n\
                               \"commit_id\": \"abc123def456...\"\n\
                             }})\n\n\
                             Workflow patterns:\n\
                             - Submit COMMENT review first for discussion\n\
                             - Then submit APPROVE or REQUEST_CHANGES after discussion\n\
                             - Use inline comments via add_pull_request_review_comment for code location feedback\n\
                             - Monitor stale review status if new commits are pushed\n\
                             - Automated approval workflows (with auth validation)\n\
                             - Status check integration with branch protection rules\n\n\
                             Note: This creates a REVIEW, not individual line comments.\n\
                             Use add_pull_request_review_comment for inline code comments."
                                .to_string()
                        }
                        _ => {
                            "Common examples:\n\n\
                             # Approve with comment\n\
                             create_pull_request_review({{\n\
                               \"owner\": \"octocat\",\n\
                               \"repo\": \"hello-world\",\n\
                               \"pull_number\": 42,\n\
                               \"event\": \"APPROVE\",\n\
                               \"body\": \"Looks good! All tests pass.\"\n\
                             }})\n\n\
                             # Request changes\n\
                             create_pull_request_review({{\n\
                               \"owner\": \"octocat\",\n\
                               \"repo\": \"hello-world\",\n\
                               \"pull_number\": 42,\n\
                               \"event\": \"REQUEST_CHANGES\",\n\
                               \"body\": \"Please address the comments before merging.\"\n\
                             }})\n\n\
                             # Comment only (no approval/block)\n\
                             create_pull_request_review({{\n\
                               \"owner\": \"octocat\",\n\
                               \"repo\": \"hello-world\",\n\
                               \"pull_number\": 42,\n\
                               \"event\": \"COMMENT\",\n\
                               \"body\": \"Some suggestions for improvement.\"\n\
                             }})\n\n\
                             Requirements:\n\
                             - GITHUB_TOKEN environment variable must be set with 'repo' scope\n\
                             - User must have write access to the repository"
                                .to_string()
                        }
                    }
                )
            }
        };

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(user_question),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(base_response),
            },
        ])
    }
}
