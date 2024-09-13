use crate::util::{canonicalize, read_config};

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use serde::Deserialize;
use xshell::{cmd, Shell};

use std::{
    env,
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////

const CONFIG_FILE_NAME: &str = ".check.toml";

////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Clone, Debug)]
pub struct CheckArgs {
    pub task_path: Vec<PathBuf>,
}

#[derive(Deserialize)]
struct Config {
    lint: LintConfig,
    test: TestConfig,

    #[serde(default)]
    build: BuildConfig,
}

#[derive(Deserialize)]
struct LintConfig {
    #[serde(default)]
    fmt: bool,

    #[serde(default)]
    clippy: bool,

    #[serde(default)]
    allow_unsafe: bool,
}

#[derive(Deserialize, Default)]
struct BuildConfig {
    #[serde(default)]
    debug: bool,

    #[serde(default)]
    release: bool,
}

#[derive(Deserialize)]
struct TestConfig {
    #[serde(default)]
    debug: bool,

    #[serde(default)]
    release: bool,
}

////////////////////////////////////////////////////////////////////////////////

fn run_lints(shell: &Shell, config: LintConfig) -> Result<()> {
    if config.fmt {
        cmd!(shell, "cargo fmt -- --check").run()?;
    }

    if config.clippy {
        let deny_unsafe_code = if config.allow_unsafe {
            &[] as &[_]
        } else {
            &["--deny", "unsafe_code"]
        };
        cmd!(
            shell,
            "cargo clippy -- --deny warnings {deny_unsafe_code...}"
        )
        .run()?;
    } else if !config.allow_unsafe {
        bail!("`lint.allow_unsafe` cannot be false with `lint.clippy` disabled");
    }

    Ok(())
}

fn run_build(shell: &Shell, config: BuildConfig) -> Result<()> {
    if config.debug {
        cmd!(shell, "cargo build").run()?;
    }

    if config.release {
        cmd!(shell, "cargo build --release").run()?;
    }

    Ok(())
}

fn run_tests(shell: &Shell, config: TestConfig) -> Result<()> {
    if config.debug {
        cmd!(shell, "cargo test").run()?;
    }

    if config.release {
        cmd!(shell, "cargo test --release").run()?;
    }

    Ok(())
}

fn ensure_grader_config_exists(task_path: &Path) -> Result<()> {
    let path = task_path.join(".grade.toml");
    ensure!(path.exists(), "file not found: {path:?}");
    Ok(())
}

fn check_task(path: &Path) -> Result<()> {
    let config =
        read_config::<Config>(path.join(CONFIG_FILE_NAME)).context("failed to read config")?;

    let shell = Shell::new().context("failed to create shell")?;
    shell.change_dir(path);

    run_lints(&shell, config.lint).context("lints failed")?;
    run_build(&shell, config.build).context("build failed")?;
    run_tests(&shell, config.test).context("tests failed")?;

    ensure_grader_config_exists(path)
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
        check_task(&task_path).with_context(|| format!("task \"{task_name}\" check failed"))?;
    }

    eprintln!("OK!");
    Ok(())
}
