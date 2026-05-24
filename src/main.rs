#![recursion_limit = "256"]

mod cli;
mod client;
mod commands;
mod errors;
mod output;
mod parallel;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
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
