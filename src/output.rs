use owo_colors::OwoColorize;
use owo_colors::Stream::Stdout;

use crate::types::{FetchOutcome, PullResult, RepoKind, RepoStatus};

fn has_worktree_changes(status: &RepoStatus) -> bool {
    status.worktrees.iter().any(|wt| matches!(wt.ahead_behind, Some((a, b)) if a > 0 || b > 0))
}

pub fn print_results(statuses: &[RepoStatus], emoji: bool, show_all: bool) {
    let mut shown = 0;
    let mut with_changes = 0;
    let mut errors = 0;

    for status in statuses {
        let has_fetch_changes = matches!(status.fetch_outcome, FetchOutcome::Updated { .. });
        let has_wt_changes = has_worktree_changes(status);
        let has_error = matches!(status.fetch_outcome, FetchOutcome::Error(_));

        if has_fetch_changes || has_wt_changes {
            with_changes += 1;
        }
        if has_error {
            errors += 1;
        }

        if !show_all && !has_fetch_changes && !has_wt_changes && !has_error {
            continue;
        }

        print_repo(status, emoji);
        shown += 1;
    }

    if shown > 0 {
        println!();
    }

    // Summary line
    let total = statuses.len();
    let summary_prefix = if emoji { "📊 " } else { "" };
    println!(
        "{}Scanned {} repo(s): {} with changes, {} error(s)",
        summary_prefix,
        total.if_supports_color(Stdout, |t| t.bold()),
        with_changes.if_supports_color(Stdout, |t| t.green()),
        errors.if_supports_color(Stdout, |t| if errors > 0 { t.red().to_string() } else { t.to_string() })
    );
}

fn print_repo(status: &RepoStatus, emoji: bool) {
    let repo_name = status
        .repo
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| status.repo.path.display().to_string());

    let kind_str = match status.repo.kind {
        RepoKind::Bare => " (bare)",
        RepoKind::NonBare => "",
    };

    let prefix = if emoji {
        match status.repo.kind {
            RepoKind::Bare => "📦 ",
            RepoKind::NonBare => "📁 ",
        }
    } else {
        ""
    };

    let fetch_info = match &status.fetch_outcome {
        FetchOutcome::Updated { refs_updated } => {
            let arrow = if emoji { " 📥" } else { "" };
            format!(
                "{} {} ref(s) updated",
                arrow,
                refs_updated.if_supports_color(Stdout, |t| t.yellow())
            )
        }
        FetchOutcome::NoChanges => " (no changes)".to_string(),
        FetchOutcome::NoRemote => " (no remote)".to_string(),
        FetchOutcome::Error(e) => format!(
            " {}",
            format!("error: {}", e).if_supports_color(Stdout, |t| t.red())
        ),
    };

    println!(
        "{}{}{}{}",
        prefix,
        repo_name.if_supports_color(Stdout, |t| t.bold()),
        kind_str,
        fetch_info
    );

    for wt in &status.worktrees {
        print_worktree(wt, emoji);
    }
}

fn print_worktree(wt: &crate::types::WorktreeInfo, emoji: bool) {
    let wt_name = wt
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| wt.path.display().to_string());

    let branch_display = if let Some(ref branch) = wt.branch {
        branch.if_supports_color(Stdout, |t| t.cyan()).to_string()
    } else if let Some(ref commit) = wt.detached_head {
        format!(
            "(detached {})",
            commit.if_supports_color(Stdout, |t| t.yellow())
        )
    } else {
        "(unknown)".to_string()
    };

    let status_str = match wt.ahead_behind {
        Some((0, 0)) => {
            if emoji {
                " ✅ up to date".if_supports_color(Stdout, |t| t.green()).to_string()
            } else {
                " up to date".if_supports_color(Stdout, |t| t.green()).to_string()
            }
        }
        Some((ahead, behind)) => {
            let mut parts = Vec::new();
            if behind > 0 {
                let arrow = if emoji { "⬇️" } else { "v" };
                parts.push(format!(
                    "{} {}",
                    arrow,
                    behind.if_supports_color(Stdout, |t| t.red())
                ));
            }
            if ahead > 0 {
                let arrow = if emoji { "⬆️" } else { "^" };
                parts.push(format!(
                    "{} {}",
                    arrow,
                    ahead.if_supports_color(Stdout, |t| t.green())
                ));
            }
            format!(" {}", parts.join(" "))
        }
        None => {
            if wt.detached_head.is_some() {
                String::new()
            } else if wt.branch.is_some() {
                " (no upstream)".to_string()
            } else {
                String::new()
            }
        }
    };

    let pull_str = match &wt.pull_result {
        Some(PullResult::Pulled) => {
            if emoji {
                " ✨ pulled".if_supports_color(Stdout, |t| t.green()).to_string()
            } else {
                " (pulled)".if_supports_color(Stdout, |t| t.green()).to_string()
            }
        }
        Some(PullResult::Failed(e)) => {
            format!(
                " pull failed: {}",
                e.if_supports_color(Stdout, |t| t.red())
            )
        }
        None => String::new(),
    };

    println!(
        "  {} {}{}{}",
        wt_name.if_supports_color(Stdout, |t| t.cyan()),
        branch_display,
        status_str,
        pull_str
    );
}
