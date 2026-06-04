use std::io;

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::Shell;

use crate::cli::Cli;

/// Generate shell completion scripts to stdout.
#[allow(clippy::unnecessary_wraps)]
pub fn execute(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "fabio", &mut io::stdout());
    Ok(())
}
