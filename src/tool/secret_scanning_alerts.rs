//! GitHub secret scanning alerts tool

use anyhow;
use kodegen_mcp_schema::github::{
    SecretScanningAlertsArgs, SecretScanningAlertsPrompts, GITHUB_SECRET_SCANNING_ALERTS,
    GitHubSecretScanningAlertsOutput, GitHubSecretScanningAlert,
};
use kodegen_mcp_schema::ToolArgs;
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for listing secret scanning alerts in a GitHub repository
#[derive(Clone)]
pub struct SecretScanningAlertsTool;

impl Tool for SecretScanningAlertsTool {
    type Args = SecretScanningAlertsArgs;
    type Prompts = SecretScanningAlertsPrompts;
    
    fn name() -> &'static str {
        GITHUB_SECRET_SCANNING_ALERTS
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
    
    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as ToolArgs>::Output>, McpError> {
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

        // Build typed alert objects
        let alerts: Vec<GitHubSecretScanningAlert> = alerts
            .iter()
            .filter_map(|alert| {
                let number = alert.get("number")?.as_u64()?;
                let state = alert.get("state")?.as_str()?.to_string();
                let secret_type = alert.get("secret_type_display_name")
                    .or_else(|| alert.get("secret_type"))
                    ?.as_str()?.to_string();
                let resolution = alert.get("resolution")
                    .and_then(|r| r.as_str())
                    .map(|s| s.to_string());
                let created_at = alert.get("created_at")?.as_str()?.to_string();
                let html_url = alert.get("html_url")?.as_str()?.to_string();
                
                Some(GitHubSecretScanningAlert {
                    number,
                    state,
                    secret_type,
                    resolution,
                    created_at,
                    html_url,
                })
            })
            .collect();

        let count = alerts.len();

        // Build filters text
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

        // Build alert preview
        let alert_preview = alerts
            .iter()
            .take(10)
            .map(|a| {
                let state_emoji = if a.state == "open" { "🔓" } else { "🔒" };
                format!("  {} #{} [{}] {} [{}]", 
                    state_emoji, 
                    a.number, 
                    a.state,
                    a.secret_type,
                    a.resolution.as_deref().unwrap_or("unresolved"))
            })
            .collect::<Vec<_>>()
            .join("\n");

        let more_indicator = if count > 10 {
            format!("\n  ... and {} more alerts", count - 10)
        } else {
            String::new()
        };

        let warning_text = if alerts.iter().any(|a| a.state == "open") {
            "\n\n⚠️  WARNING: Open secrets found! Revoke them immediately and use environment variables or secret management."
        } else {
            ""
        };

        // Build display string
        let display = format!(
            "🔐 Secret Scanning Alerts: {}/{}\n\
             {} alerts found{}\n\n\
             Recent alerts:\n{}{}{}",
            args.owner,
            args.repo,
            count,
            filters_text,
            alert_preview,
            more_indicator,
            warning_text
        );

        // Build typed output
        let output = GitHubSecretScanningAlertsOutput {
            success: true,
            owner: args.owner,
            repo: args.repo,
            count,
            alerts,
        };

        Ok(ToolResponse::new(display, output))
    }
}
