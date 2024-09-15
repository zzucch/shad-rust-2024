use anyhow::{Context, Result};
use git2::{Repository, RepositoryOpenFlags};

use std::{ffi::OsStr, path::PathBuf};

////////////////////////////////////////////////////////////////////////////////

pub fn get_cwd_repo_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("failed to get cwd")?;
    let repo = Repository::open_ext(
        &cwd,
        RepositoryOpenFlags::empty(),
        std::iter::empty::<&OsStr>(),
    )
    .context("failed to open git repository")?;
    repo.workdir()
        .map(|p| p.to_path_buf())
        .context("looks like we are in a bare git repo")
}
