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

const STUDENT_GROUP_URL: &str = "https://gitlab.manytask.org/rust-ysda-students-2024-fall";
const STUDENT_REMOTE_NAME: &str = "student";

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
                    STUDENT_REMOTE_NAME
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

fn push_task(path: &Path, branch: &str) -> Result<()> {
    // NB: pushing using libgit2 would require dealing with user authentication,
    // which is very difficult to get right.
    // So we give up and use git cli.
    let shell = Shell::new().context("failed to create shell")?;
    shell.change_dir(path);

    let output = cmd!(
        shell,
        "git push --force {STUDENT_REMOTE_NAME} HEAD:{branch}"
    )
    .ignore_status()
    .output()?;

    if !output.status.success() {
        eprintln!(
            "{}",
            std::str::from_utf8(&output.stderr)
                .context("'git push' stderr is not a valid utf-8")?
        );
        bail!("failed to push to branch '{branch}'");
    }

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

    let task_name = task_path
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

    ensure!(
        uncommitted_files.is_empty(),
        "there are uncommitted changes:\n{}\nPlease either commit or stash them.",
        uncommitted_files
            .iter()
            .map(|p| format!(" * {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let student_login = get_student_login(&repo, STUDENT_REMOTE_NAME)?;

    eprintln!("Submitting '{task_name}' ...");
    push_task(&task_path, "main")?;
    push_task(&task_path, &format!("submit/{task_name}"))?;

    eprintln!("OK: task is successfully submitted.");
    eprintln!("-> {STUDENT_GROUP_URL}/{student_login}/pipelines");
    Ok(())
}
