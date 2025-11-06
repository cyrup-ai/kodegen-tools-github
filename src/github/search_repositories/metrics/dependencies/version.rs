//! Semantic version comparison utilities

use semver::Version;

/// Compare two semantic versions, returns true if current < latest
pub(crate) fn is_outdated(current: &str, latest: &str) -> bool {
    // Clean version strings (remove ^, ~, >=, etc.)
    let clean_current = current
        .trim_start_matches(&['<', '>', '=', '^', '~', 'v'][..])
        .split_whitespace()
        .next()
        .unwrap_or(current);

    let clean_latest = latest
        .trim_start_matches('v')
        .split_whitespace()
        .next()
        .unwrap_or(latest);

    match (Version::parse(clean_current), Version::parse(clean_latest)) {
        (Ok(curr_ver), Ok(latest_ver)) => curr_ver < latest_ver,
        _ => false, // Can't determine, assume not outdated
    }
}
