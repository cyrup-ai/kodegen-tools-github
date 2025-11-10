//! `kodegen_github` - GitHub API operations via Octocrab
//!
//! This library provides an async-first GitHub service layer with comprehensive
//! GitHub API support using the octocrab crate. Each GitHub operation is
//! implemented in its own module with builder patterns for ergonomic usage.

// Module declarations
pub mod github;
pub mod runtime;

// Re-export runtime types
pub use runtime::{AsyncStream, AsyncTask, EmitterBuilder};

// Re-export GitHub client types
pub use github::{GitHubClient, GitHubClientBuilder};

// Re-export GitHub error types
pub use github::{GitHubError, GitHubResult};

// Re-export GitHub operation options
pub use github::{
    CreatePullRequestReviewOptions, CreateReleaseOptions as GitHubReleaseOptions,
    ListCommitsOptions, MergePullRequestOptions, ReleaseResult as GitHubReleaseResult,
    UpdatePullRequestOptions, create_release, delete_release, get_release_by_tag,
    update_release,
};

// Re-export release asset upload types
pub use github::upload_release_asset::{UploadAssetOptions, upload_release_asset};

// Re-export GitHub types for public API
pub use github::{
    ActivityMetrics,
    CiCdMetrics,
    CodeQualityMetrics,
    DependencyMetrics,
    DocumentationMetrics,
    GithubSearch,
    LocalMetrics,
    MetadataInfo,
    Output as SearchOutput,
    QualityMetrics,
    ReadmeMetrics,
    RepositoryResult,
    SearchConfig,
    SearchError,
    // User search types
    SearchOrder,
    SearchProvider,
    SearchQuery,
    SearchSession,
    SecurityMetrics,
    StructureMetrics,
    TestMetrics,
    UserSearchSort,
    // Search functionality - both convenience functions and types
    search_repositories,
    search_repositories_with_config,
};

// MCP Tools (conditional compilation)
#[cfg(feature = "mcp")]
pub mod tool;

// Re-export MCP tools only (Args are imported from kodegen_mcp_schema::github)
#[cfg(feature = "mcp")]
pub use tool::{
    AddIssueCommentTool, AddPullRequestReviewCommentTool, CreateBranchTool, CreateIssueTool,
    CreatePullRequestReviewTool, CreatePullRequestTool, CreateRepositoryTool, ForkRepositoryTool,
    GetCommitTool, GetIssueCommentsTool, GetIssueTool, GetPullRequestFilesTool,
    GetPullRequestReviewsTool, GetPullRequestStatusTool, ListBranchesTool, ListCommitsTool,
    ListIssuesTool, MergePullRequestTool, RequestCopilotReviewTool, SearchCodeTool,
    SearchIssuesTool, SearchRepositoriesTool, SearchUsersTool, UpdateIssueTool,
    UpdatePullRequestTool,
};

/// Start the HTTP server programmatically for embedded mode
///
/// This is called by kodegend instead of spawning an external process.
/// Blocks until the server shuts down.
///
/// # Arguments
/// * `addr` - Socket address to bind to (e.g., "127.0.0.1:30451")
/// * `tls_cert` - Optional path to TLS certificate file
/// * `tls_key` - Optional path to TLS private key file
///
/// # Returns
/// ServerHandle for graceful shutdown, or error if startup fails
#[cfg(feature = "mcp")]
pub async fn start_server(
    addr: std::net::SocketAddr,
    tls_cert: Option<std::path::PathBuf>,
    tls_key: Option<std::path::PathBuf>,
) -> anyhow::Result<kodegen_server_http::ServerHandle> {
    use kodegen_server_http::{create_http_server, Managers, RouterSet, register_tool};
    use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
    use std::time::Duration;

    let tls_config = match (tls_cert, tls_key) {
        (Some(cert), Some(key)) => Some((cert, key)),
        _ => None,
    };

    let shutdown_timeout = Duration::from_secs(30);

    create_http_server("github", addr, tls_config, shutdown_timeout, |_config, _tracker| {
        Box::pin(async move {
            let mut tool_router = ToolRouter::new();
            let mut prompt_router = PromptRouter::new();
            let managers = Managers::new();

            // Register all GitHub tools (zero-state structs, no constructors)
            
            // Issue tools (7)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreateIssueTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetIssueTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, ListIssuesTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, UpdateIssueTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, SearchIssuesTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, AddIssueCommentTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetIssueCommentsTool);

            // Pull Request tools (9)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreatePullRequestTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, UpdatePullRequestTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, MergePullRequestTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetPullRequestStatusTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetPullRequestFilesTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetPullRequestReviewsTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreatePullRequestReviewTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, AddPullRequestReviewCommentTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, RequestCopilotReviewTool);

            // Repository tools (2)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreateRepositoryTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, ForkRepositoryTool);

            // Branch/Commit tools (4)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, ListBranchesTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreateBranchTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, ListCommitsTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetCommitTool);

            // Search tools (3)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, SearchCodeTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, SearchRepositoriesTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, SearchUsersTool);

            Ok(RouterSet::new(tool_router, prompt_router, managers))
        })
    }).await
}
