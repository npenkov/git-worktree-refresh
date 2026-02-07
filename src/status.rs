use crate::git;
use crate::types::{FetchResult, RepoKind, RepoStatus, WorktreeInfo};

pub async fn build_repo_statuses(fetch_results: Vec<FetchResult>) -> Vec<RepoStatus> {
    let mut statuses = Vec::with_capacity(fetch_results.len());

    for result in fetch_results {
        let worktrees = gather_worktrees(&result).await;
        statuses.push(RepoStatus {
            repo: result.repo,
            fetch_outcome: result.outcome,
            worktrees,
        });
    }

    statuses
}

async fn gather_worktrees(result: &FetchResult) -> Vec<WorktreeInfo> {
    let repo = &result.repo;

    // For bare repos, list worktrees via git worktree list
    // For non-bare repos, the repo itself is the worktree
    let mut worktrees = match repo.kind {
        RepoKind::Bare => match git::list_worktrees(&repo.path, repo.kind).await {
            Ok(wts) => wts
                .into_iter()
                .filter(|wt| {
                    // Skip the bare repo entry itself (path matches repo path)
                    wt.path != repo.path
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        RepoKind::NonBare => {
            // For non-bare, get the branch name
            match git::list_worktrees(&repo.path, repo.kind).await {
                Ok(wts) => wts,
                Err(_) => Vec::new(),
            }
        }
    };

    // Gather ahead/behind for each worktree
    for wt in &mut worktrees {
        if wt.branch.is_some() && wt.detached_head.is_none() {
            wt.ahead_behind = git::ahead_behind(&wt.path).await;
        }
    }

    worktrees
}
