//! GitHub issue update tool

use anyhow;
use kodegen_mcp_schema::github::{UpdateIssueArgs, UpdateIssuePromptArgs, GITHUB_UPDATE_ISSUE};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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
            title: args.title,
            body: args.body,
            state,
            labels: args.labels,
            assignees: args.assignees,
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

        // Build dual-content response
        let mut contents = Vec::new();

        // Content[0]: Human-Readable Summary
        let state_str = match issue.state {
            octocrab::models::IssueState::Open => "open",
            octocrab::models::IssueState::Closed => "closed",
            _ => "unknown",
        };

        let summary = format!(
            "\x1b[33m Issue Updated: #{}\x1b[0m\n\
             󰋼 Repo: {}/{} · State: {}",
            issue.number,
            args.owner,
            args.repo,
            state_str
        );
        contents.push(Content::text(summary));

        // Content[1]: Machine-Parseable JSON
        let json_str = serde_json::to_string_pretty(&issue)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
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
            if detail_level == "advanced" {
                examples.push_str(
                    "Update only the body:\n\
                    update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"body\": \"Appended note: fixed in latest patch\"})\n\n"
                );
            }
        }

        if scope == "all" || scope == "labels" {
            examples.push_str(
                "Replace labels:\n\
                update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"labels\": [\"bug\", \"resolved\"]})\n\n"
            );
            if detail_level == "advanced" {
                examples.push_str(
                    "Clear all labels:\n\
                    update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"labels\": []})\n\n"
                );
            }
        }

        if scope == "all" || scope == "assignees" {
            examples.push_str(
                "Update assignees:\n\
                update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"assignees\": [\"alice\", \"bob\"]})\n\n"
            );
            if detail_level == "advanced" {
                examples.push_str(
                    "Unassign everyone:\n\
                    update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"assignees\": []})\n\n"
                );
            }
        }

        if scope == "all" {
            examples.push_str(
                "Combined update (multiple fields):\n\
                update_issue({\n\
                  \"owner\": \"octocat\",\n\
                  \"repo\": \"hello-world\",\n\
                  \"issue_number\": 42,\n\
                  \"state\": \"closed\",\n\
                  \"labels\": [\"bug\", \"fixed\"],\n\
                  \"body\": \"Fixed in PR #123\"\n\
                })\n\n"
            );
        }

        // Build notes based on include_warnings parameter
        let notes = if include_warnings {
            "Important notes:\n\
            - All fields are optional - only specified fields are updated\n\
            - state: \"open\" or \"closed\"\n\
            - labels: REPLACES all existing labels (not additive)\n\
            - assignees: REPLACES all existing assignees (not additive)\n\
            - To clear labels or assignees, pass empty array: []\n\
            - Requires write access to the repository\n\
            - GITHUB_TOKEN environment variable must be set"
        } else {
            "All fields are optional - only specified fields are updated"
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
