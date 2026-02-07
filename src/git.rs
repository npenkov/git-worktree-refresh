use std::path::Path;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::types::{FetchOutcome, RepoKind, WorktreeInfo};

pub async fn check_git_available() -> Result<()> {
    let output = Command::new("git")
        .arg("--version")
        .output()
        .await
        .context("git is not installed or not in PATH")?;
    if !output.status.success() {
        anyhow::bail!("git --version failed");
    }
    Ok(())
}

pub async fn has_remote(repo_path: &Path, kind: RepoKind) -> bool {
    let result = git_cmd(repo_path, kind, &["remote"]).await;
    match result {
        Ok(output) => !output.trim().is_empty(),
        Err(_) => false,
    }
}

pub async fn fetch_all(repo_path: &Path, kind: RepoKind) -> FetchOutcome {
    let result = git_cmd_full(repo_path, kind, &["fetch", "--all", "--prune"]).await;
    match result {
        Ok((_, stderr)) => {
            let refs_updated = count_ref_updates(&stderr);
            if refs_updated > 0 {
                FetchOutcome::Updated { refs_updated }
            } else {
                FetchOutcome::NoChanges
            }
        }
        Err(e) => FetchOutcome::Error(e.to_string()),
    }
}

fn count_ref_updates(stderr: &str) -> usize {
    // git fetch prints lines like:
    //   abc1234..def5678  main       -> origin/main
    //   * [new branch]    feature    -> origin/feature
    //   - [deleted]        (none)    -> origin/old-branch
    stderr
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.contains("->")
                && (trimmed.contains("..")
                    || trimmed.starts_with("* [new")
                    || trimmed.starts_with("- [deleted]")
                    || trimmed.starts_with('+'))
        })
        .count()
}

pub async fn list_worktrees(repo_path: &Path, kind: RepoKind) -> Result<Vec<WorktreeInfo>> {
    let output = git_cmd(repo_path, kind, &["worktree", "list", "--porcelain"]).await?;
    parse_worktree_porcelain(&output)
}

fn parse_worktree_porcelain(output: &str) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<std::path::PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut is_detached = false;
    let mut detached_commit: Option<String> = None;

    for line in output.lines() {
        if let Some(path_str) = line.strip_prefix("worktree ") {
            // Save previous worktree if any
            if let Some(path) = current_path.take() {
                worktrees.push(WorktreeInfo {
                    path,
                    branch: current_branch.take(),
                    detached_head: if is_detached { detached_commit.take() } else { None },
                    ahead_behind: None,
                    pull_result: None,
                });
            }
            current_path = Some(std::path::PathBuf::from(path_str));
            current_branch = None;
            is_detached = false;
            detached_commit = None;
        } else if let Some(ref_str) = line.strip_prefix("branch ") {
            // refs/heads/main -> main
            current_branch = Some(
                ref_str
                    .strip_prefix("refs/heads/")
                    .unwrap_or(ref_str)
                    .to_string(),
            );
        } else if line.starts_with("HEAD ") {
            detached_commit = line.strip_prefix("HEAD ").map(|s| s[..7.min(s.len())].to_string());
        } else if line == "detached" {
            is_detached = true;
        } else if line == "bare" {
            // Mark bare worktree - we'll skip it later
            current_branch = None;
            is_detached = false;
        }
    }

    // Push last worktree
    if let Some(path) = current_path {
        worktrees.push(WorktreeInfo {
            path,
            branch: current_branch,
            detached_head: if is_detached { detached_commit } else { None },
            ahead_behind: None,
            pull_result: None,
        });
    }

    Ok(worktrees)
}

pub async fn ahead_behind(worktree_path: &Path) -> Option<(usize, usize)> {
    let result = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy()])
        .args(["rev-list", "--left-right", "--count", "HEAD...HEAD@{upstream}"])
        .output()
        .await
        .ok()?;

    if !result.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    let parts: Vec<&str> = stdout.trim().split('\t').collect();
    if parts.len() == 2 {
        let ahead = parts[0].parse().ok()?;
        let behind = parts[1].parse().ok()?;
        Some((ahead, behind))
    } else {
        None
    }
}

pub async fn pull_ff_only(worktree_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy()])
        .args(["pull", "--ff-only"])
        .output()
        .await
        .context("failed to run git pull")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{}", stderr.trim());
    }
    Ok(())
}

async fn git_cmd(repo_path: &Path, kind: RepoKind, args: &[&str]) -> Result<String> {
    let (stdout, _) = git_cmd_full(repo_path, kind, args).await?;
    Ok(stdout)
}

async fn git_cmd_full(
    repo_path: &Path,
    kind: RepoKind,
    args: &[&str],
) -> Result<(String, String)> {
    let mut cmd = Command::new("git");

    match kind {
        RepoKind::Bare => {
            cmd.arg("--git-dir").arg(repo_path);
        }
        RepoKind::NonBare => {
            cmd.arg("-C").arg(repo_path);
        }
    }

    cmd.args(args);

    let output = cmd
        .output()
        .await
        .with_context(|| format!("failed to run git {:?} in {}", args, repo_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "git {:?} failed in {}: {}",
            args,
            repo_path.display(),
            stderr.trim()
        );
    }

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}
