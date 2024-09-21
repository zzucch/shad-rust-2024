use xtask_util::canonicalize;

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use gix::{progress::prodash::progress, remote::Direction, Repository};
use xshell::{cmd, Shell};

use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////

const STUDENT_GROUP_URL: &str = "https://gitlab.manytask.org/rust-ysda-students-2024-fall";
const STUDENT_REMOTE_NAME: &str = "student";

////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Clone, Debug)]
pub struct SubmitArgs {
    pub task_path: Option<PathBuf>,

    #[arg(short, long, action)]
    pub verbose: bool,
}

////////////////////////////////////////////////////////////////////////////////

fn uncommitted_changes(repo: &Repository, task_name: &str) -> Result<Vec<PathBuf>> {
    let status = repo
        .status(progress::Discard)
        .context("failed to get repository status")?;

    let task_prefix = format!("task/{task_name}/");

    let mut paths = vec![];
    for mb_entry in status.into_index_worktree_iter(None)? {
        let path = String::from_utf8_lossy(mb_entry?.rela_path()).into_owned();
        if path.starts_with(&task_prefix) {
            paths.push(path.into());
        }
    }

    Ok(paths)
}

fn get_student_login(repo: &Repository, remote: &str) -> Result<String> {
    let remote = match repo.find_remote(remote) {
        Ok(remote) => remote,
        Err(err) if matches!(err, gix::remote::find::existing::Error::NotFound { .. }) => {
            bail!(
                "remote '{}' does not exist. Please create it according to the course tutorial.",
                STUDENT_REMOTE_NAME
            );
        }
        Err(err) => bail!("failed to find remote '{}': {}", remote, err),
    };

    let url = remote
        .url(Direction::Push)
        .context("failed to get remote url: not a valid utf-8")?;
    let path = String::from_utf8_lossy(&url.path);
    let (_, tail) = path.rsplit_once("/").context("remote url has no '/'")?;
    Ok(tail.trim_end_matches(".git").to_string())
}

fn push_task(path: &Path, branch: &str, verbose: bool) -> Result<()> {
    // NB: pushing using gix would require dealing with user authentication,
    // which is very difficult to get right.
    // So we give up and use git cli.
    let sh = Shell::new().context("failed to create shell")?;
    sh.change_dir(path);

    let cmd = cmd!(sh, "git push --force {STUDENT_REMOTE_NAME} HEAD:{branch}");

    if verbose {
        return cmd
            .run()
            .with_context(|| format!("failed to push to branch \"{branch}\""));
    }

    let output = cmd.ignore_status().output()?;
    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        bail!("failed to push to branch \"{branch}\"");
    }

    Ok(())
}

pub fn submit(args: SubmitArgs) -> Result<()> {
    let task_path = canonicalize(
        args.task_path
            .unwrap_or(env::current_dir().context("failed to get cwd")?),
    )?;

    ensure!(
        task_path.join(".grade.toml").exists(),
        "not a task directory: {task_path:?}",
    );

    let task_name = task_path
        .file_name()
        .and_then(OsStr::to_str)
        .with_context(|| format!("invalid task path: {task_path:?}"))?
        .to_owned();

    let repo = gix::discover(&task_path).context("failed to discover git repository")?;

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

    eprintln!("Submitting \"{task_name}\" ...");
    push_task(&task_path, "main", args.verbose)?;
    push_task(&task_path, &format!("submit/{task_name}"), args.verbose)?;

    eprintln!("OK: task is successfully submitted.");
    eprintln!("-> {STUDENT_GROUP_URL}/{student_login}/pipelines");
    Ok(())
}
