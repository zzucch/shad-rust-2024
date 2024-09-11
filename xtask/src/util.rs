use anyhow::{Context, Result};
use git2::{Repository, RepositoryOpenFlags};
use serde::Deserialize;

use std::{
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////

pub fn canonicalize(path: impl AsRef<Path>) -> Result<PathBuf> {
    std::fs::canonicalize(path.as_ref())
        .with_context(|| format!("failed to canonicalize path {}", path.as_ref().display()))
}

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

pub fn read_config<T>(path: impl AsRef<Path>) -> Result<T>
where
    for<'a> T: Deserialize<'a>,
{
    let path = path.as_ref();
    let mut file =
        std::fs::File::open(path).context(format!("failed to open {}", path.display()))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .context(format!("failed to read {}", path.display()))?;

    serde_yaml::from_slice(&buffer).context("failed to parse config")
}
