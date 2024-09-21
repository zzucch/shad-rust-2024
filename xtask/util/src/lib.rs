use anyhow::{Context, Result};
use serde::Deserialize;

use std::{
    io::Read,
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////

pub fn canonicalize(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    std::fs::canonicalize(path).with_context(|| format!("failed to canonicalize path {path:?}"))
}

pub fn get_cwd_repo_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("failed to get cwd")?;
    let repo = gix::discover(cwd).context("failed to discover git repository")?;
    repo.work_dir()
        .map(|p| p.to_path_buf())
        .context("looks like we are in a bare git repo")
}

pub fn read_config<T>(path: impl AsRef<Path>) -> Result<T>
where
    for<'a> T: Deserialize<'a>,
{
    let path = path.as_ref();
    let mut file = std::fs::File::open(path).with_context(|| format!("failed to open {path:?}"))?;

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .with_context(|| format!("failed to read {path:?}"))?;

    toml::from_str(&buffer).context("failed to parse config")
}
