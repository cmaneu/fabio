#![recursion_limit = "256"]

mod cli;
mod client;
mod commands;
mod errors;
mod output;
mod parallel;

use std::io::Write;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Use try_parse so we can handle missing subcommand gracefully:
    // print help to stdout (composable) instead of stderr (clap default).
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp
                | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                | clap::error::ErrorKind::DisplayVersion => {
                    // Write help/version to stdout so piping works:
                    // `fabio | grep`, `fabio | less`
                    write!(std::io::stdout(), "{e}").ok();
                    std::process::exit(0);
                }
                _ => {
                    e.print().ok();
                    std::process::exit(2);
                }
            }
        }
    };

    let result = Box::pin(commands::execute(cli)).await;

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Some(fabio_err) = e.downcast_ref::<errors::FabioError>() {
                output::render_error(fabio_err);
                std::process::exit(1);
            }
            // Unexpected error - still render as structured JSON
            let fabio_err = errors::FabioError::new(errors::ErrorCode::Unknown, e.to_string());
            output::render_error(&fabio_err);
            std::process::exit(1);
        }
    }
}
