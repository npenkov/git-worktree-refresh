use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::git;
use crate::types::{DiscoveredRepo, FetchOutcome, FetchResult};

pub async fn fetch_all_repos(
    repos: Vec<DiscoveredRepo>,
    concurrency: usize,
) -> Vec<FetchResult> {
    let total = repos.len();
    let counter = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(total);

    for repo in repos {
        let sem = semaphore.clone();
        let counter = counter.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let outcome = fetch_one(&repo).await;
            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            eprint!("\rFetching... [{}/{}]", done, total);
            FetchResult { repo, outcome }
        }));
    }

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => eprintln!("Warning: fetch task panicked: {}", e),
        }
    }

    // Clear the progress line
    eprint!("\r{}\r", " ".repeat(30));

    results
}

async fn fetch_one(repo: &DiscoveredRepo) -> FetchOutcome {
    if !git::has_remote(&repo.path, repo.kind).await {
        return FetchOutcome::NoRemote;
    }
    git::fetch_all(&repo.path, repo.kind).await
}
