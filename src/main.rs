#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

use tracing::{debug, error, instrument};

use crate::command_line_args::CommandLineArgs;

mod command;
mod command_line_args;
mod common;
mod input;
mod output;
mod parser;
mod process;
mod progress;

#[instrument(skip_all, name = "try_main", level = "debug")]
async fn try_main() -> anyhow::Result<()> {
    debug!("begin try_main");

    let command_line_args = CommandLineArgs::instance().await;

    let command_service = command::CommandService::new(command_line_args);

    command_service.run_commands().await?;

    debug!("end try_main");

    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if let Err(err) = try_main().await {
        error!("fatal error in main:\n{:#}", err);
        std::process::exit(1);
    }
}
