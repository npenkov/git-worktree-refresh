use clap::Parser;
use std::path::PathBuf;

/// Scan directories for git repos, fetch remotes in parallel,
/// and show worktree behind/ahead status.
#[derive(Parser, Debug)]
#[command(name = "git-worktree-refresh", version, about)]
pub struct Cli {
    /// Directories to scan (repeatable)
    #[arg(short = 'd', long = "directories", value_name = "DIR")]
    pub directories: Vec<PathBuf>,

    /// Max parallel fetch operations
    #[arg(short = 'j', long, value_name = "N")]
    pub concurrency: Option<usize>,

    /// Disable emoji in output
    #[arg(long)]
    pub no_emoji: bool,

    /// Pull changes into FF-safe worktrees
    #[arg(long)]
    pub auto_pull: bool,

    /// Custom config file path
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Max directory scan depth
    #[arg(long, value_name = "N")]
    pub max_depth: Option<usize>,

    /// Show repos even if no changes fetched
    #[arg(long)]
    pub show_all: bool,
}
