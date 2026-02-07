use clap::CommandFactory;
use clap_mangen::Man;
use std::fs;
use std::path::PathBuf;

#[path = "src/cli.rs"]
mod cli;

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let cmd = cli::Cli::command();
    let man = Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    fs::write(out_dir.join("git-worktree-refresh.1"), buf)?;
    Ok(())
}
