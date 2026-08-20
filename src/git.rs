use std::path::Path;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::types::{FetchOutcome, RemoteError, RepoKind, WorktreeInfo};

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

/// List the configured remotes for a repo, in git's order.
pub async fn list_remotes(repo_path: &Path, kind: RepoKind) -> Result<Vec<String>> {
    let output = git_cmd(repo_path, kind, &["remote"]).await?;
    Ok(output
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect())
}

/// Fetch every remote individually so one broken remote does not hide the
/// others. `git fetch --all` exits non-zero if any single remote fails, which
/// discards the ref updates of the remotes that did succeed.
pub async fn fetch_all(repo_path: &Path, kind: RepoKind) -> FetchOutcome {
    let remotes = match list_remotes(repo_path, kind).await {
        Ok(remotes) => remotes,
        Err(e) => return FetchOutcome::Error(first_line(&e.to_string())),
    };

    if remotes.is_empty() {
        return FetchOutcome::NoRemote;
    }

    let total_remotes = remotes.len();
    let mut refs_updated = 0;
    let mut failed: Vec<RemoteError> = Vec::new();

    for remote in remotes {
        match fetch_remote(repo_path, kind, &remote).await {
            Ok(updated) => refs_updated += updated,
            Err(stderr) => failed.push(RemoteError {
                remote,
                message: first_line(&stderr),
                detail: stderr,
            }),
        }
    }

    if failed.is_empty() {
        if refs_updated > 0 {
            FetchOutcome::Updated { refs_updated }
        } else {
            FetchOutcome::NoChanges
        }
    } else if failed.len() == total_remotes {
        FetchOutcome::Failed { failed }
    } else {
        FetchOutcome::Partial {
            refs_updated,
            failed,
            total_remotes,
        }
    }
}

/// Fetch a single remote. On success returns the number of refs updated;
/// on failure returns git's stderr.
async fn fetch_remote(
    repo_path: &Path,
    kind: RepoKind,
    remote: &str,
) -> std::result::Result<usize, String> {
    match git_cmd_raw(repo_path, kind, &["fetch", "--prune", remote]).await {
        Ok((true, _, stderr)) => Ok(count_ref_updates(&stderr)),
        Ok((false, _, stderr)) => Err(stderr.trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// First meaningful line of a git error, for one-line summaries. Git sometimes
/// splits a message across two lines ("fatal: remote error:" / "  <reason>"),
/// so a line ending in a colon is joined with the one that follows.
fn first_line(text: &str) -> String {
    const MAX: usize = 200;
    let mut lines = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    let mut summary = lines.next().unwrap_or("unknown error").to_string();
    if summary.ends_with(':') {
        if let Some(next) = lines.next() {
            summary.push(' ');
            summary.push_str(next);
        }
    }
    if summary.chars().count() > MAX {
        let truncated: String = summary.chars().take(MAX).collect();
        format!("{}...", truncated)
    } else {
        summary
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

/// Run a git command without treating a non-zero exit as an error.
/// Returns (success, stdout, stderr).
async fn git_cmd_raw(
    repo_path: &Path,
    kind: RepoKind,
    args: &[&str],
) -> Result<(bool, String, String)> {
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

    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
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
