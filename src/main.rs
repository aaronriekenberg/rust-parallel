#![warn(rust_2018_idioms)]

mod command_line_args;
mod commands;

use tracing::debug;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    debug!("begin main");

    command_line_args::initialize()?;

    let command_service = commands::CommandService::new();

    let mut wait_group = command_service.spawn_commands().await?;

    debug!("before wait_group.wait wait_group = {:?}", wait_group);

    wait_group.wait().await;

    debug!("end main");

    Ok(())
}
