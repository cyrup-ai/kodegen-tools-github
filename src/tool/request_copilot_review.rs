use anyhow;
use kodegen_mcp_schema::github::{RequestCopilotReviewArgs, RequestCopilotReviewPromptArgs, GITHUB_REQUEST_COPILOT_REVIEW};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for requesting GitHub Copilot to review a pull request
#[derive(Clone)]
pub struct RequestCopilotReviewTool;

impl Tool for RequestCopilotReviewTool {
    type Args = RequestCopilotReviewArgs;
    type PromptArgs = RequestCopilotReviewPromptArgs;

    fn name() -> &'static str {
        GITHUB_REQUEST_COPILOT_REVIEW
    }

    fn description() -> &'static str {
        "Request GitHub Copilot to review a pull request (experimental feature). \
         Triggers automated code review from Copilot. Requires GITHUB_TOKEN and Copilot access."
    }

    fn read_only() -> bool {
        false // Triggers an action
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        true // Can be called multiple times safely
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

        // Call API wrapper (returns AsyncTask<Result<(), GitHubError>>)
        let task_result = client
            .request_copilot_review(args.owner.clone(), args.repo.clone(), args.pull_number)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build typed output
        let output = kodegen_mcp_schema::github::GitHubRequestCopilotReviewOutput {
            success: true,
            owner: args.owner.clone(),
            repo: args.repo.clone(),
            pr_number: args.pull_number,
            message: format!("Copilot review requested for PR #{}", args.pull_number),
        };

        // Build human-readable display
        let display = format!(
            "🤖 Copilot Review Requested\n\n\
             Repository: {}/{}\n\
             PR: #{}\n\
             Status: Pending",
            output.owner,
            output.repo,
            output.pr_number
        );

        Ok(ToolResponse::new(display, output))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "Optional focus area for the review (e.g., 'security', 'performance', 'style', 'best_practices')"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "depth".to_string(),
                title: None,
                description: Some(
                    "Optional depth of explanation (e.g., 'basic' for quick overview, 'detailed' for comprehensive guide)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        // Determine focus area from arguments (defaults to "general")
        let focus_area = args.focus_area.as_deref().unwrap_or("general").to_lowercase();
        
        // Determine depth level from arguments (defaults to "basic")
        let depth = args.depth.as_deref().unwrap_or("basic").to_lowercase();
        
        // Build a tailored teaching conversation based on focus_area and depth
        let teaching_content = match (focus_area.as_str(), depth.as_str()) {
            // Security-focused, basic depth
            ("security", "basic") => {
                "Use request_copilot_review to trigger Copilot's security analysis:\n\n\
                 request_copilot_review({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"pull_number\": 42})\n\n\
                 Security Focus:\n\
                 Copilot will analyze your PR for common security vulnerabilities including:\n\
                 - Authentication and authorization flaws\n\
                 - Injection vulnerabilities (SQL, command, etc.)\n\
                 - Insecure data handling\n\
                 - Missing input validation\n\
                 - Exposure of secrets or sensitive information\n\n\
                 Requirements:\n\
                 - GITHUB_TOKEN environment variable set\n\
                 - Token needs 'repo' scope for private repos\n\
                 - Repository must have Copilot access enabled\n\n\
                 Tip: Check PR comments after a few moments to see Copilot's security recommendations."
            }
            
            // Security-focused, detailed depth
            ("security", "detailed") => {
                "Using request_copilot_review for Comprehensive Security Analysis:\n\n\
                 request_copilot_review({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"pull_number\": 42})\n\n\
                 Comprehensive Security Review Scope:\n\
                 - Injection attacks (SQL, OS command, template, etc.)\n\
                 - Broken authentication and session management\n\
                 - Sensitive data exposure and leakage\n\
                 - XML external entity (XXE) processing\n\
                 - Broken access control (authorization flaws)\n\
                 - Security misconfiguration\n\
                 - Cross-site scripting (XSS) vulnerabilities\n\
                 - Insecure deserialization\n\
                 - Using components with known vulnerabilities\n\
                 - Insufficient logging and monitoring\n\
                 - Cryptographic failures\n\
                 - Server-side request forgery (SSRF)\n\n\
                 How Copilot Analyzes:\n\
                 1. Examines all changed code in the PR\n\
                 2. Identifies patterns matching known vulnerability types\n\
                 3. Checks for missing security best practices\n\
                 4. Provides specific remediation suggestions\n\n\
                 Important Limitations:\n\
                 - This is EXPERIMENTAL - API may change\n\
                 - Analysis depends on code context and clarity\n\
                 - Copilot may miss some edge cases\n\
                 - Manual security review should still be performed\n\n\
                 Workflow:\n\
                 1. Create/update PR with code changes\n\
                 2. Request review: request_copilot_review({...})\n\
                 3. Wait 2-5 seconds for Copilot to analyze\n\
                 4. Fetch PR reviews: get_pull_request_reviews({...})\n\
                 5. Review all security suggestions from Copilot\n\
                 6. Address findings and update code\n\
                 7. Request another review if needed\n\n\
                 Best Practices:\n\
                 - Use before merging PRs that touch authentication or data handling\n\
                 - Combine with static analysis tools for comprehensive coverage\n\
                 - Treat Copilot's feedback as a first pass, not authoritative\n\
                 - Keep GITHUB_TOKEN safe and rotate regularly"
            }
            
            // Performance-focused, basic depth
            ("performance", "basic") => {
                "Using request_copilot_review to Identify Performance Issues:\n\n\
                 request_copilot_review({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"pull_number\": 42})\n\n\
                 Performance Aspects Analyzed:\n\
                 - Inefficient algorithms and loops\n\
                 - Memory leaks and unnecessary allocations\n\
                 - N+1 query problems in database access\n\
                 - Blocking operations in async code\n\
                 - Missing caching opportunities\n\
                 - Unoptimized resource usage\n\n\
                 Usage:\n\
                 request_copilot_review({...}) triggers the review\n\
                 Check PR comments for performance suggestions\n\n\
                 Quick tip: Combine with profiling tools for deeper analysis."
            }
            
            // Performance-focused, detailed depth
            ("performance", "detailed") => {
                "Using request_copilot_review for Deep Performance Analysis:\n\n\
                 request_copilot_review({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"pull_number\": 42})\n\n\
                 Performance Dimensions Analyzed:\n\n\
                 1. Algorithm Complexity\n\
                 - Identifies inefficient algorithms (e.g., nested loops where linear is possible)\n\
                 - Detects O(n²) or worse patterns in code\n\
                 - Suggests more efficient data structures\n\n\
                 2. Memory Usage\n\
                 - Flags unnecessary allocations\n\
                 - Detects memory leaks\n\
                 - Identifies large object copies\n\
                 - Suggests optimal memory patterns\n\n\
                 3. Concurrency & Async\n\
                 - Finds blocking operations in async code\n\
                 - Detects lock contention issues\n\
                 - Identifies missed parallelization opportunities\n\n\
                 4. I/O & Network\n\
                 - N+1 query patterns in database access\n\
                 - Inefficient API calls and batching\n\
                 - Missing caching layers\n\
                 - Suboptimal connection pooling\n\n\
                 5. Resource Optimization\n\
                 - File handle leaks\n\
                 - Connection pool exhaustion risks\n\
                 - String concatenation in loops\n\
                 - Unnecessary data transformations\n\n\
                 Workflow for Performance Reviews:\n\
                 1. Create PR with performance-critical changes\n\
                 2. Request review: request_copilot_review({...})\n\
                 3. Wait for Copilot analysis (2-5 seconds)\n\
                 4. Read all performance suggestions\n\
                 5. Profile the code to validate suggestions\n\
                 6. Implement optimizations\n\
                 7. Measure improvement with benchmarks\n\n\
                 Combining with Other Tools:\n\
                 - Use with profilers (perf, flame graphs, etc.)\n\
                 - Check benchmarks before/after changes\n\
                 - Compare with previous PR performance metrics\n\n\
                 Note: Copilot provides static analysis insights. Use profiling tools \
                 to verify actual runtime performance impact."
            }
            
            // Style/best practices-focused, basic depth
            ("style", "basic") => {
                "Using request_copilot_review for Code Style and Best Practices:\n\n\
                 request_copilot_review({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"pull_number\": 42})\n\n\
                 Style Checks Include:\n\
                 - Naming consistency (variables, functions, classes)\n\
                 - Code formatting and whitespace\n\
                 - Comment clarity and completeness\n\
                 - Function/class organization\n\
                 - Error handling patterns\n\
                 - Language idioms and best practices\n\n\
                 Simple workflow:\n\
                 1. Push your PR code\n\
                 2. Call request_copilot_review({...})\n\
                 3. Review Copilot's style suggestions\n\
                 4. Adjust code style as recommended"
            }
            
            // Style/best practices-focused, detailed depth
            ("style", "detailed") => {
                "Using request_copilot_review for Comprehensive Code Style and Best Practices:\n\n\
                 request_copilot_review({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"pull_number\": 42})\n\n\
                 Detailed Style Analysis Covers:\n\n\
                 1. Naming Conventions\n\
                 - Variable names are clear and descriptive\n\
                 - Function names express intent\n\
                 - Constant names follow conventions\n\
                 - Class/interface names are appropriate\n\
                 - Avoid single-letter names (except loop counters)\n\n\
                 2. Code Organization\n\
                 - Functions have single responsibility\n\
                 - Classes are appropriately sized\n\
                 - Related code is grouped together\n\
                 - Module structure is logical\n\
                 - Dependencies flow correctly\n\n\
                 3. Error Handling\n\
                 - Exceptions are caught appropriately\n\
                 - Error messages are descriptive\n\
                 - Error handling follows language conventions\n\
                 - Resources are cleaned up (RAII patterns)\n\
                 - Error paths don't silently fail\n\n\
                 4. Documentation & Comments\n\
                 - Complex logic is explained\n\
                 - Public APIs are documented\n\
                 - Edge cases are noted\n\
                 - Comments explain 'why', not 'what'\n\
                 - Outdated comments are removed\n\n\
                 5. Language Idioms & Patterns\n\
                 - Using language features correctly\n\
                 - Following framework conventions\n\
                 - Avoiding anti-patterns\n\
                 - Using appropriate design patterns\n\
                 - Language-specific best practices\n\n\
                 6. Type Safety & Generics\n\
                 - Types are appropriately specific\n\
                 - Generics are used effectively\n\
                 - Type hints are present where helpful\n\
                 - Null/None handling is explicit\n\n\
                 7. Testing & Maintainability\n\
                 - Code is testable\n\
                 - Dependencies are injected\n\
                 - Complex behavior is isolated\n\
                 - Magic numbers are extracted\n\n\
                 Implementation Workflow:\n\
                 1. Push PR with changes\n\
                 2. Request review: request_copilot_review({...})\n\
                 3. Let Copilot analyze (2-5 seconds)\n\
                 4. Fetch and review all suggestions: get_pull_request_reviews({...})\n\
                 5. Categorize feedback (must-fix vs. nice-to-have)\n\
                 6. Refactor code based on suggestions\n\
                 7. Add comments explaining non-obvious logic\n\
                 8. Request follow-up review if major changes made\n\n\
                 Advanced Pattern: Code Quality Gates\n\
                 Use request_copilot_review in CI/CD to:\n\
                 - Enforce style consistency automatically\n\
                 - Catch anti-patterns before merge\n\
                 - Ensure all code follows team standards\n\
                 - Archive review feedback for learning"
            }
            
            // General/best_practices focus (covers multiple areas)
            ("general" | "best_practices", _) => {
                "How to Use request_copilot_review for Code Review:\n\n\
                 request_copilot_review({\n\
                   \"owner\": \"octocat\",\n\
                   \"repo\": \"hello-world\",\n\
                   \"pull_number\": 42\n\
                 })\n\n\
                 This is an EXPERIMENTAL feature that:\n\
                 - Requests GitHub Copilot to analyze your PR\n\
                 - Copilot reviews code changes and provides suggestions\n\
                 - Results appear as PR comments/reviews\n\n\
                 What Copilot Reviews:\n\
                 - Code quality and best practices\n\
                 - Potential bugs and issues\n\
                 - Security vulnerabilities\n\
                 - Performance improvements\n\
                 - Code style and conventions\n\n\
                 Requirements:\n\
                 - GitHub Copilot access on the repository\n\
                 - GITHUB_TOKEN environment variable must be set\n\
                 - Token needs 'repo' scope for private repos\n\
                 - User must have appropriate permissions\n\
                 - May not be available on all repository types\n\n\
                 Important Notes:\n\
                 - This endpoint is EXPERIMENTAL and may change\n\
                 - The request triggers the review but doesn't return the review content\n\
                 - Check PR comments after a short delay to see Copilot's feedback\n\
                 - Review availability depends on repository settings\n\
                 - Not all repositories have Copilot review enabled\n\n\
                 Example Workflow:\n\n\
                 1. Create or update a pull request\n\
                 2. Request Copilot review: request_copilot_review({...})\n\
                 3. Wait a few moments for Copilot to analyze\n\
                 4. Check PR comments: get_pull_request_reviews({...})\n\
                 5. Review Copilot's suggestions and feedback\n\n\
                 Use Cases:\n\
                 - Automated code review for initial feedback\n\
                 - Catch common issues before human review\n\
                 - Get suggestions for improvements\n\
                 - Security and quality checks\n\
                 - Learn from AI-generated best practices\n\n\
                 Tip: Combine with get_pull_request_reviews to see all reviews \
                 including Copilot's automated feedback."
            }
            
            // Default fallback for any other combination
            _ => {
                // Same as general
                "How to Use request_copilot_review for Code Review:\n\n\
                 request_copilot_review({\n\
                   \"owner\": \"octocat\",\n\
                   \"repo\": \"hello-world\",\n\
                   \"pull_number\": 42\n\
                 })\n\n\
                 This is an EXPERIMENTAL feature that:\n\
                 - Requests GitHub Copilot to analyze your PR\n\
                 - Copilot reviews code changes and provides suggestions\n\
                 - Results appear as PR comments/reviews\n\n\
                 What Copilot Reviews:\n\
                 - Code quality and best practices\n\
                 - Potential bugs and issues\n\
                 - Security vulnerabilities\n\
                 - Performance improvements\n\
                 - Code style and conventions\n\n\
                 Requirements:\n\
                 - GitHub Copilot access on the repository\n\
                 - GITHUB_TOKEN environment variable must be set\n\
                 - Token needs 'repo' scope for private repos\n\
                 - User must have appropriate permissions\n\
                 - May not be available on all repository types\n\n\
                 Important Notes:\n\
                 - This endpoint is EXPERIMENTAL and may change\n\
                 - The request triggers the review but doesn't return the review content\n\
                 - Check PR comments after a short delay to see Copilot's feedback\n\
                 - Review availability depends on repository settings\n\
                 - Not all repositories have Copilot review enabled\n\n\
                 Example Workflow:\n\n\
                 1. Create or update a pull request\n\
                 2. Request Copilot review: request_copilot_review({...})\n\
                 3. Wait a few moments for Copilot to analyze\n\
                 4. Check PR comments: get_pull_request_reviews({...})\n\
                 5. Review Copilot's suggestions and feedback\n\n\
                 Use Cases:\n\
                 - Automated code review for initial feedback\n\
                 - Catch common issues before human review\n\
                 - Get suggestions for improvements\n\
                 - Security and quality checks\n\
                 - Learn from AI-generated best practices\n\n\
                 Tip: Combine with get_pull_request_reviews to see all reviews \
                 including Copilot's automated feedback."
            }
        };
        
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I request a GitHub Copilot review?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(teaching_content),
            },
        ])
    }
}
