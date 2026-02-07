use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::types::{DiscoveredRepo, RepoKind};

pub fn discover_repos(directories: &[PathBuf], max_depth: usize) -> Vec<DiscoveredRepo> {
    let mut repos = Vec::new();
    let mut seen = HashSet::new();

    for dir in directories {
        if !dir.exists() {
            eprintln!("Warning: directory does not exist: {}", dir.display());
            continue;
        }
        if let Err(e) = scan_dir(dir, max_depth, 0, &mut repos, &mut seen) {
            eprintln!("Warning: error scanning {}: {}", dir.display(), e);
        }
    }

    repos
}

fn scan_dir(
    dir: &Path,
    max_depth: usize,
    current_depth: usize,
    repos: &mut Vec<DiscoveredRepo>,
    seen: &mut HashSet<PathBuf>,
) -> Result<()> {
    if current_depth > max_depth {
        return Ok(());
    }

    // Check if this is a git repo
    if let Some(repo) = detect_repo(dir) {
        let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        if seen.insert(canonical) {
            repos.push(repo);
        }
        // Don't descend into repos
        return Ok(());
    }

    // Read directory entries and recurse
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Warning: cannot read {}: {}", dir.display(), e);
            return Ok(());
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Skip hidden directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }

        scan_dir(&path, max_depth, current_depth + 1, repos, seen)?;
    }

    Ok(())
}

fn detect_repo(dir: &Path) -> Option<DiscoveredRepo> {
    let dot_git = dir.join(".git");

    // Non-bare repo: has .git directory (not file — files indicate worktree links)
    if dot_git.is_dir() {
        return Some(DiscoveredRepo {
            path: dir.to_path_buf(),
            kind: RepoKind::NonBare,
        });
    }

    // If .git is a file, this is a linked worktree — skip it
    if dot_git.is_file() {
        return None;
    }

    // Bare repo: has HEAD file + refs/ dir + objects/ dir (and no .git)
    if dir.join("HEAD").is_file()
        && dir.join("refs").is_dir()
        && dir.join("objects").is_dir()
    {
        return Some(DiscoveredRepo {
            path: dir.to_path_buf(),
            kind: RepoKind::Bare,
        });
    }

    None
}
