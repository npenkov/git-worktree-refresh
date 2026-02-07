use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::git;
use crate::types::{DiscoveredRepo, FetchOutcome, FetchResult};

pub async fn fetch_all_repos(
    repos: Vec<DiscoveredRepo>,
    concurrency: usize,
) -> Vec<FetchResult> {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(repos.len());

    for repo in repos {
        let sem = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let outcome = fetch_one(&repo).await;
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

    results
}

async fn fetch_one(repo: &DiscoveredRepo) -> FetchOutcome {
    if !git::has_remote(&repo.path, repo.kind).await {
        return FetchOutcome::NoRemote;
    }
    git::fetch_all(&repo.path, repo.kind).await
}
