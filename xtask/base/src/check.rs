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

fn run_lints(sh: &Shell, config: LintConfig) -> Result<()> {
    if config.fmt {
        cmd!(sh, "cargo fmt -- --check").run()?;
    }

    if config.clippy {
        let deny_unsafe_code = if config.allow_unsafe {
            &[] as &[_]
        } else {
            &["--deny", "unsafe_code"]
        };
        cmd!(sh, "cargo clippy -- --deny warnings {deny_unsafe_code...}").run()?;
    } else if !config.allow_unsafe {
        bail!("`lint.allow_unsafe` cannot be false with `lint.clippy` disabled");
    }

    Ok(())
}

fn run_build(sh: &Shell, config: BuildConfig) -> Result<()> {
    if config.debug {
        cmd!(sh, "cargo build").run()?;
    }

    if config.release {
        cmd!(sh, "cargo build --release").run()?;
    }

    Ok(())
}

fn run_tests(sh: &Shell, config: TestConfig) -> Result<()> {
    if config.debug {
        cmd!(sh, "cargo test").run()?;
    }

    if config.release {
        cmd!(sh, "cargo test --release").run()?;
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

    let sh = Shell::new().context("failed to create shell")?;
    sh.change_dir(path);

    run_lints(&sh, config.lint)?;
    run_build(&sh, config.build)?;
    run_tests(&sh, config.test)?;

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
        check_task(&task_path)?;
    }

    eprintln!("OK!");
    Ok(())
}
