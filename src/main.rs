mod autopull;
mod cli;
mod config;
mod discovery;
mod fetch;
mod git;
mod output;
mod status;
mod types;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let config = config::resolve_config(&cli)?;

    // Ensure git is available
    git::check_git_available().await?;

    // Discover repos
    let repos = discovery::discover_repos(&config.directories, config.max_depth);
    if repos.is_empty() {
        println!("No git repositories found in configured directories.");
        return Ok(());
    }

    // Fetch all repos in parallel
    let fetch_results = fetch::fetch_all_repos(repos, config.concurrency).await;

    // Build status with worktree info
    let mut statuses = status::build_repo_statuses(fetch_results).await;

    // Auto-pull if enabled
    if config.auto_pull {
        autopull::auto_pull_eligible(&mut statuses).await;
    }

    // Print results
    output::print_results(&statuses, config.emoji, config.show_all);

    Ok(())
}
