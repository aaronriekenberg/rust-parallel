#![warn(rust_2018_idioms)]

use tracing::{debug, error};

mod command;
mod command_line_args;
mod input;
mod output;

async fn try_main() -> anyhow::Result<()> {
    debug!("begin try_main");

    command_line_args::initialize()?;

    let command_service = command::CommandService::new();

    let mut wait_group = command_service.spawn_commands().await?;

    debug!("before wait_group.wait wait_group = {:?}", wait_group);

    wait_group.wait().await;

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
