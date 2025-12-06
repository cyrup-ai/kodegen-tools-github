//! GitHub code scanning alerts tool

use anyhow;
use kodegen_mcp_schema::github::{
    CodeScanningAlertsArgs, GITHUB_CODE_SCANNING_ALERTS,
    GitHubCodeScanningAlertsOutput, GitHubCodeScanningAlert,
};
use kodegen_mcp_schema::ToolArgs;
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

/// Tool for listing code scanning security alerts in a GitHub repository
#[derive(Clone)]
pub struct CodeScanningAlertsTool;

impl Tool for CodeScanningAlertsTool {
    type Args = CodeScanningAlertsArgs;
    
    fn name() -> &'static str {
        GITHUB_CODE_SCANNING_ALERTS
    }
    
    fn description() -> &'static str {
        "List code scanning security alerts for a GitHub repository. Returns alerts \
         with details about vulnerabilities, their severity, location, and status. \
         Supports filtering by state, branch, tool, and severity. Requires GitHub \
         Advanced Security enabled. Requires GITHUB_TOKEN environment variable."
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
        
        // Call API wrapper (returns AsyncTask<Result<Vec<Value>, GitHubError>>)
        let task_result = client.list_code_scanning_alerts(
            args.owner.clone(),
            args.repo.clone(),
            args.state.clone(),
            args.ref_name.clone(),
            args.tool_name.clone(),
            args.severity.clone(),
        ).await;
        
        // Handle outer Result (channel error)
        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;
        
        // Handle inner Result (GitHub API error)
        let alerts = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;

        // Build typed alert objects
        let alerts: Vec<GitHubCodeScanningAlert> = alerts
            .iter()
            .filter_map(|alert| {
                let number = alert.get("number")?.as_u64()?;
                let state = alert.get("state")?.as_str()?.to_string();
                let rule = alert.get("rule")?;
                let severity = rule.get("severity")?.as_str()?.to_string();
                let rule_id = rule.get("id")?.as_str()?.to_string();
                let rule_description = rule.get("description")?.as_str()?.to_string();
                let tool_name = alert.get("tool")?.get("name")?.as_str()?.to_string();
                let created_at = alert.get("created_at")?.as_str()?.to_string();
                let html_url = alert.get("html_url")?.as_str()?.to_string();
                
                Some(GitHubCodeScanningAlert {
                    number,
                    state,
                    severity,
                    rule_id,
                    rule_description,
                    tool_name,
                    created_at,
                    html_url,
                })
            })
            .collect();

        let count = alerts.len();

        // Count by severity
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;
        
        for alert in &alerts {
            match alert.severity.as_str() {
                "critical" => critical += 1,
                "high" => high += 1,
                "medium" => medium += 1,
                "low" | "warning" | "note" | "error" => low += 1,
                _ => {}
            }
        }

        // Build filters text
        let filters_applied = vec![
            args.state.as_ref().map(|s| format!("state: {}", s)),
            args.ref_name.as_ref().map(|r| format!("branch: {}", r)),
            args.tool_name.as_ref().map(|t| format!("tool: {}", t)),
            args.severity.as_ref().map(|s| format!("severity: {}", s)),
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
                let sev_emoji = match a.severity.as_str() {
                    "critical" => "🔴",
                    "high" => "🟠",
                    "medium" => "🟡",
                    _ => "🔵",
                };
                format!("  {} #{} [{}] {} - {} ({})", 
                    sev_emoji, a.number, a.severity, a.rule_id, a.state, a.tool_name)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let more_indicator = if count > 10 {
            format!("\n  ... and {} more alerts", count - 10)
        } else {
            String::new()
        };

        // Build display string
        let display = format!(
            "🛡️  Code Scanning Alerts: {}/{}\n\
             {} alerts found{}\n\n\
             By severity:\n\
             🔴 Critical: {}\n\
             🟠 High: {}\n\
             🟡 Medium: {}\n\
             🔵 Low/Other: {}\n\n\
             Recent alerts:\n{}{}",
            args.owner,
            args.repo,
            count,
            filters_text,
            critical,
            high,
            medium,
            low,
            alert_preview,
            more_indicator
        );

        // Build typed output
        let output = GitHubCodeScanningAlertsOutput {
            success: true,
            owner: args.owner,
            repo: args.repo,
            count,
            alerts,
        };

        Ok(ToolResponse::new(display, output))
    }
}
