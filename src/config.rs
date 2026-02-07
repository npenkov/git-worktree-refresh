use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;

use crate::cli::Cli;
use crate::types::AppConfig;

#[derive(Debug, Deserialize, Default)]
pub struct FileConfig {
    pub directories: Option<Vec<String>>,
    pub concurrency: Option<usize>,
    pub emoji: Option<bool>,
    pub auto_pull: Option<bool>,
    pub max_depth: Option<usize>,
    pub show_all: Option<bool>,
}

fn default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "git-worktree-refresh")
        .map(|dirs| dirs.config_dir().join("config.yaml"))
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn dirs_home() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())
}

fn load_file_config(path: &Path) -> Option<FileConfig> {
    let contents = std::fs::read_to_string(path).ok()?;
    match serde_yaml_ng::from_str(&contents) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("Warning: failed to parse config {}: {}", path.display(), e);
            None
        }
    }
}

pub fn resolve_config(cli: &Cli) -> Result<AppConfig> {
    let mut config = AppConfig::default();

    // Load file config
    let config_path = cli
        .config
        .clone()
        .or_else(default_config_path);

    if let Some(path) = config_path {
        if let Some(file_cfg) = load_file_config(&path) {
            if let Some(dirs) = file_cfg.directories {
                config.directories = dirs.iter().map(|d| expand_tilde(d)).collect();
            }
            if let Some(c) = file_cfg.concurrency {
                config.concurrency = c;
            }
            if let Some(e) = file_cfg.emoji {
                config.emoji = e;
            }
            if let Some(ap) = file_cfg.auto_pull {
                config.auto_pull = ap;
            }
            if let Some(md) = file_cfg.max_depth {
                config.max_depth = md;
            }
            if let Some(sa) = file_cfg.show_all {
                config.show_all = sa;
            }
        }
    }

    // CLI overrides
    if !cli.directories.is_empty() {
        config.directories = cli.directories.clone();
    }
    if let Some(c) = cli.concurrency {
        config.concurrency = c;
    }
    if cli.no_emoji {
        config.emoji = false;
    }
    if cli.auto_pull {
        config.auto_pull = true;
    }
    if let Some(md) = cli.max_depth {
        config.max_depth = md;
    }
    if cli.show_all {
        config.show_all = true;
    }

    if config.directories.is_empty() {
        anyhow::bail!(
            "No directories configured.\n\
             Specify directories with -d <DIR> or in config file.\n\
             Config file location: {}",
            default_config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "~/.config/git-worktree-refresh/config.yaml".into())
        );
    }

    Ok(config)
}
