//! GitHub issue retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{GetIssueArgs, GetIssuePromptArgs, GITHUB_GET_ISSUE};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for fetching a GitHub issue by number
#[derive(Clone)]
pub struct GetIssueTool;

impl Tool for GetIssueTool {
    type Args = GetIssueArgs;
    type PromptArgs = GetIssuePromptArgs;

    fn name() -> &'static str {
        GITHUB_GET_ISSUE
    }

    fn description() -> &'static str {
        "Fetch a single GitHub issue by number. Returns detailed issue information including \
         title, body, state, labels, assignees, comments count, and timestamps. \
         Requires GITHUB_TOKEN environment variable."
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

        // Call API wrapper (returns AsyncTask<Result<Issue, GitHubError>>)
        // The .await returns Result<Result<Issue, GitHubError>, RecvError>
        let task_result = client
            .get_issue(args.owner, args.repo, args.issue_number)
            .await;

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
            "\x1b[36m Issue #{}: {}\x1b[0m\n\
              Status: {} · Comments: {} · Author: @{}",
            issue.number,
            issue.title,
            state_str,
            issue.comments,
            issue.user.login
        );
        contents.push(Content::text(summary));

        // Content[1]: Machine-Parseable JSON
        let json_str = match serde_json::to_string_pretty(&issue) {
            Ok(json) => json,
            Err(e) => return Err(McpError::Other(anyhow::anyhow!("Failed to serialize issue to JSON: {}", e))),
        };
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "detail_focus".to_string(),
                title: None,
                description: Some(
                    "Focus teaching on: 'basic' (minimal usage), 'advanced' (complex patterns, response interpretation), or 'pr' (pull request specific usage)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            // Exchange 1: Basic Usage
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I fetch a specific GitHub issue?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the get_issue tool to fetch a GitHub issue by its number:\n\n\
                     BASIC USAGE:\n\
                     get_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42})\n\n\
                     REQUIRED PARAMETERS:\n\
                     - owner: Repository owner (user or organization name)\n\
                     - repo: Repository name\n\
                     - issue_number: The issue NUMBER (e.g., 42 from #42), NOT the internal ID\n\n\
                     RESPONSE FORMAT (Dual Output):\n\
                     1. Content[0]: Human-readable summary with ANSI colors\n\
                        - Displays issue number, title, state (open/closed), comment count, author\n\
                        - Example: \"[36m Issue #42: Add authentication[0m\"\n\n\
                     2. Content[1]: Complete JSON object\n\
                        - Full issue details: title, body, state, labels, assignees, timestamps\n\
                        - Use this for programmatic processing\n\n\
                     BASIC RESPONSE FIELDS:\n\
                     - number: Issue number\n\
                     - title: Issue title\n\
                     - body: Issue description (Markdown)\n\
                     - state: \"open\" or \"closed\"\n\
                     - user: {login, avatar_url, ...} - Issue creator\n\
                     - labels: [{name, color, ...}] - Applied labels\n\
                     - assignees: [{login, ...}] - Assigned users\n\
                     - comments: Number of comments\n\
                     - created_at, updated_at: ISO timestamps\n\
                     - html_url: Link to issue on GitHub.com"
                ),
            },
            // Exchange 2: Important Distinction - Issues vs PRs
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "Can I use get_issue to fetch pull requests?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Yes! On GitHub, pull requests ARE treated as issues internally, so get_issue works for both.\n\n\
                     TO FETCH A PULL REQUEST:\n\
                     - Use the same syntax: get_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 123})\n\
                     - It doesn't matter if #123 is a PR or issue - the endpoint returns both\n\n\
                     DETECTING IF IT'S A PULL REQUEST:\n\
                     In the returned JSON, check the \"pull_request\" field:\n\
                     - If pull_request exists: It's a PR (has url, html_url, diff_url, etc.)\n\
                     - If pull_request is null/missing: It's a regular issue\n\n\
                     WHY THIS MATTERS:\n\
                     - Use get_issue to check if a number refers to a PR before working with it\n\
                     - For PR-specific operations (reviews, merge), use dedicated tools\n\
                     - This tool returns the same data structure for both"
                ),
            },
            // Exchange 3: Common Pitfalls
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What are common mistakes when using get_issue?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "PITFALL 1: Using Internal ID Instead of Issue Number\n\
                     ❌ WRONG: get_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 987654321})\n\
                     ✓ RIGHT: get_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42})\n\
                     The issue_number is the visible #42 in GitHub URLs, NOT the internal database ID.\n\n\
                     PITFALL 2: Forgetting GITHUB_TOKEN Environment Variable\n\
                     Error: \"GITHUB_TOKEN environment variable not set\"\n\
                     Solution: Set token before calling tool\n\
                     - For public repos: Any token works, even read-only scopes\n\
                     - For private repos: Token needs 'repo' scope\n\
                     - Public repo access: 'public_repo' scope sufficient\n\n\
                     PITFALL 3: Wrong Repository Ownership\n\
                     If you get 404 \"Not Found\":\n\
                     - Verify owner is correct (not the username that created repo, but the org that owns it)\n\
                     - Check spelling of repo name (case-sensitive)\n\
                     - Confirm token has access to this repo (private repos need explicit access)\n\n\
                     PITFALL 4: Misinterpreting State Values\n\
                     - state is always lowercase: \"open\" or \"closed\"\n\
                     - \"draft\" status for PRs is NOT in state field - check draft boolean in pull_request object\n\
                     - state is never \"in progress\" - GitHub only uses open/closed\n\n\
                     PITFALL 5: Ignoring Rate Limits\n\
                     - Authenticated requests: 5,000 per hour\n\
                     - Each get_issue call counts as 1 request\n\
                     - Check response headers for X-RateLimit-Remaining\n\
                     - 403 response = rate limit exceeded"
                ),
            },
            // Exchange 4: Integration with Related Tools
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How does get_issue integrate with other GitHub tools?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "WORKFLOW PATTERNS:\n\n\
                     PATTERN 1: Read Issue Then Comment\n\
                     1. get_issue({owner, repo, issue_number}) → fetch issue details\n\
                     2. Analyze issue.body and issue.labels\n\
                     3. add_issue_comment({owner, repo, issue_number, body: \"...\"}) → respond\n\n\
                     PATTERN 2: Get Comments for Context\n\
                     1. get_issue({owner, repo, issue_number}) → get basic info\n\
                     2. get_issue_comments({owner, repo, issue_number}) → fetch all discussion\n\
                     3. Combine issue.body + comments to understand full context\n\n\
                     PATTERN 3: Update Based on Current State\n\
                     1. get_issue({owner, repo, issue_number}) → check current state\n\
                     2. Verify state and fields before calling update_issue()\n\
                     3. update_issue({owner, repo, issue_number, state: \"closed\", ...}) → make changes\n\n\
                     PATTERN 4: Check PR Status Before Merging\n\
                     1. get_issue({owner, repo, issue_number}) → fetch\n\
                     2. Check pull_request field to confirm it's a PR\n\
                     3. Use get_pull_request_status() for merge details\n\
                     4. merge_pull_request() with confidence\n\n\
                     RELATED TOOLS:\n\
                     - get_issue_comments: Fetch discussion on an issue\n\
                     - add_issue_comment: Reply to an issue\n\
                     - update_issue: Modify issue (title, body, state, labels, assignees)\n\
                     - create_issue: Create new issue\n\
                     - get_issue_comments: Load discussion history\n\
                     - search_issues: Find issues by query (when you don't know the number)"
                ),
            },
            // Exchange 5: Response Interpretation
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How should I interpret the dual-output response from get_issue?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "GET_ISSUE RETURNS TWO CONTENT BLOCKS:\n\n\
                     CONTENT[0]: Human-Readable Summary (ANSI formatted)\n\
                     Example: \"[36m Issue #42: Add authentication[0m\n\
                     Status: open · Comments: 5 · Author: @octocat\"\n\
                     USE WHEN:\n\
                     - Displaying to humans\n\
                     - Quick visual check of issue state\n\
                     - Note: Colors are ANSI escape codes ([36m = cyan, [0m = reset)\n\n\
                     CONTENT[1]: Complete JSON Object\n\
                     Example: {\"number\": 42, \"title\": \"Add authentication\", ...all fields...}\n\
                     USE WHEN:\n\
                     - Extracting specific fields for logic (state, labels, assignees)\n\
                     - Passing data to other tools\n\
                     - Storing structured issue data\n\
                     - Processing labels or assignees arrays\n\n\
                     COMMON PARSING PATTERNS:\n\n\
                     Check if issue is open:\n\
                     - Parse JSON from Content[1]\n\
                     - if json.state == \"open\" { ... }\n\n\
                     Extract all assignees:\n\
                     - json.assignees is array of objects with \"login\" field\n\
                     - Extract logins: assignees.map(a => a.login)\n\n\
                     Get issue age in days:\n\
                     - Parse created_at timestamp\n\
                     - Calculate days_ago = now - created_at\n\n\
                     Check if edited since creation:\n\
                     - Compare updated_at vs created_at\n\
                     - if updated_at > created_at: issue was edited\n\n\
                     Extract action items:\n\
                     - Issue body is Markdown - may contain checkbox lists [ ] or [x]\n\
                     - Parse body for task markers\n\n\
                     Best Practice: Always use Content[1] (JSON) for programmatic logic, never parse Content[0]"
                ),
            },
            // Exchange 6: Error Handling
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What errors can get_issue return and how do I handle them?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "ERROR SCENARIOS:\n\n\
                     ERROR: \"GITHUB_TOKEN environment variable not set\"\n\
                     CAUSE: No authentication token configured\n\
                     SOLUTION: Set GITHUB_TOKEN before calling tool\n\n\
                     ERROR: 404 Not Found\n\
                     CAUSES:\n\
                     - Issue number doesn't exist in repository\n\
                     - Owner or repo name incorrect (typo?)\n\
                     - Accessing deleted issue\n\
                     - No access to private repository\n\
                     ACTION: Verify owner/repo/issue_number are correct\n\n\
                     ERROR: 401 Unauthorized\n\
                     CAUSE: Token is invalid, expired, or lacks required scope\n\
                     SOLUTION:\n\
                     - Check token is still valid\n\
                     - Verify token has 'repo' scope (for private repos)\n\
                     - For public repos, 'public_repo' scope works\n\n\
                     ERROR: 403 Forbidden\n\
                     CAUSES:\n\
                     - Rate limit exceeded (5000/hour)\n\
                     - User doesn't have access to private repo\n\
                     - Organization has restricted API access\n\
                     ACTION: Wait 1 hour for rate limit, or use different token\n\n\
                     ERROR: 422 Unprocessable Entity\n\
                     CAUSE: Parameters invalid (usually issue_number not a number)\n\
                     SOLUTION: Verify all parameters are correct types\n\n\
                     RATE LIMIT HANDLING:\n\
                     - GitHub allows 5,000 authenticated requests/hour\n\
                     - Check X-RateLimit-Remaining header\n\
                     - Implement exponential backoff if hitting limits\n\
                     - Single get_issue call = 1 request count\n\n\
                     BEST PRACTICE: Always check returned content is valid JSON before parsing"
                ),
            },
        ])
    }
}
