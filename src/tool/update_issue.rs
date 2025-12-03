//! GitHub issue update tool

use anyhow;
use kodegen_mcp_schema::github::{
    UpdateIssueArgs, UpdateIssuePromptArgs, GitHubUpdateIssueOutput, GITHUB_UPDATE_ISSUE,
};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::github::UpdateIssueRequest;

/// Tool for updating GitHub issues
#[derive(Clone)]
pub struct UpdateIssueTool;

impl Tool for UpdateIssueTool {
    type Args = UpdateIssueArgs;
    type PromptArgs = UpdateIssuePromptArgs;

    fn name() -> &'static str {
        GITHUB_UPDATE_ISSUE
    }

    fn description() -> &'static str {
        "Update an existing GitHub issue. Supports partial updates - only specified fields \
         will be modified. Can update title, body, state (open/closed), labels, and assignees. \
         Requires GITHUB_TOKEN environment variable with write access."
    }

    fn read_only() -> bool {
        false // Modifies data
    }

    fn destructive() -> bool {
        false // Modifies, doesn't delete
    }

    fn idempotent() -> bool {
        false // Multiple updates may differ
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

        // Convert state string to IssueState enum
        let state = args
            .state
            .as_ref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "open" => Some(octocrab::models::IssueState::Open),
                "closed" => Some(octocrab::models::IssueState::Closed),
                _ => None,
            });

        // Build request
        let request = UpdateIssueRequest {
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            issue_number: args.issue_number,
            title: args.title.clone(),
            body: args.body.clone(),
            state,
            labels: args.labels.clone(),
            assignees: args.assignees.clone(),
            milestone: None,
        };

        // Call API wrapper (returns AsyncTask<Result<Issue, GitHubError>>)
        // The .await returns Result<Result<Issue, GitHubError>, RecvError>
        let task_result = client.update_issue(request).await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let issue =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build message based on what was updated
        let state_str = match issue.state {
            octocrab::models::IssueState::Open => "open",
            octocrab::models::IssueState::Closed => "closed",
            _ => "unknown",
        };

        let message = format!("Issue #{} updated successfully (state: {})", issue.number, state_str);

        let output = GitHubUpdateIssueOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            issue_number: args.issue_number,
            message,
        };

        // Build display string
        let mut updates = Vec::new();
        if args.title.is_some() {
            updates.push("title");
        }
        if args.body.is_some() {
            updates.push("body");
        }
        if args.state.is_some() {
            updates.push(format!("state ({})", state_str).leak() as &str);
        }
        if args.labels.is_some() {
            updates.push("labels");
        }
        if args.assignees.is_some() {
            updates.push("assignees");
        }

        let updates_str = if updates.is_empty() {
            "no changes".to_string()
        } else {
            updates.join(", ")
        };

        let display = format!(
            "Successfully updated issue #{} in {}/{}\nUpdated: {}\nCurrent state: {}",
            args.issue_number, args.owner, args.repo, updates_str, state_str
        );

        Ok(ToolResponse::new(display, output))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "scope".to_string(),
                title: Some("Update Scope".to_string()),
                description: Some(
                    "Which update types to focus on: 'state' (open/closed), 'labels', 'assignees', \
                     'title_body', or 'all' (default). Narrows teaching examples to specific fields."
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "detail_level".to_string(),
                title: Some("Teaching Detail Level".to_string()),
                description: Some(
                    "How detailed the examples should be: 'basic' for simple one-example-per-type \
                     or 'advanced' for multiple scenarios and edge cases. Defaults to 'basic'."
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "include_warnings".to_string(),
                title: Some("Include Important Notes".to_string()),
                description: Some(
                    "Whether to include critical warnings about tool behavior (default: true). \
                     Emphasizes that labels and assignees are replacement operations, not additive."
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        let scope = args.scope.as_deref().unwrap_or("all").to_lowercase();
        let detail_level = args.detail_level.as_deref().unwrap_or("basic").to_lowercase();
        let include_warnings = args.include_warnings.unwrap_or(true);

        // Build examples based on scope
        let mut examples = String::new();

        if scope == "all" || scope == "state" {
            examples.push_str(
                "Close an issue:\n\
                update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"state\": \"closed\"})\n\n"
            );
            if detail_level == "advanced" {
                examples.push_str(
                    "Reopen a closed issue:\n\
                    update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"state\": \"open\"})\n\n"
                );
            }
        }

        if scope == "all" || scope == "title_body" {
            examples.push_str(
                "Update title and body:\n\
                update_issue({\n\
                  \"owner\": \"octocat\",\n\
                  \"repo\": \"hello-world\",\n\
                  \"issue_number\": 42,\n\
                  \"title\": \"Updated: Bug in login\",\n\
                  \"body\": \"Revised description...\"\n\
                })\n\n"
            );
        }

        if scope == "all" || scope == "labels" {
            examples.push_str(
                "Replace labels:\n\
                update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"labels\": [\"bug\", \"resolved\"]})\n\n"
            );
        }

        if scope == "all" || scope == "assignees" {
            examples.push_str(
                "Update assignees:\n\
                update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"assignees\": [\"alice\", \"bob\"]})\n\n"
            );
        }

        // Build notes based on include_warnings parameter
        let notes = if include_warnings {
            "Returns GitHubUpdateIssueOutput with:\n\
            - success: boolean\n\
            - owner, repo: repository info\n\
            - issue_number: the updated issue number\n\
            - message: status message\n\n\
            Important notes:\n\
            - All fields are optional - only specified fields are updated\n\
            - state: \"open\" or \"closed\"\n\
            - labels: REPLACES all existing labels (not additive)\n\
            - assignees: REPLACES all existing assignees (not additive)\n\
            - To clear labels or assignees, pass empty array: []\n\
            - Requires write access to the repository\n\
            - GITHUB_TOKEN environment variable must be set"
        } else {
            "Returns GitHubUpdateIssueOutput. All fields are optional - only specified fields are updated."
        };

        let assistant_response = format!(
            "Use the update_issue tool to modify an existing GitHub issue:\n\n\
             {examples}\n\
             {notes}"
        );

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I update a GitHub issue?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(assistant_response),
            },
        ])
    }
}
