use anyhow::{Context, Result};

use std::path::PathBuf;

////////////////////////////////////////////////////////////////////////////////

pub fn get_cwd_repo_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("failed to get cwd")?;
    let repo = gix::discover(cwd).context("failed to discover git repository")?;
    repo.work_dir()
        .map(|p| p.to_path_buf())
        .context("looks like we are in a bare git repo")
}
