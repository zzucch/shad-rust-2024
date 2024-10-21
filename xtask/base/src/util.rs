use anyhow::{Context, Result};
use xshell::Shell;

use std::path::Path;

pub fn create_shell(path: &Path) -> Result<Shell> {
    let sh = Shell::new().context("failed to create shell")?;
    sh.change_dir(path);
    Ok(sh)
}
