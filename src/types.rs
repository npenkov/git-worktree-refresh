use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DiscoveredRepo {
    pub path: PathBuf,
    pub kind: RepoKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoKind {
    Bare,
    NonBare,
}

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub repo: DiscoveredRepo,
    pub outcome: FetchOutcome,
}

#[derive(Debug, Clone)]
pub enum FetchOutcome {
    Updated { refs_updated: usize },
    NoChanges,
    NoRemote,
    Skipped,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub detached_head: Option<String>,
    pub ahead_behind: Option<(usize, usize)>,
    pub pull_result: Option<PullResult>,
}

#[derive(Debug, Clone)]
pub enum PullResult {
    Pulled,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct RepoStatus {
    pub repo: DiscoveredRepo,
    pub fetch_outcome: FetchOutcome,
    pub worktrees: Vec<WorktreeInfo>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub directories: Vec<PathBuf>,
    pub concurrency: usize,
    pub fetch: bool,
    pub emoji: bool,
    pub auto_pull: bool,
    pub max_depth: usize,
    pub show_all: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            directories: Vec::new(),
            concurrency: 5,
            fetch: true,
            emoji: true,
            auto_pull: false,
            max_depth: 3,
            show_all: false,
        }
    }
}
