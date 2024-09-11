use crate::util::canonicalize;

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use git2::{Repository, RepositoryOpenFlags, StatusOptions};
use xshell::{cmd, Shell};

use std::{
    env,
    ffi::OsStr,
    iter,
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////

const STUDENTS_GROUP_URL: &str = "https://gitlab.manytask.org/rust-ysda-students-2024-fall";
const REMOTE_NAME: &str = "student";

////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Clone, Debug)]
pub struct SubmitArgs {
    pub task_path: Option<PathBuf>,
}

////////////////////////////////////////////////////////////////////////////////

fn uncommitted_changes(repo: &Repository, task_name: &str) -> Result<Vec<PathBuf>> {
    let statuses = repo
        .statuses(Some(
            &mut StatusOptions::new()
                .include_untracked(false)
                .include_ignored(false),
        ))
        .with_context(|| format!("failed to get git statuses in {:?}", task_name))?;

    let task_prefix = format!("{}/", task_name);

    let mut paths = vec![];
    for status in statuses.iter() {
        let path = PathBuf::from(
            status
                .path()
                .context("'git diff' contains an entry with non-utf8 path")?,
        );
        if path.starts_with(&task_prefix) {
            paths.push(path.into());
        }
    }

    Ok(paths)
}

fn get_student_login(repo: &Repository, remote: &str) -> Result<String> {
    let remote = match repo.find_remote(remote) {
        Ok(remote) => remote,
        Err(err) => {
            if err.code() == git2::ErrorCode::NotFound {
                bail!(
                    "remote '{}' does not exist. Please create it according to the course tutorial.",
                    REMOTE_NAME
                );
            } else {
                bail!("failed to find remote '{}': {}", remote, err);
            }
        }
    };

    let url = remote
        .url()
        .context("failed to get remote url: not a valid utf-8")?;
    let (_, tail) = url.rsplit_once("/").context("remote url has no '/'")?;
    Ok(tail.trim_end_matches(".git").to_string())
}

fn push_task(path: &Path, task_name: &str, remote_name: &str) -> Result<()> {
    // NB: push using git cli is way less tedious than using libgit2.
    let shell = Shell::new().context("failed to create shell")?;
    shell.change_dir(path);
    cmd!(
        shell,
        "git push --force {remote_name} HEAD:submit/{task_name}"
    )
    .run()?;

    Ok(())
}

pub fn submit(args: SubmitArgs) -> Result<()> {
    let task_path = canonicalize(
        args.task_path
            .unwrap_or(env::current_dir().context("failed to get cwd")?),
    )?;

    ensure!(
        task_path.join(".grade.yml").exists(),
        "not a task directory: {}",
        task_path.display(),
    );

    let task_name = canonicalize(&task_path)?
        .file_name()
        .and_then(OsStr::to_str)
        .with_context(|| format!("invalid task path: {}", task_path.display()))?
        .to_owned();

    let repo = Repository::open_ext(
        &task_path,
        RepositoryOpenFlags::empty(),
        iter::empty::<&OsStr>(),
    )
    .context("failed to open git repository")?;

    let uncommitted_files = uncommitted_changes(&repo, &task_name)
        .context("failed to check for uncommitted changes")?;
    if !uncommitted_files.is_empty() {
        bail!(
            "there are uncommitted changes:\n{}\nPlease either commit or stash them.",
            uncommitted_files
                .iter()
                .map(|p| format!(" * {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }

    let student_login = get_student_login(&repo, REMOTE_NAME)?;

    push_task(&task_path, &task_name, REMOTE_NAME).context("failed to push task")?;

    eprintln!("\nOK: task is successfully submitted.");
    eprintln!("-> {}/{}/pipelines", STUDENTS_GROUP_URL, student_login);
    Ok(())
}
