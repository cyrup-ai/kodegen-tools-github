//! GitHub code scanning alerts tool

use anyhow;
use kodegen_mcp_schema::github::{
    CodeScanningAlertsArgs, CodeScanningAlertsPromptArgs, GITHUB_CODE_SCANNING_ALERTS,
    GitHubCodeScanningAlertsOutput, GitHubCodeScanningAlert,
};
use kodegen_mcp_schema::ToolArgs;
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};

/// Tool for listing code scanning security alerts in a GitHub repository
#[derive(Clone)]
pub struct CodeScanningAlertsTool;

impl Tool for CodeScanningAlertsTool {
    type Args = CodeScanningAlertsArgs;
    type PromptArgs = CodeScanningAlertsPromptArgs;
    
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
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "Optional focus area for examples: 'basic' (filtering), 'dismissal' (dismissed alerts), \
                     'analysis_tools' (CodeQL/Semgrep), 'severity' (severity interpretation), \
                     or 'remediation' (fixing issues)"
                        .to_string(),
                ),
                required: Some(false),
            }
        ]
    }
    
    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            // BASIC USAGE EXAMPLE
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I get code scanning alerts for a repository?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use code_scanning_alerts to retrieve security alerts:\n\n\
                     # Get all open alerts\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\"\n\
                     })\n\n\
                     # Get critical severity alerts\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\",\n\
                       \"severity\": \"critical\"\n\
                     })\n\n\
                     # Get alerts for specific branch\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"ref_name\": \"main\",\n\
                       \"state\": \"open\"\n\
                     })\n\n\
                     States: \"open\", \"closed\", \"dismissed\"\n\
                     Severities: \"critical\", \"high\", \"medium\", \"low\", \"warning\", \"note\", \"error\"\n\n\
                     Each alert includes:\n\
                     - Alert number and state\n\
                     - Severity and description\n\
                     - Location (file, line)\n\
                     - Tool that found it (CodeQL, etc.)\n\
                     - Created/updated timestamps\n\
                     - Dismissal reason (if dismissed)\n\n\
                     Requires: GitHub Advanced Security enabled on the repository.\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'security_events' scope\n\
                     - Repository must have code scanning enabled\n\
                     - User must have appropriate permissions"
                ),
            },
            
            // ADVANCED SCENARIOS: DISMISSED ALERTS & ANALYSIS TOOLS
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I work with dismissed alerts and different analysis tools?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "You can filter dismissed alerts and target specific analysis tools:\n\n\
                     # Get all dismissed alerts (e.g., false positives)\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"dismissed\"\n\
                     })\n\n\
                     # Get alerts from specific tool only (CodeQL)\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\",\n\
                       \"tool_name\": \"CodeQL\"\n\
                     })\n\n\
                     # Combine filters: critical CodeQL alerts on main branch\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\",\n\
                       \"severity\": \"critical\",\n\
                       \"tool_name\": \"CodeQL\",\n\
                       \"ref_name\": \"main\"\n\
                     })\n\n\
                     Common analysis tools:\n\
                     - CodeQL: GitHub's semantic analysis engine\n\
                     - Semgrep: Pattern-based static analysis\n\
                     - Custom tools: Can integrate with other SAST tools\n\n\
                     Alert states explained:\n\
                     - \"open\": Active, needs attention\n\
                     - \"dismissed\": Marked as not applicable (false positive, intentional, etc.)\n\
                     - \"closed\": Fixed in the codebase\n\n\
                     Key gotchas:\n\
                     - Dismissed alerts may have reasons: false_positive, inaccurate, wont_fix, used_in_tests\n\
                     - Results limited to 30 per page; pagination required for large result sets\n\
                     - Alerts from main branch only by default; use ref_name to check other branches\n\
                     - Closed alerts appear when code is fixed, not when manually dismissed"
                ),
            },
        ])
    }
}
