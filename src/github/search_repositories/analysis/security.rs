//! Security scoring calculations

use crate::github::search_repositories::types::SecurityMetrics;

/// Calculate signed commits ratio from a gix repository
pub(crate) fn calculate_signed_commits_ratio(repo: &gix::Repository) -> f32 {
    let mut total_commits = 0u32;
    let mut signed_commits = 0u32;

    if let Ok(head) = repo.head()
        && let Some(mut head_ref) = head.try_into_referent()
        && let Ok(peeled) = head_ref.peel_to_id()
    {
        let walk = peeled
            .ancestors()
            .sorting(gix::revision::walk::Sorting::ByCommitTime(
                Default::default(),
            ))
            .all();

        if let Ok(iter) = walk {
            for commit_info in iter.take(100).flatten() {
                if let Ok(commit_obj) = repo.find_object(commit_info.id)
                    && let Ok(commit) = commit_obj.try_into_commit()
                {
                    total_commits += 1;
                    if commit.signature().is_ok() {
                        signed_commits += 1;
                    }
                }
            }
        }
    }

    if total_commits > 0 {
        signed_commits as f32 / total_commits as f32
    } else {
        0.0
    }
}

/// Calculate security score from `SecurityMetrics` components
pub(crate) fn calculate_security_score(metrics: &SecurityMetrics) -> f32 {
    let mut score = 0.0;

    // Security policy presence (15%)
    if metrics.security_policy {
        score += 0.15;
    }

    // Vulnerability disclosure process (10%)
    if metrics.vulnerability_disclosure {
        score += 0.10;
    }

    // Dependency scanning enabled (20%)
    if metrics.dependency_scanning {
        score += 0.20;
    }

    // No secrets detected (20%)
    // NOTE: secrets_scanning=true means secrets FOUND (bad)
    if !metrics.secrets_scanning {
        score += 0.20;
    }

    // Signed commits ratio (20%)
    score += metrics.signed_commits_ratio * 0.20;

    // No security advisories (10%)
    if metrics.security_advisories == 0 {
        score += 0.10;
    }

    // Low CVE references (5%)
    if metrics.cve_references == 0 {
        score += 0.05;
    } else if metrics.cve_references <= 2 {
        score += 0.025;
    }

    score.min(1.0)
}
