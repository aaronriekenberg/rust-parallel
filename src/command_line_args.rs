use anyhow::Context;

use clap::Parser;

use getset::Getters;

use tokio::sync::OnceCell;

use tracing::debug;

fn default_jobs() -> u32 {
    num_cpus::get().try_into().unwrap()
}

#[derive(Parser, Debug, Getters)]
#[command(version, about)]
#[getset(get = "pub")]
pub struct CommandLineArgs {
    /// Maximum number of commands to run in parallel, defauts to num cpus
    #[arg(short, long, default_value_t = default_jobs(), value_parser = clap::value_parser!(u32).range(1..))]
    jobs: u32,

    /// Input file or - for stdin.  Defaults to stdin if no inputs are specified.
    #[arg(short, long)]
    inputs: Vec<String>,

    /// Optional command and initial arguments to run for each input line.
    #[arg(trailing_var_arg(true))]
    command_and_initial_arguments: Vec<String>,
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
