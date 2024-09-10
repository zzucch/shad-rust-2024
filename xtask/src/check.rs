use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use serde::Deserialize;
use xshell::{cmd, Shell};

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////

const CONFIG_FILE_NAME: &str = ".check.yml";

////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Clone, Debug)]
pub struct CheckArgs {
    #[arg(short, long, help = "Path to the task dir (defaults to CWD)")]
    pub task_path: Option<PathBuf>,
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

fn read_config(path: &Path) -> Result<Config> {
    let mut file = fs::File::open(path).context(format!("failed to open {}", path.display()))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .context(format!("failed to read {}", path.display()))?;

    serde_yaml::from_slice(&buffer).context("failed to parse config")
}

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
    let path = task_path.join(".allowlist");
    ensure!(path.exists(), "file not found: {}", path.display());
    Ok(())
}

pub fn check(args: CheckArgs) -> Result<()> {
    let rel_task_path = args
        .task_path
        .unwrap_or(env::current_dir().context("failed to get cwd")?);
    let task_path = fs::canonicalize(rel_task_path).context("failed to canonicalize task path")?;

    let config = read_config(&task_path.join(CONFIG_FILE_NAME)).context("failed to read config")?;

    let shell = Shell::new().context("failed to create shell")?;
    shell.change_dir(&task_path);

    run_lints(&shell, config.lint).context("lints failed")?;
    run_build(&shell, config.build).context("build failed")?;
    run_tests(&shell, config.test).context("tests failed")?;

    ensure_grader_config_exists(&task_path)?;

    eprintln!("OK!");
    Ok(())
}
