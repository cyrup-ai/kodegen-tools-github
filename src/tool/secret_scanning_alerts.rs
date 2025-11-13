//! GitHub secret scanning alerts tool

use anyhow;
use kodegen_mcp_schema::github::{SecretScanningAlertsArgs, SecretScanningAlertsPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use serde_json::Value;

/// Tool for listing secret scanning alerts in a GitHub repository
#[derive(Clone)]
pub struct SecretScanningAlertsTool;

impl Tool for SecretScanningAlertsTool {
    type Args = SecretScanningAlertsArgs;
    type PromptArgs = SecretScanningAlertsPromptArgs;
    
    fn name() -> &'static str {
        "github_secret_scanning_alerts"
    }
    
    fn description() -> &'static str {
        "List secret scanning alerts (leaked credentials) for a GitHub repository. \
         Returns alerts about exposed secrets like API keys, tokens, passwords, and \
         private keys. Supports filtering by state, secret type, and resolution. \
         Requires GitHub Advanced Security or public repository. Requires GITHUB_TOKEN."
    }
    
    fn read_only() -> bool {
        true  // Only reads data
    }
    
    fn destructive() -> bool {
        false  // No destructive operations
    }
    
    fn idempotent() -> bool {
        true  // Same request returns same result
    }
    
    fn open_world() -> bool {
        true  // Calls external GitHub API
    }
    
    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| McpError::Other(anyhow::anyhow!(
                "GITHUB_TOKEN environment variable not set"
            )))?;
        
        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {}", e)))?;
        
        // Call API wrapper (returns AsyncTask<Result<Vec<SecretScanningAlert>, GitHubError>>)
        let task_result = client.list_secret_scanning_alerts(
            args.owner.clone(),
            args.repo.clone(),
            args.state.clone(),
            args.secret_type.clone(),
            args.resolution.clone(),
        ).await;
        
        // Handle outer Result (channel error)
        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;
        
        // Handle inner Result (GitHub API error)
        let alerts = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;

        // Build human-readable summary
        let filters_applied = vec![
            args.state.as_ref().map(|s| format!("state: {}", s)),
            args.secret_type.as_ref().map(|t| format!("type: {}", t)),
            args.resolution.as_ref().map(|r| format!("resolution: {}", r)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");

        let filters_text = if !filters_applied.is_empty() {
            format!("\nFilters: {}", filters_applied)
        } else {
            String::new()
        };

        let alert_preview = alerts
            .iter()
            .take(5)
            .filter_map(|alert| {
                let number = alert.get("number")?.as_u64()?;
                let state = alert.get("state")?.as_str()?;
                let secret_type = alert.get("secret_type_display_name")?.as_str()?;
                
                let state_emoji = if state == "open" { "🔓" } else { "🔒" };
                
                Some(format!("  {} #{} [{}] {}", state_emoji, number, state, secret_type))
            })
            .collect::<Vec<_>>()
            .join("\n");

        let more_indicator = if alerts.len() > 5 {
            format!("\n  ... and {} more alerts", alerts.len() - 5)
        } else {
            String::new()
        };

        let warning_text = if alerts.iter().any(|a| a.get("state").and_then(|s| s.as_str()) == Some("open")) {
            "\n\n⚠️  WARNING: Open secrets found! Revoke them immediately and use environment variables or secret management."
        } else {
            ""
        };

        let summary = format!(
            "🔐 Retrieved {} secret scanning alert(s)\n\n\
             Repository: {}/{}{}\n\n\
             Recent alerts:\n{}{}{}",
            alerts.len(),
            args.owner,
            args.repo,
            filters_text,
            alert_preview,
            more_indicator,
            warning_text
        );

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&alerts)
            .unwrap_or_else(|_| "[]".to_string());
        
        Ok(vec![
            Content::text(summary),
            Content::text(json_str),
        ])
    }
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
    
    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I check for leaked secrets in my repository?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use secret_scanning_alerts to find exposed secrets:\n\n\
                     # Get all open secret alerts\n\
                     secret_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\"\n\
                     })\n\n\
                     # Get resolved alerts\n\
                     secret_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"resolved\",\n\
                       \"resolution\": \"revoked\"\n\
                     })\n\n\
                     States: \"open\", \"resolved\"\n\
                     Resolutions: \"false_positive\", \"wont_fix\", \"revoked\", \"used_in_tests\"\n\n\
                     Each alert includes:\n\
                     - Alert number and state\n\
                     - Secret type (API key, token, password, etc.)\n\
                     - Location in code\n\
                     - Created/resolved timestamps\n\
                     - Resolution details (if resolved)\n\
                     - Push protection status\n\n\
                     Secrets detected:\n\
                     - API keys (AWS, Azure, Google, etc.)\n\
                     - Authentication tokens\n\
                     - Private keys\n\
                     - Database credentials\n\
                     - OAuth tokens\n\n\
                     Requires: GitHub Advanced Security or public repository.\n\n\
                     IMPORTANT: If secrets are found, REVOKE them immediately\n\
                     and update the code to use environment variables or\n\
                     secret management systems.\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'security_events' scope\n\
                     - Repository must have secret scanning enabled\n\
                     - User must have appropriate permissions"
                ),
            },
        ])
    }
}
