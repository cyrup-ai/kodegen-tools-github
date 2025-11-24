use anyhow;
use kodegen_mcp_tool::{McpError, Tool, ToolExecutionContext};
use kodegen_mcp_schema::github::{ForkRepositoryArgs, GitHubForkRepositoryPromptArgs, GITHUB_FORK_REPOSITORY};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::GitHubClient;

/// Tool for forking a repository
pub struct ForkRepositoryTool;

impl Tool for ForkRepositoryTool {
    type Args = ForkRepositoryArgs;
    type PromptArgs = GitHubForkRepositoryPromptArgs;

    fn name() -> &'static str {
        GITHUB_FORK_REPOSITORY
    }

    fn description() -> &'static str {
        "Fork a repository to your account or an organization"
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false
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
            .fork_repository(args.owner.clone(), args.repo.clone(), args.organization.clone())
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let repository =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Build human-readable summary with ANSI colors and Nerd Font icons
        let fork_full_name = repository.full_name.as_deref().unwrap_or("N/A");
        let html_url = repository.html_url.as_ref().map(|u| u.as_str()).unwrap_or("N/A");

        let summary = format!(
            "\x1b[32m Repository Forked: {}/{}\x1b[0m\n  Fork: {} · URL: {}",
            args.owner,
            args.repo,
            fork_full_name,
            html_url
        );

        // Serialize full metadata
        let json_str = serde_json::to_string_pretty(&repository)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(vec![
            Content::text(summary),
            Content::text(json_str),
        ])
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        // Normalize arguments with sensible defaults
        let scenario = args.scenario
            .as_deref()
            .unwrap_or("all")
            .to_lowercase();
        let depth = args.depth
            .as_deref()
            .unwrap_or("detailed")
            .to_lowercase();
        let include_troubleshooting = args.include_troubleshooting.unwrap_or(true);

        // Build prompt content dynamically based on args
        let mut content = String::new();

        // Title and introduction
        content.push_str("# GitHub Repository Fork Guide\n\n");

        // BASIC SCENARIO OVERVIEW
        if scenario == "all" || scenario == "personal-account" {
            content.push_str("## Fork to Your Personal Account\n");
            content.push_str("To fork a repository to your personal GitHub account:\n\n");
            content.push_str("```json\n");
            content.push_str("{\n");
            content.push_str("  \"owner\": \"octocat\",\n");
            content.push_str("  \"repo\": \"hello-world\"\n");
            content.push_str("}\n");
            content.push_str("```\n\n");
        }

        if scenario == "all" || scenario == "organization" {
            if scenario == "all" {
                content.push_str("## Fork to an Organization\n");
            } else {
                content.push_str("## Fork to Your Organization\n");
            }
            content.push_str("To fork a repository to an organization you belong to:\n\n");
            content.push_str("```json\n");
            content.push_str("{\n");
            content.push_str("  \"owner\": \"octocat\",\n");
            content.push_str("  \"repo\": \"hello-world\",\n");
            content.push_str("  \"organization\": \"my-org\"\n");
            content.push_str("}\n");
            content.push_str("```\n\n");
        }

        // COMMON USE CASES (include at all depths)
        content.push_str("## Common Use Cases\n\n");
        content.push_str("1. **Contributing**: Fork a project to make contributions via pull requests\n");
        content.push_str("2. **Experimentation**: Fork to try changes without affecting the original\n");
        content.push_str("3. **Starting Point**: Fork to use as a template for your own project\n");
        content.push_str("4. **Organization Copy**: Fork to your organization for internal use\n\n");

        // WHAT IS A FORK (include at all depths)
        content.push_str("## What is a Fork?\n\n");
        content.push_str("A fork is a complete copy of a repository that:\n");
        content.push_str("- Lives under your account or organization\n");
        content.push_str("- Maintains a connection to the original repository\n");
        content.push_str("- Allows you to freely experiment with changes\n");
        content.push_str("- Enables contributing back via pull requests\n");
        content.push_str("- Includes all branches, commits, and history\n\n");

        // WORKFLOW AFTER FORKING (only for detailed/advanced)
        if depth != "basic" {
            content.push_str("## Workflow After Forking\n\n");
            content.push_str("1. **Fork** the repository (this tool)\n");
            content.push_str("2. **Clone** your fork to your local machine\n");
            content.push_str("3. **Create** a new branch for your changes\n");
            content.push_str("4. **Make** your changes and commit them\n");
            content.push_str("5. **Push** to your fork\n");
            content.push_str("6. **Create** a pull request to the original repository\n\n");
        }

        // BEST PRACTICES
        content.push_str("## Best Practices\n\n");
        content.push_str("- Fork when you plan to contribute back to the project\n");
        if depth != "basic" {
            content.push_str("- Keep your fork synced with the upstream repository\n");
        }
        content.push_str("- Use descriptive branch names for your changes\n");
        content.push_str("- Follow the project's contribution guidelines\n");
        if depth != "basic" {
            content.push_str("- Test your changes before creating pull requests\n\n");
        } else {
            content.push('\n');
        }

        // IMPORTANT NOTES
        content.push_str("## Important Notes\n\n");
        content.push_str("- Forking is instantaneous but may take a few moments for large repositories\n");
        content.push_str("- You cannot fork your own repositories\n");
        if depth != "basic" {
            content.push_str("- You cannot fork a repository you've already forked (delete the old fork first)\n");
        }
        content.push_str("- Forks maintain a link to the upstream (original) repository\n");
        if depth == "advanced" {
            content.push_str("- You can configure whether to fork all branches or just the default branch\n");
        }
        content.push('\n');

        // ADVANCED: Fork Syncing and Management (only for advanced depth)
        if depth == "advanced" {
            content.push_str("## Advanced: Keeping Your Fork Synced\n\n");
            content.push_str("### Understanding Fork Branches\n\n");
            content.push_str("When you fork a repository, you get:\n");
            content.push_str("- **Your default branch**: Points to your fork's main development\n");
            content.push_str("- **Upstream tracking**: GitHub automatically creates tracking branches if you pull from upstream\n");
            content.push_str("- **Multiple remotes**: Configure 'origin' (your fork) and 'upstream' (original repo)\n\n");

            content.push_str("### Syncing with Upstream\n\n");
            content.push_str("```bash\n");
            content.push_str("# Add upstream remote\n");
            content.push_str("git remote add upstream https://github.com/ORIGINAL_OWNER/ORIGINAL_REPO.git\n\n");
            content.push_str("# Fetch upstream changes\n");
            content.push_str("git fetch upstream\n\n");
            content.push_str("# Rebase your branch on upstream main\n");
            content.push_str("git rebase upstream/main\n\n");
            content.push_str("# Force push to your fork\n");
            content.push_str("git push origin your-branch-name --force-with-lease\n");
            content.push_str("```\n\n");

            content.push_str("### Branch Protection for Forks\n\n");
            content.push_str("Consider protecting your main branch from accidental pushes:\n");
            content.push_str("- Use branch protection rules in fork settings\n");
            content.push_str("- Require pull request reviews before merging\n");
            content.push_str("- Protect against history rewrites\n\n");

            content.push_str("### Managing Multiple Forks\n\n");
            content.push_str("If maintaining multiple forks:\n");
            content.push_str("- Use descriptive fork names/descriptions\n");
            content.push_str("- Document the purpose of each fork in README\n");
            content.push_str("- Consider using GitHub organizations to group related forks\n");
            content.push_str("- Archive unused forks to reduce clutter\n\n");
        }

        // RESPONSE INFORMATION
        if depth != "basic" {
            content.push_str("## Response Information\n\n");
            content.push_str("The response includes:\n");
            content.push_str("- **id**: Unique repository ID of the fork\n");
            content.push_str("- **full_name**: Your username or org/repo format\n");
            content.push_str("- **html_url**: Web URL to your forked repository\n");
            content.push_str("- **clone_url**: HTTPS clone URL for your fork\n");
            content.push_str("- **fork**: true (indicates this is a fork)\n");
            content.push_str("- **parent**: Information about the original repository\n");
            content.push_str("- **source**: Information about the root repository (if parent is also a fork)\n\n");
        }

        // TROUBLESHOOTING (conditional based on include_troubleshooting)
        if include_troubleshooting {
            content.push_str("## Troubleshooting Common Issues\n\n");

            content.push_str("### Issue: Fork Button Missing or Disabled\n");
            content.push_str("**Cause**: You're trying to fork your own repository or a repository you've already forked.\n");
            content.push_str("**Solution**: \n");
            content.push_str("- To fork again: Delete your existing fork and try again\n");
            content.push_str("- Use GitHub Settings → Repositories → Delete Repository\n");
            content.push_str("- Wait a few minutes before forking again\n\n");

            content.push_str("### Issue: Fork Takes Too Long\n");
            content.push_str("**Cause**: The repository is very large or the API is experiencing delays.\n");
            content.push_str("**Solution**:\n");
            content.push_str("- For large repos: Fork completes but may take several minutes\n");
            content.push_str("- Check fork status by refreshing your profile\n");
            content.push_str("- Use GitHub API to poll fork status if automating\n\n");

            content.push_str("### Issue: Can't Push to Fork\n");
            content.push_str("**Cause**: Authentication issues or permission problems.\n");
            content.push_str("**Solution**:\n");
            content.push_str("- Verify your GitHub token has 'repo' scope\n");
            content.push_str("- Check SSH key configuration: `ssh -T git@github.com`\n");
            content.push_str("- Verify clone URL uses correct authentication method\n");
            content.push_str("- Run `git remote -v` to check remote URLs\n\n");

            content.push_str("### Issue: Fork is Out of Sync with Original\n");
            content.push_str("**Cause**: Original repository has new commits, but your fork hasn't been updated.\n");
            content.push_str("**Solution**: See 'Advanced: Keeping Your Fork Synced' section above.\n\n");

            content.push_str("### Issue: Accidental Changes in Main Branch\n");
            content.push_str("**Cause**: Made commits to main branch instead of a feature branch.\n");
            content.push_str("**Solution**:\n");
            content.push_str("- Create new branch from your current main: `git checkout -b feature-branch`\n");
            content.push_str("- Reset main to upstream: `git checkout main && git reset --hard upstream/main`\n");
            content.push_str("- Continue work on feature branch\n\n");
        }

        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(content),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "scenario".to_string(),
                title: Some("Forking Scenario".to_string()),
                description: Some(
                    "Focus on a specific forking scenario: 'personal-account' (fork to your personal GitHub account), \
                     'organization' (fork to an organization you belong to), or 'all' (comprehensive examples for both scenarios). \
                     Default: 'all'".to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "depth".to_string(),
                title: Some("Learning Depth".to_string()),
                description: Some(
                    "Learning depth level: 'basic' (simplified for GitHub newcomers), 'detailed' (comprehensive with workflows, default), \
                     or 'advanced' (deep dive with fork management and syncing strategies). Default: 'detailed'".to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "include_troubleshooting".to_string(),
                title: Some("Include Troubleshooting".to_string()),
                description: Some(
                    "Include a troubleshooting section covering common forking issues and gotchas (true/false). \
                     Default: true".to_string(),
                ),
                required: Some(false),
            },
        ]
    }
}
