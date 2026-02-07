# git-worktree-refresh

A fast CLI utility that scans directories for git repositories (bare and non-bare), fetches remote changes in parallel, and displays a summary with per-worktree behind/ahead status.

Built for workflows that use bare repos with multiple worktrees — but works with regular repos too.

## Usage

```
git-worktree-refresh -d ~/src/oss
```

```
📦 proj1.git (bare) 📥 1 ref(s) updated
  proj1-dev dev ✅ up to date
  proj1-prod prod ✅ up to date
  proj1-test test ⬇️ 1

📦 proj1.git (bare) (no changes)
  proj1-dev dev ✅ up to date
  proj1-prod prod ⬇️ 127
  proj1-test test ✅ up to date

📊 Scanned 32 repo(s): 1 with changes, 0 error(s)
```

## Installation

```
cargo install --path .
```

## Options

```
-d, --directories <DIR>   Directories to scan (repeatable)
-j, --concurrency <N>     Max parallel fetch operations (default: 5)
    --no-emoji             Disable emoji in output
    --auto-pull            Pull changes into FF-safe worktrees (--ff-only)
-c, --config <FILE>       Custom config file path
    --max-depth <N>        Max directory scan depth (default: 3)
    --show-all             Show repos even if no changes fetched
```

## Configuration

Reads from `~/.config/git-worktree-refresh/config.yaml` (respects `XDG_CONFIG_HOME`):

```yaml
directories:
  - ~/src/oss
  - ~/src/personal
concurrency: 5
emoji: true
auto_pull: false
max_depth: 3
show_all: false
```

CLI flags override config file values.

## How it works

1. **Discovery** — recursively scans configured directories for git repos. Detects bare repos (has `HEAD` + `refs/` + `objects/`) and non-bare repos (has `.git/` directory). Skips worktree links (`.git` files) and hidden directories.
2. **Fetch** — runs `git fetch --all --prune` on each repo in parallel, bounded by a concurrency semaphore.
3. **Status** — lists worktrees for each repo and checks `ahead/behind` vs upstream using `git rev-list --left-right --count`.
4. **Auto-pull** (optional) — runs `git pull --ff-only` on worktrees that are behind with no local commits.

## Requirements

- Git
- Rust 1.70+
