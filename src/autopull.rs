use crate::git;
use crate::types::{PullResult, RepoStatus};

pub async fn auto_pull_eligible(statuses: &mut [RepoStatus]) {
    for status in statuses.iter_mut() {
        for wt in &mut status.worktrees {
            // Only eligible if: has branch, not detached, has upstream,
            // behind > 0, ahead == 0
            let eligible = wt.branch.is_some()
                && wt.detached_head.is_none()
                && matches!(wt.ahead_behind, Some((0, behind)) if behind > 0);

            if !eligible {
                continue;
            }

            match git::pull_ff_only(&wt.path).await {
                Ok(()) => {
                    wt.pull_result = Some(PullResult::Pulled);
                    // Refresh ahead/behind after pull
                    wt.ahead_behind = git::ahead_behind(&wt.path).await;
                }
                Err(e) => {
                    wt.pull_result = Some(PullResult::Failed(e.to_string()));
                }
            }
        }
    }
}
