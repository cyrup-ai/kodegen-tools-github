use anyhow;
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext};
use kodegen_mcp_schema::github::{SearchRepositoriesArgs, SearchRepositoriesPromptArgs, GITHUB_SEARCH_REPOSITORIES};
use octocrab::Octocrab;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

/// Tool for searching GitHub repositories
pub struct SearchRepositoriesTool;

impl Tool for SearchRepositoriesTool {
    type Args = SearchRepositoriesArgs;
    type PromptArgs = SearchRepositoriesPromptArgs;

    fn name() -> &'static str {
        GITHUB_SEARCH_REPOSITORIES
    }

    fn description() -> &'static str {
        "Search GitHub repositories using GitHub's repository search syntax"
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
        true
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Create octocrab instance directly
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let mut request = octocrab.search().repositories(&args.query);

        if let Some(sort_val) = &args.sort {
            request = request.sort(sort_val);
        }

        if let Some(order_val) = &args.order {
            request = request.order(order_val);
        }

        if let Some(p) = args.page {
            request = request.page(p);
        }

        if let Some(pp) = args.per_page {
            request = request.per_page(pp);
        }

        let page = request
            .send()
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build human-readable summary
        let total_count = page.total_count.unwrap_or(0);
        let items = &page.items;

        let summary = if items.is_empty() {
            format!(
                "\x1b[36m Repository Search: {}\x1b[0m\n 󰋗 Results: {} · Top: N/A",
                args.query,
                total_count
            )
        } else {
            let first_repo = items[0].full_name.as_deref().unwrap_or("N/A");
            let first_stars = items[0].stargazers_count.unwrap_or(0);
            format!(
                "\x1b[36m Repository Search: {}\x1b[0m\n 󰋗 Results: {} · Top: {} ⭐ {}",
                args.query,
                total_count,
                first_repo,
                first_stars
            )
        };

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&page)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(vec![
            Content::text(summary),
            Content::text(json_str),
        ])
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        // Determine customization based on args
        let _is_brief = args.depth.as_deref() == Some("brief");
        let is_detailed = args.depth.as_deref() == Some("detailed") || args.depth.is_none();
        let language = args.language.as_deref();
        let use_case = args.use_case.as_deref();
        
        // Build customized content
        let mut content = String::from("# GitHub Repository Search Examples\n\n");
        
        // Basic examples section
        content.push_str("## Basic Repository Search\n");
        content.push_str("To search for repositories by name or description:\n\n");
        content.push_str("```json\n{\n  \"query\": \"machine learning\",\n  \"per_page\": 20\n}\n```\n\n");
        
        // Language-specific search
        if let Some(lang) = language {
            content.push_str(&format!("## Search for {} Repositories\n", lang.to_uppercase()));
            content.push_str(&format!("To find {} repositories:\n\n", lang));
            content.push_str(&format!("```json\n{{\n  \"query\": \"language:{} stars:>100\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\",\n  \"per_page\": 30\n}}\n```\n\n", lang));
        } else {
            content.push_str("## Search by Language\n");
            content.push_str("To find repositories in a specific programming language:\n\n");
            content.push_str("```json\n{\n  \"query\": \"language:rust\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\",\n  \"per_page\": 30\n}\n```\n\n");
        }
        
        // Add comprehensive syntax guide unless brief
        if is_detailed {
            content.push_str("## GitHub Repository Search Query Syntax\n\n");
            
            content.push_str("### Language Filter\n\n");
            content.push_str("**language:name** - Filter by programming language\n");
            content.push_str("```json\n{\n  \"query\": \"language:rust stars:>100\"\n}\n```\n\n");
            
            if language.is_none() {
                content.push_str("Popular languages: rust, javascript, python, go, typescript, java, c++, ruby, php, swift\n\n");
            }
            
            content.push_str("### Stars Filter\n\n");
            content.push_str("**stars:>n** - Repositories with more than n stars\n");
            content.push_str("**stars:<n** - Repositories with fewer than n stars\n");
            content.push_str("**stars:n..m** - Repositories with stars in range\n\n");
            content.push_str("```json\n{\n  \"query\": \"language:rust stars:>1000\",\n  \"sort\": \"stars\"\n}\n```\n\n");
            content.push_str("```json\n{\n  \"query\": \"stars:100..500 language:python\"\n}\n```\n\n");
            
            content.push_str("### Forks Filter\n\n");
            content.push_str("**forks:>n** - Repositories with more than n forks\n");
            content.push_str("**forks:<n** - Repositories with fewer than n forks\n");
            content.push_str("**forks:n..m** - Repositories with forks in range\n\n");
            content.push_str("```json\n{\n  \"query\": \"language:javascript forks:>100\"\n}\n```\n\n");
            
            content.push_str("### Date Filters\n\n");
            content.push_str("**created:>YYYY-MM-DD** - Created after date\n");
            content.push_str("**created:<YYYY-MM-DD** - Created before date\n");
            content.push_str("**pushed:>YYYY-MM-DD** - Updated after date\n");
            content.push_str("**pushed:<YYYY-MM-DD** - Updated before date\n\n");
            content.push_str("```json\n{\n  \"query\": \"language:rust created:>2024-01-01\",\n  \"sort\": \"stars\"\n}\n```\n\n");
            content.push_str("```json\n{\n  \"query\": \"pushed:>2024-06-01 stars:>100\"\n}\n```\n\n");
            
            content.push_str("### Topic Filter\n\n");
            content.push_str("**topic:name** - Repositories with specific topic\n");
            content.push_str("```json\n{\n  \"query\": \"topic:async language:rust\"\n}\n```\n\n");
            content.push_str("```json\n{\n  \"query\": \"topic:machine-learning topic:python\"\n}\n```\n\n");
            
            content.push_str("### User and Organization Filters\n\n");
            content.push_str("**user:username** - Repositories owned by user\n");
            content.push_str("**org:orgname** - Repositories owned by organization\n\n");
            content.push_str("```json\n{\n  \"query\": \"user:octocat language:ruby\"\n}\n```\n\n");
            content.push_str("```json\n{\n  \"query\": \"org:github topic:ai\",\n  \"sort\": \"updated\"\n}\n```\n\n");
            
            content.push_str("### Combining Multiple Filters\n\n");
            
            if let Some(lang) = language {
                content.push_str(&format!("Find popular {} libraries:\n", lang));
                content.push_str(&format!("```json\n{{\n  \"query\": \"language:{} stars:>100\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\",\n  \"per_page\": 20\n}}\n```\n\n", lang));
            } else {
                content.push_str("Find popular async Rust libraries:\n");
                content.push_str("```json\n{\n  \"query\": \"language:rust stars:>100 topic:async\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\",\n  \"per_page\": 20\n}\n```\n\n");
            }
            
            content.push_str("Find recently updated Python ML projects:\n");
            content.push_str("```json\n{\n  \"query\": \"language:python topic:machine-learning pushed:>2024-01-01\",\n  \"sort\": \"updated\",\n  \"order\": \"desc\"\n}\n```\n\n");
            
            content.push_str("Find active projects in an organization:\n");
            content.push_str("```json\n{\n  \"query\": \"org:github stars:>50 pushed:>2024-06-01\",\n  \"sort\": \"stars\"\n}\n```\n\n");
        }
        
        // Sort and Order options
        content.push_str("## Sort Options\n\n");
        content.push_str("**stars** - Sort by number of stars (most popular)\n");
        content.push_str("**forks** - Sort by number of forks (most forked)\n");
        content.push_str("**updated** - Sort by last update date (most recently updated)\n\n");
        content.push_str("```json\n{\n  \"query\": \"language:rust\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}\n```\n\n");
        
        content.push_str("## Order Options\n\n");
        content.push_str("**asc** - Ascending order (least to most)\n");
        content.push_str("**desc** - Descending order (most to least)\n\n");
        
        // Response info - only in detailed mode
        if is_detailed {
            content.push_str("## Response Information\n\n");
            content.push_str("The response includes:\n");
            content.push_str("- **total_count**: Total number of matching repositories\n");
            content.push_str("- **incomplete_results**: Whether the search timed out\n");
            content.push_str("- **items**: Array of repository objects\n\n");
            content.push_str("Each repository object contains:\n");
            content.push_str("- **id**: Unique repository ID\n");
            content.push_str("- **name**: Repository name\n");
            content.push_str("- **full_name**: Owner/repo format\n");
            content.push_str("- **description**: Repository description\n");
            content.push_str("- **html_url**: Web URL to the repository\n");
            content.push_str("- **stargazers_count**: Number of stars\n");
            content.push_str("- **forks_count**: Number of forks\n");
            content.push_str("- **language**: Primary programming language\n");
            content.push_str("- **topics**: Array of repository topics\n");
            content.push_str("- **created_at**: Creation date\n");
            content.push_str("- **updated_at**: Last update date\n");
            content.push_str("- **pushed_at**: Last push date\n\n");
            
            content.push_str("## Pagination\n\n");
            content.push_str("- Default per_page is 30 repositories\n");
            content.push_str("- Maximum per_page is 100\n");
            content.push_str("- Use page parameter to navigate through results\n");
            content.push_str("- Check total_count for total matches\n\n");
        }
        
        // Use case specific examples
        match use_case {
            Some("discovery") => {
                content.push_str("## Discovery Use Case Examples\n\n");
                if let Some(lang) = language {
                    content.push_str(&format!("### Find Popular {} Libraries\n", lang.to_uppercase()));
                    content.push_str(&format!("```json\n{{\n  \"query\": \"language:{} stars:>500\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}}\n```\n\n", lang));
                } else {
                    content.push_str("### Find Popular Rust Web Frameworks\n");
                    content.push_str("```json\n{\n  \"query\": \"language:rust topic:web stars:>500\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}\n```\n\n");
                }
                content.push_str("### Find Trending Projects\n");
                content.push_str("```json\n{\n  \"query\": \"created:>2024-01-01 stars:>100\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}\n```\n\n");
            }
            Some("contribution") => {
                content.push_str("## Contribution Use Case Examples\n\n");
                content.push_str("### Find Beginner-Friendly Projects\n");
                if let Some(lang) = language {
                    content.push_str(&format!("```json\n{{\n  \"query\": \"good-first-issue language:{} stars:50..500\",\n  \"sort\": \"updated\"\n}}\n```\n\n", lang));
                } else {
                    content.push_str("```json\n{\n  \"query\": \"good-first-issue language:javascript stars:50..500\",\n  \"sort\": \"updated\"\n}\n```\n\n");
                }
                content.push_str("### Find Active Projects\n");
                content.push_str("```json\n{\n  \"query\": \"pushed:>2024-01-01 stars:>100\",\n  \"sort\": \"updated\",\n  \"order\": \"desc\"\n}\n```\n\n");
            }
            Some("research") => {
                content.push_str("## Research Use Case Examples\n\n");
                if let Some(lang) = language {
                    content.push_str(&format!("### Find {} Implementation Examples\n", lang.to_uppercase()));
                    content.push_str(&format!("```json\n{{\n  \"query\": \"language:{} topic:example\",\n  \"sort\": \"stars\"\n}}\n```\n\n", lang));
                } else {
                    content.push_str("### Find Implementation Examples\n");
                    content.push_str("```json\n{\n  \"query\": \"topic:example stars:>50\",\n  \"sort\": \"stars\"\n}\n```\n\n");
                }
                content.push_str("### Find Specific Patterns\n");
                content.push_str("```json\n{\n  \"query\": \"topic:design-patterns stars:>100\",\n  \"sort\": \"stars\"\n}\n```\n\n");
            }
            Some("trending") => {
                content.push_str("## Trending Use Case Examples\n\n");
                content.push_str("### Recently Created Projects\n");
                if let Some(lang) = language {
                    content.push_str(&format!("```json\n{{\n  \"query\": \"language:{} created:>2024-01-01\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}}\n```\n\n", lang));
                } else {
                    content.push_str("```json\n{\n  \"query\": \"created:>2024-01-01 stars:>10\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}\n```\n\n");
                }
                content.push_str("### Recently Updated Active Projects\n");
                content.push_str("```json\n{\n  \"query\": \"pushed:>2024-06-01 stars:>100\",\n  \"sort\": \"updated\",\n  \"order\": \"desc\"\n}\n```\n\n");
            }
            Some("evaluation") => {
                content.push_str("## Evaluation Use Case Examples\n\n");
                content.push_str("### Compare Similar Projects\n");
                if let Some(lang) = language {
                    content.push_str(&format!("```json\n{{\n  \"query\": \"language:{} stars:>500\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}}\n```\n\n", lang));
                } else {
                    content.push_str("```json\n{\n  \"query\": \"topic:web-framework stars:>500\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}\n```\n\n");
                }
                content.push_str("### Analyze Project Activity\n");
                content.push_str("```json\n{\n  \"query\": \"pushed:>2024-01-01 forks:>50\",\n  \"sort\": \"forks\",\n  \"order\": \"desc\"\n}\n```\n\n");
            }
            _ => {
                // Default common use cases
                content.push_str("## Common Use Cases\n\n");
                content.push_str("1. **Discover Libraries**: Find popular libraries in your language\n");
                content.push_str("2. **Research Tools**: Find tools for specific tasks\n");
                content.push_str("3. **Find Examples**: Discover example projects and implementations\n");
                content.push_str("4. **Track Trends**: Monitor emerging projects and technologies\n");
                content.push_str("5. **Competitor Analysis**: Research similar projects\n");
                content.push_str("6. **Open Source Discovery**: Find projects to contribute to\n");
                content.push_str("7. **Technology Assessment**: Evaluate adoption of technologies\n\n");
                
                content.push_str("## Example Workflows\n\n");
                
                if let Some(lang) = language {
                    content.push_str(&format!("### Find Popular {} Web Frameworks\n", lang.to_uppercase()));
                    content.push_str(&format!("```json\n{{\n  \"query\": \"language:{} topic:web stars:>500\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}}\n```\n\n", lang));
                } else {
                    content.push_str("### Find Popular Rust Web Frameworks\n");
                    content.push_str("```json\n{\n  \"query\": \"language:rust topic:web stars:>500\",\n  \"sort\": \"stars\",\n  \"order\": \"desc\"\n}\n```\n\n");
                }
                
                content.push_str("### Find Active Python Projects\n");
                content.push_str("```json\n{\n  \"query\": \"language:python pushed:>2024-01-01 stars:>100\",\n  \"sort\": \"updated\",\n  \"order\": \"desc\"\n}\n```\n\n");
                
                content.push_str("### Find Beginner-Friendly Projects\n");
                content.push_str("```json\n{\n  \"query\": \"good-first-issue language:javascript stars:50..500\",\n  \"sort\": \"updated\"\n}\n```\n\n");
                
                content.push_str("### Find Organization Repositories\n");
                content.push_str("```json\n{\n  \"query\": \"org:rust-lang topic:compiler\",\n  \"sort\": \"stars\"\n}\n```\n\n");
            }
        }
        
        // Best practices - only in detailed mode
        if is_detailed {
            content.push_str("## Best Practices\n\n");
            content.push_str("- **Use specific filters**: Combine multiple filters to narrow results\n");
            content.push_str("- **Sort appropriately**: Use \"stars\" for popularity, \"updated\" for activity\n");
            content.push_str("- **Filter by language**: Dramatically improves result relevance\n");
            content.push_str("- **Use date filters**: Find actively maintained projects\n");
            content.push_str("- **Check topics**: Topics provide better categorization than text search\n");
            content.push_str("- **Consider forks**: High fork count indicates usefulness\n");
            content.push_str("- **Look at update dates**: Avoid abandoned projects\n");
            content.push_str("- **Use star ranges**: Find projects at your experience level\n\n");
            
            content.push_str("## Search Tips\n\n");
            content.push_str("- Search repository names: Just use the text without qualifiers\n");
            content.push_str("- Combine with text search: `machine learning language:python`\n");
            content.push_str("- Use topic: for precise categorization\n");
            content.push_str("- Filter archived repos: Add `archived:false` to exclude archived\n");
            content.push_str("- Find templates: Search for `template` or `boilerplate`\n");
            content.push_str("- Discover trending: Use `created:>YYYY-MM-DD` for new projects\n");
            content.push_str("- Find maintained projects: Use `pushed:>YYYY-MM-DD` for recent activity\n");
        }
        
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(&content),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "language".to_string(),
                title: None,
                description: Some(
                    "Optional: programming language to focus examples on (e.g., 'rust', 'python', 'javascript', 'go', 'typescript')"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "use_case".to_string(),
                title: None,
                description: Some(
                    "Optional: use case to tailor examples to (discovery, research, trending, contribution, evaluation)"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "depth".to_string(),
                title: None,
                description: Some(
                    "Optional: level of detail for examples (brief, detailed)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }
}
