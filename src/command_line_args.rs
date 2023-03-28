use anyhow::Context;

use clap::Parser;

use tokio::sync::OnceCell;

use tracing::debug;

/// Execute commands in parallel
///
/// By Aaron Riekenberg <aaron.riekenberg@gmail.com>
///
/// https://github.com/aaronriekenberg/rust-parallel
/// https://crates.io/crates/rust-parallel
#[derive(Parser, Debug, Default)]
#[command(verbatim_doc_comment, version)]
pub struct CommandLineArgs {
    /// Input file or - for stdin.  Defaults to stdin if no inputs are specified.
    #[arg(short, long)]
    pub input: Vec<String>,

    /// Maximum number of commands to run in parallel, defauts to num cpus
    #[arg(short, long, default_value_t = num_cpus(), value_parser = clap::value_parser!(u64).range(jobs_range()))]
    pub jobs: u64,

    /// Use null separator for reading input instead of newline.
    #[arg(short('0'), long)]
    pub null_separator: bool,

    /// Use shell for running commands.
    ///
    /// If $SHELL environment variable is set use it else use /bin/bash.
    ///
    /// Each input line is passed to $SHELL -c <line> as a single argument.
    #[arg(short, long)]
    pub shell: bool,

    /// Output channel capacity, defauts to num cpus
    #[arg(long, default_value_t = num_cpus(), value_parser = clap::value_parser!(u64).range(1..))]
    pub output_channel_capacity: u64,

    /// Optional command and initial arguments to run for each input line.
    #[arg(trailing_var_arg(true))]
    pub command_and_initial_arguments: Vec<String>,
}

impl CommandLineArgs {
    pub fn jobs_usize(&self) -> usize {
        self.jobs.try_into().unwrap()
    }

    pub fn output_channel_capacity_usize(&self) -> usize {
        self.output_channel_capacity.try_into().unwrap()
    }
}

fn num_cpus() -> u64 {
    num_cpus::get().try_into().unwrap()
}

fn jobs_range() -> std::ops::RangeInclusive<u64> {
    let max_permits: u64 = tokio::sync::Semaphore::MAX_PERMITS.try_into().unwrap();
    1..=max_permits
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
