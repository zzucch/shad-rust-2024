use xtask_util::read_config;

use anyhow::Result;
use serde::Deserialize;

use std::path::{Path, PathBuf};

////////////////////////////////////////////////////////////////////////////////

const CHECKER_CONFIG_FILE_NAME: &str = ".check.toml";

#[derive(Deserialize)]
pub struct CheckerConfig {
    pub lint: LintConfig,
    pub test: TestConfig,
    pub grade: GradeConfig,

    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Deserialize)]
pub struct LintConfig {
    pub package: Option<String>,

    #[serde(default)]
    pub fmt: bool,

    #[serde(default)]
    pub clippy: bool,

    #[serde(default)]
    pub allow_unsafe: bool,

    #[serde(default)]
    pub allow_exit: bool,
}

#[derive(Deserialize, Default)]
pub struct BuildConfig {
    pub package: Option<String>,

    #[serde(default)]
    pub debug: bool,

    #[serde(default)]
    pub release: bool,
}

#[derive(Deserialize)]
pub struct CustomHookConfig {
    pub command: Vec<String>,
}

#[derive(Deserialize)]
pub struct TestConfig {
    pub package: Option<String>,

    #[serde(default)]
    pub debug: bool,

    #[serde(default)]
    pub release: bool,

    #[serde(default)]
    pub custom_hooks: Vec<CustomHookConfig>,
}

#[derive(Deserialize)]
pub struct GradeConfig {
    pub allowlist: Vec<PathBuf>,
}

pub fn read_checker_config(task_path: impl AsRef<Path>) -> Result<CheckerConfig> {
    let config_path = task_path.as_ref().join(CHECKER_CONFIG_FILE_NAME);
    read_config(config_path)
}
