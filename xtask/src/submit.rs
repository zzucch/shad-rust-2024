use anyhow::{bail, Context, Result};
use clap::Parser;
use git2::{Repository, RepositoryOpenFlags, StatusOptions};

use std::{
    env,
    ffi::OsStr,
    fs, iter,
    path::{Path, PathBuf},
    process::Command,
};

////////////////////////////////////////////////////////////////////////////////

const STUDENTS_GROUP_URL: &str = "https://gitlab.manytask.org/rust-ysda-students-2024-fall";
const REMOTE_NAME: &str = "student";

////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Clone, Debug)]
pub struct SubmitArgs {
    #[arg(short, long, help = "Path to the task dir (defaults to CWD)")]
    pub task_path: Option<PathBuf>,

    #[arg(help = "Subtask name")]
    pub subtask_name: Option<String>,
}

////////////////////////////////////////////////////////////////////////////////

fn is_valid_task_path(path: &Path) -> bool {
    path.join(".allowlist").exists()
}

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

fn push_task(
    path: &Path,
    task_name: &str,
    remote_name: &str,
    subtask_name: Option<&str>,
) -> Result<()> {
    // NB: push using git as a subcommand is way less tedious than using libgit2.
    let branch_name = match subtask_name {
        Some(subtask) => format!("submit/{}@{}", subtask, task_name),
        None => format!("submit/{}", task_name),
    };
    let status = Command::new("git")
        .args(&[
            "push",
            "--force",
            remote_name,
            &format!("HEAD:{}", branch_name),
        ])
        .current_dir(path)
        .spawn()
        .context("failed to call 'git'")?
        .wait()
        .context("failed to wait for 'git'")?;
    if !status.success() {
        bail!("'git push' failed");
    }
    Ok(())
}

pub fn submit(args: SubmitArgs) -> Result<()> {
    let rel_task_path = args
        .task_path
        .unwrap_or(env::current_dir().context("failed to get cwd")?);
    let task_path = fs::canonicalize(rel_task_path).context("failed to canonicalize task path")?;
    if !is_valid_task_path(&task_path) {
        bail!(
            "this doesn't look like a valid task directory: {}",
            task_path.display()
        );
    }

    let task_name = fs::canonicalize(&task_path)
        .context("failed to canonicalize path")?
        .file_name()
        .context("failed to get file name")?
        .to_str()
        .context("task name is not a str")?
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

    push_task(
        &task_path,
        &task_name,
        REMOTE_NAME,
        args.subtask_name.as_deref(),
    )
    .context("failed to push task")?;

    eprintln!("\nOK: task is successfully submitted.");
    eprintln!("-> {}/{}/pipelines", STUDENTS_GROUP_URL, student_login);
    Ok(())
}
