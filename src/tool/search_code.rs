use anyhow;
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext};
use kodegen_mcp_schema::github::{SearchCodeArgs, GITHUB_SEARCH_CODE};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;

/// Tool for searching code across GitHub
pub struct SearchCodeTool;

impl Tool for SearchCodeTool {
    type Args = SearchCodeArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        GITHUB_SEARCH_CODE
    }

    fn description() -> &'static str {
        "Search code across GitHub repositories using GitHub's code search syntax"
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

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let task_result = client
            .search_code(
                args.query.clone(),
                args.sort.clone(),
                args.order.clone(),
                args.page,
                args.per_page,
                args.enrich_stars,
            )
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let page =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build human-readable summary with ANSI colors and Nerd Font icons
        let total_count = page.total_count.unwrap_or(0);
        let items = &page.items;

        // Extract first result info or use "N/A"
        let first_result = if let Some(first) = items.first() {
            format!(
                "{}/{}",
                first.repository.full_name.as_deref().unwrap_or("N/A"),
                first.path.as_str()
            )
        } else {
            "N/A".to_string()
        };

        let summary = format!(
            "\x1b[36m Code Search: {}\x1b[0m\n\
              Results: {} · Top: {}",
            args.query,
            total_count,
            first_result
        );

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&page)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(vec![
            Content::text(summary),
            Content::text(json_str),
        ])
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Code Search Examples

## Basic Code Search
To search for code across all accessible repositories:

```json
{
  "query": "async fn",
  "per_page": 20
}
```

## Search in Specific Repository
To search within a specific repository:

```json
{
  "query": "repo:octocat/hello-world async fn",
  "per_page": 30
}
```

## GitHub Code Search Query Syntax

### Repository Qualifiers

**repo:owner/repo** - Search in specific repository
```json
{
  "query": "repo:octocat/hello-world authentication"
}
```

**user:username** - Search across user's repositories
```json
{
  "query": "user:octocat tokio"
}
```

**org:orgname** - Search across organization's repositories
```json
{
  "query": "org:github async"
}
```

### Language Qualifiers

**language:name** - Filter by programming language
```json
{
  "query": "language:rust async fn"
}
```

**language:javascript** - JavaScript files
**language:python** - Python files
**language:go** - Go files
**language:typescript** - TypeScript files

### Path and File Qualifiers

**path:directory/** - Search in specific directory
```json
{
  "query": "path:src/ authentication"
}
```

**extension:ext** - Filter by file extension
```json
{
  "query": "extension:rs async fn"
}
```

**filename:name** - Search in files with specific name
```json
{
  "query": "filename:main.rs"
}
```

### Combining Multiple Filters

Search for Rust async functions in src directory:
```json
{
  "query": "repo:octocat/hello-world language:rust path:src/ async fn"
}
```

Search for configuration files in specific repo:
```json
{
  "query": "repo:octocat/hello-world extension:json filename:config"
}
```

Find authentication code in JavaScript:
```json
{
  "query": "org:github language:javascript authentication path:src/"
}
```

## Sort and Order

**Sort option:** Only "indexed" is valid for code search
```json
{
  "query": "language:rust tokio",
  "sort": "indexed",
  "order": "desc"
}
```

**Order options:**
- **asc**: Ascending order (oldest indexed first)
- **desc**: Descending order (newest indexed first)

## Response Information

The response includes:
- **total_count**: Total number of matches found
- **incomplete_results**: Whether the search timed out
- **items**: Array of code search results

Each result item contains:
- **name**: File name
- **path**: Full file path
- **sha**: File content SHA
- **url**: API URL for the file
- **git_url**: Git API URL
- **html_url**: Web URL to view the file
- **repository**: Repository object containing the file
- **score**: Relevance score

## Pagination

- Default per_page is 30 results
- Maximum per_page is 100
- Use page parameter to navigate through results
- Check total_count for total number of matches

## Rate Limiting

**IMPORTANT:** Code search has strict rate limits:
- **30 requests per minute** for authenticated requests
- **10 requests per minute** for unauthenticated requests
- Plan your searches carefully
- Consider caching results
- Use specific filters to reduce result sets

## Common Use Cases

1. **Find Examples**: Search for code examples across open source projects
2. **Security Audit**: Find potential security vulnerabilities in codebases
3. **API Usage**: Discover how others use a particular API or library
4. **Pattern Discovery**: Find common patterns and best practices
5. **Dependency Check**: Locate usage of specific dependencies
6. **License Compliance**: Find files with specific license headers
7. **Migration Planning**: Identify code that needs updating

## Example Workflows

### Find Tokio Usage in Rust
```json
{
  "query": "language:rust tokio spawn",
  "per_page": 50
}
```

### Find TODO Comments in JavaScript
```json
{
  "query": "language:javascript TODO",
  "per_page": 100
}
```

### Find Configuration Files
```json
{
  "query": "filename:config.json",
  "per_page": 30
}
```

### Find API Keys (Security Audit)
```json
{
  "query": "org:myorg API_KEY",
  "per_page": 100
}
```

## Best Practices

- **Be Specific**: Use multiple qualifiers to narrow results
- **Use repo: when possible**: Searching specific repos is faster and more accurate
- **Respect Rate Limits**: Space out searches, cache results
- **Use language: filter**: Dramatically improves search relevance
- **Combine path: and extension:**: For precise file targeting
- **Check incomplete_results**: If true, search timed out and results may be partial
- **Use meaningful queries**: Generic terms return too many results
- **Paginate wisely**: Don't fetch all pages if you only need top results

## Tips for Better Results

- Use exact phrases in quotes: `"async fn main"`
- Exclude terms with minus: `language:rust -test`
- Search for function signatures: `fn process_data`
- Look for imports: `import { useState }`
- Find specific patterns: `TODO:` or `FIXME:`
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "focus_area".to_string(),
                title: None,
                description: Some(
                    "Optional focus area for the teaching prompt (e.g., 'syntax', 'qualifiers', 'rate_limits', 'examples', 'best_practices', 'use_cases')"
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "language".to_string(),
                title: None,
                description: Some(
                    "Optional programming language to focus examples on (e.g., 'rust', 'python', 'javascript', 'go', 'typescript')"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }
}
