use anyhow::Context;

use clap::Parser;

use getset::Getters;

use tokio::sync::OnceCell;

use tracing::debug;

#[derive(Parser, Debug, Getters)]
#[command(version, about)]
#[getset(get = "pub")]
pub struct CommandLineArgs {
    /// Maximum number of commands to run in parallel, defauts to num cpus
    #[arg(short, long, default_value_t = num_cpus::get())]
    jobs: usize,

    /// Use /bin/sh -c shell to run commands
    #[arg(short, long)]
    shell_enabled: bool,

    /// Input file or - for stdin.  Defaults to stdin if no inputs are specified.
    inputs: Vec<String>,
}

static INSTANCE: OnceCell<CommandLineArgs> = OnceCell::const_new();

pub fn initialize() -> anyhow::Result<()> {
    let command_line_args = CommandLineArgs::parse();

    debug!("command_line_args = {:?}", command_line_args);

    INSTANCE
        .set(command_line_args)
        .context("INSTANCE.set error")?;

    Ok(())
}

pub fn instance() -> &'static CommandLineArgs {
    INSTANCE.get().unwrap()
}
