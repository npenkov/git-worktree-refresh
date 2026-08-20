use owo_colors::OwoColorize;
use owo_colors::Stream::Stdout;

use crate::types::{FetchOutcome, PullResult, RemoteError, RepoKind, RepoStatus};

fn has_worktree_changes(status: &RepoStatus) -> bool {
    status
        .worktrees
        .iter()
        .any(|wt| matches!(wt.ahead_behind, Some((a, b)) if a > 0 || b > 0))
}

pub fn print_results(statuses: &[RepoStatus], emoji: bool, show_all: bool, verbose: bool) {
    let mut shown = 0;
    let mut with_changes = 0;
    let mut partial = 0;
    let mut errors = 0;

    for status in statuses {
        let has_fetch_changes = matches!(
            status.fetch_outcome,
            FetchOutcome::Updated { .. } | FetchOutcome::Partial { refs_updated: 1.., .. }
        );
        let has_wt_changes = has_worktree_changes(status);
        let has_error = status.fetch_outcome.has_error();

        if has_fetch_changes || has_wt_changes {
            with_changes += 1;
        }
        match status.fetch_outcome {
            FetchOutcome::Partial { .. } => partial += 1,
            FetchOutcome::Failed { .. } | FetchOutcome::Error(_) => errors += 1,
            _ => {}
        }

        if !show_all && !has_fetch_changes && !has_wt_changes && !has_error {
            continue;
        }

        print_repo(status, emoji, verbose);
        shown += 1;
    }

    if shown > 0 {
        println!();
    }

    // Summary line
    let total = statuses.len();
    let summary_prefix = if emoji { "📊 " } else { "" };
    let partial_part = if partial > 0 {
        format!(
            ", {} partial",
            partial.if_supports_color(Stdout, |t| t.yellow())
        )
    } else {
        String::new()
    };
    println!(
        "{}Scanned {} repo(s): {} with changes{}, {} error(s)",
        summary_prefix,
        total.if_supports_color(Stdout, |t| t.bold()),
        with_changes.if_supports_color(Stdout, |t| t.green()),
        partial_part,
        errors.if_supports_color(Stdout, |t| if errors > 0 {
            t.red().to_string()
        } else {
            t.to_string()
        })
    );
}

fn print_repo(status: &RepoStatus, emoji: bool, verbose: bool) {
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

    let updated_arrow = if emoji { " 📥" } else { "" };
    let warn = if emoji { "⚠️ " } else { "! " };

    // The header always stays on a single line; per-remote detail is printed
    // as indented lines below it.
    let fetch_info = match &status.fetch_outcome {
        FetchOutcome::Updated { refs_updated } => format!(
            "{} {} ref(s) updated",
            updated_arrow,
            refs_updated.if_supports_color(Stdout, |t| t.yellow())
        ),
        FetchOutcome::NoChanges => " (no changes)".to_string(),
        FetchOutcome::NoRemote => " (no remote)".to_string(),
        FetchOutcome::Skipped => " (fetch skipped)".to_string(),
        FetchOutcome::Partial {
            refs_updated,
            failed,
            total_remotes,
        } => {
            let updated_part = if *refs_updated > 0 {
                format!(
                    "{} {} ref(s) updated",
                    updated_arrow,
                    refs_updated.if_supports_color(Stdout, |t| t.yellow())
                )
            } else {
                " (no changes)".to_string()
            };
            format!(
                "{}, {}",
                updated_part,
                format!("{}{}/{} remotes failed", warn, failed.len(), total_remotes)
                    .if_supports_color(Stdout, |t| t.red())
            )
        }
        FetchOutcome::Failed { failed } => format!(
            " {}",
            format!("{}fetch failed ({}/{} remotes)", warn, failed.len(), failed.len())
                .if_supports_color(Stdout, |t| t.red())
        ),
        FetchOutcome::Error(e) => format!(
            " {}",
            format!("{}{}", warn, e).if_supports_color(Stdout, |t| t.red())
        ),
    };

    println!(
        "{}{}{}{}",
        prefix,
        repo_name.if_supports_color(Stdout, |t| t.bold()),
        kind_str,
        fetch_info
    );

    match &status.fetch_outcome {
        FetchOutcome::Partial { failed, .. } | FetchOutcome::Failed { failed } => {
            print_remote_errors(failed, verbose)
        }
        _ => {}
    }

    let stale = status.fetch_outcome.is_stale();
    for wt in &status.worktrees {
        print_worktree(wt, emoji, stale);
    }
}

fn print_remote_errors(failed: &[RemoteError], verbose: bool) {
    for err in failed {
        if verbose {
            println!(
                "     {}:",
                err.remote.if_supports_color(Stdout, |t| t.red())
            );
            for line in err.detail.lines() {
                println!("       {}", line.trim_end());
            }
        } else {
            println!(
                "     {}: {}",
                err.remote.if_supports_color(Stdout, |t| t.red()),
                err.message
            );
        }
    }
}

fn print_worktree(wt: &crate::types::WorktreeInfo, emoji: bool, stale: bool) {
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
        // Nothing was fetched, so "up to date" would be a claim we cannot make.
        Some((0, 0)) if stale => " (status stale)"
            .if_supports_color(Stdout, |t| t.dimmed())
            .to_string(),
        Some((0, 0)) => {
            if emoji {
                " ✅ up to date"
                    .if_supports_color(Stdout, |t| t.green())
                    .to_string()
            } else {
                " up to date"
                    .if_supports_color(Stdout, |t| t.green())
                    .to_string()
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
                " ✨ pulled"
                    .if_supports_color(Stdout, |t| t.green())
                    .to_string()
            } else {
                " (pulled)"
                    .if_supports_color(Stdout, |t| t.green())
                    .to_string()
            }
        }
        Some(PullResult::Failed(e)) => {
            format!(" pull failed: {}", e.if_supports_color(Stdout, |t| t.red()))
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
