//! Dependency freshness scoring based on lock file modification times

use std::path::Path;
use std::time::SystemTime;

/// Calculate dependency freshness based on lock file modification time
/// Returns score from 0.0 (very stale) to 1.0 (very fresh)
#[inline]
fn calculate_freshness_from_lock_age(lock_file_path: &Path) -> f32 {
    // Attempt to get file metadata
    let metadata = match std::fs::metadata(lock_file_path) {
        Ok(m) => m,
        Err(_) => return 0.5, // File doesn't exist or inaccessible, neutral score
    };

    // Get modification time
    let modified = match metadata.modified() {
        Ok(time) => time,
        Err(_) => return 0.5, // Can't read modification time, neutral score
    };

    // Calculate age
    let now = SystemTime::now();
    let age = match now.duration_since(modified) {
        Ok(duration) => duration,
        Err(_) => return 0.5, // Clock skew or future timestamp, neutral score
    };

    // Convert to days
    let days = age.as_secs() / 86400;

    // Exponential decay scoring based on age thresholds
    match days {
        0..=30 => 1.0,     // < 1 month: excellent
        31..=90 => 0.9,    // 1-3 months: very good
        91..=180 => 0.8,   // 3-6 months: good
        181..=365 => 0.6,  // 6-12 months: acceptable
        366..=730 => 0.4,  // 1-2 years: aging
        731..=1095 => 0.2, // 2-3 years: stale
        _ => 0.1,          // > 3 years: very stale
    }
}

/// Helper to calculate freshness from `SystemTime` timestamp
#[inline]
fn calculate_freshness_from_timestamp(modified: SystemTime) -> f32 {
    let now = SystemTime::now();
    let age = match now.duration_since(modified) {
        Ok(duration) => duration,
        Err(_) => return 0.5,
    };

    let days = age.as_secs() / 86400;

    match days {
        0..=30 => 1.0,
        31..=90 => 0.9,
        91..=180 => 0.8,
        181..=365 => 0.6,
        366..=730 => 0.4,
        731..=1095 => 0.2,
        _ => 0.1,
    }
}

/// Get the most recent modification time from multiple possible lock files
#[inline]
fn get_most_recent_lock_file(repo_path: &Path, lock_files: &[&str]) -> Option<SystemTime> {
    lock_files
        .iter()
        .filter_map(|&filename| {
            let path = repo_path.join(filename);
            std::fs::metadata(&path).ok()?.modified().ok()
        })
        .max() // Get the most recent timestamp
}

/// Calculate dependency freshness based on lock file ages
/// Returns score from 0.0 to 1.0
pub(crate) fn calculate_dependency_freshness(
    repo_path: &Path,
    package_managers: &[String],
) -> f32 {
    if package_managers.is_empty() {
        return 0.0; // No dependency management
    }

    let mut first_score: Option<f32> = None;
    let mut additional_scores: Vec<f32> = Vec::new();

    // Check each detected package manager's lock files
    for pm in package_managers {
        let score = match pm.as_str() {
            "Cargo" => {
                let lock_path = repo_path.join("Cargo.lock");
                if lock_path.exists() {
                    calculate_freshness_from_lock_age(&lock_path)
                } else {
                    continue;
                }
            }
            "npm" => {
                let lock_files = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml"];
                if let Some(most_recent) = get_most_recent_lock_file(repo_path, &lock_files) {
                    calculate_freshness_from_timestamp(most_recent)
                } else {
                    0.4 // Has package.json but no lock file
                }
            }
            "pip" => {
                let lock_files = ["Pipfile.lock", "poetry.lock"];
                if let Some(most_recent) = get_most_recent_lock_file(repo_path, &lock_files) {
                    calculate_freshness_from_timestamp(most_recent)
                } else {
                    // Check requirements.txt as fallback
                    let req_path = repo_path.join("requirements.txt");
                    if req_path.exists() {
                        calculate_freshness_from_lock_age(&req_path)
                    } else {
                        0.4
                    }
                }
            }
            "Go modules" => {
                let lock_path = repo_path.join("go.sum");
                if lock_path.exists() {
                    calculate_freshness_from_lock_age(&lock_path)
                } else {
                    0.4
                }
            }
            _ => continue,
        };

        match first_score {
            None => first_score = Some(score),
            Some(_) => additional_scores.push(score),
        }
    }

    // Calculate final score
    match (first_score, additional_scores.is_empty()) {
        (None, _) => 0.4,             // No scores collected
        (Some(score), true) => score, // Single PM: zero allocation path
        (Some(first), false) => {
            // Multiple PMs: average all scores
            let sum = first + additional_scores.iter().sum::<f32>();
            sum / (1 + additional_scores.len()) as f32
        }
    }
}
