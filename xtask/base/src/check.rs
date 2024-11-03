use crate::{
    checker_config::{read_checker_config, BuildConfig, LintConfig, TestConfig},
    util::create_shell,
};

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use walkdir::WalkDir;
use xshell::cmd;
use xtask_util::canonicalize;

use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Clone, Debug)]
pub struct CheckArgs {
    pub task_path: Vec<PathBuf>,
}

fn make_package_args(package: &Option<String>) -> Vec<&str> {
    match package {
        Some(package) => vec!["--package", package],
        None => vec![],
    }
}

fn find_forbidden_ident(
    token_stream: TokenStream,
    forbidden_idents: &HashSet<Ident>,
) -> Option<Ident> {
    for token in token_stream {
        match token {
            TokenTree::Group(group) => {
                if let Some(ident) = find_forbidden_ident(group.stream(), forbidden_idents) {
                    return Some(ident);
                }
            }
            TokenTree::Ident(ident) => {
                if forbidden_idents.contains(&ident) {
                    return Some(ident);
                }
            }
            TokenTree::Punct(_) => continue,
            TokenTree::Literal(_) => continue,
        }
    }
    None
}

fn ensure_no_forbidden_idents(
    task_path: &Path,
    allowlist: &[PathBuf],
    forbidden_idents: &HashSet<Ident>,
) -> Result<()> {
    for entry in allowlist {
        for mb_subentry in WalkDir::new(task_path.join(entry)) {
            let subentry = mb_subentry.with_context(|| format!("failed to traverse {entry:?}"))?;

            let path = subentry.path();
            if path.extension() != Some(OsStr::new("rs")) {
                continue;
            }

            let source =
                fs::read_to_string(path).with_context(|| format!("failed to read {path:?}"))?;
            let Ok(token_stream) = TokenStream::from_str(&source) else {
                bail!("file contains invalid Rust source: {path:?}");
            };
            if let Some(ident) = find_forbidden_ident(token_stream, forbidden_idents) {
                bail!("found forbidden identifier \"{ident}\" in file {path:?}");
            }
        }
    }
    Ok(())
}

fn run_lints(task_path: &Path, config: &LintConfig, allowlist: &[PathBuf]) -> Result<()> {
    let sh = create_shell(task_path)?;

    let package_args = &make_package_args(&config.package);

    if config.fmt {
        cmd!(sh, "cargo fmt {package_args...} -- --check").run()?;
    }

    if config.clippy {
        let mut args = Vec::<&str>::new();

        if !config.allow_unsafe {
            args.extend(&["--deny", "unsafe_code"]);
        }

        if !config.allow_exit {
            args.extend(&["--deny", "clippy::exit"]);
        }

        cmd!(
            sh,
            "cargo clippy {package_args...} -- --deny warnings {args...}"
        )
        .run()?;
    }

    let mut forbidden_idents = HashSet::new();
    if !config.allow_unsafe {
        forbidden_idents.insert(Ident::new("unsafe", Span::call_site()));
    }
    if !config.allow_exit {
        forbidden_idents.insert(Ident::new("exit", Span::call_site()));
    }

    ensure_no_forbidden_idents(task_path, allowlist, &forbidden_idents)
}

fn run_build(task_path: &Path, config: &BuildConfig) -> Result<()> {
    let sh = create_shell(task_path)?;

    let package_args = &make_package_args(&config.package);

    if config.debug {
        cmd!(sh, "cargo build {package_args...}").run()?;
    }

    if config.release {
        cmd!(sh, "cargo build {package_args...} --release").run()?;
    }

    Ok(())
}

fn run_tests(task_path: &Path, config: &TestConfig) -> Result<()> {
    let sh = create_shell(task_path)?;

    let package_args = &make_package_args(&config.package);

    if config.debug {
        cmd!(sh, "cargo test {package_args...}").run()?;
    }

    if config.release {
        cmd!(sh, "cargo test {package_args...} --release").run()?;
    }

    for hook in &config.custom_hooks {
        ensure!(
            !hook.command.is_empty(),
            "test custom hook command cannot be empty",
        );
        sh.cmd(&hook.command[0]).args(&hook.command[1..]).run()?;
    }

    Ok(())
}

fn check_task(path: &Path) -> Result<()> {
    let config = read_checker_config(path).context("failed to read config")?;

    run_lints(path, &config.lint, &config.grade.allowlist)?;
    run_build(path, &config.build)?;
    run_tests(path, &config.test)?;

    Ok(())
}

pub fn check(args: CheckArgs) -> Result<()> {
    let task_paths = if args.task_path.is_empty() {
        vec![env::current_dir().context("failed to get cwd")?]
    } else {
        args.task_path
    }
    .into_iter()
    .map(canonicalize)
    .collect::<Result<Vec<_>>>()?;

    for task_path in task_paths {
        let task_name = task_path
            .file_name()
            .map(|t| t.to_string_lossy().into_owned())
            .with_context(|| format!("invalid task path: {task_path:?}"))?;

        eprintln!("Checking task \"{task_name}\" at {task_path:?}");
        check_task(&task_path)?;
    }

    eprintln!("OK!");
    Ok(())
}
