use anyhow::Context;

use clap::Parser;

use tokio::{
    io::AsyncBufReadExt,
    process::Command,
    sync::{OnceCell, Semaphore, SemaphorePermit},
    task::JoinSet,
};

use tracing::{debug, warn};

/// Run commands from stdin in parallel
#[derive(Parser, Debug)]
#[command(version, about)]
struct CommandLineArgs {
    /// Maximum number of commands to run in parallel, defauts to num cpus
    #[arg(short, long, default_value_t = num_cpus::get())]
    jobs: usize,

    /// Use /bin/sh shell to run commands, defaults to false
    #[arg(short, long, default_value_t = false)]
    shell_enabled: bool,
}

#[derive(Debug)]
struct CommandInfo {
    _line_number: u64,
    command: String,
    shell_enabled: bool,
}

async fn run_command(_permit: SemaphorePermit<'static>, command_info: CommandInfo) -> CommandInfo {
    let command_output = if command_info.shell_enabled {
        Command::new("/bin/sh")
            .args(["-c", &command_info.command])
            .output()
            .await
    } else {
        let split: Vec<&str> = command_info.command.split_whitespace().collect();
        Command::new(split[0]).args(&split[1..]).output().await
    };

    match command_output {
        Ok(output) => {
            debug!("got command status = {}", output.status);
            if output.stdout.len() > 0 {
                print!("{}", &String::from_utf8_lossy(&output.stdout));
            }
            if output.stderr.len() > 0 {
                eprint!("{}", &String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            warn!("got error running command {:?}: {}", command_info, e);
        }
    };

    command_info
}

static COMMAND_SEMAPHORE: OnceCell<Semaphore> = OnceCell::const_new();

async fn acquire_command_semaphore(
    command_line_args: &CommandLineArgs,
) -> SemaphorePermit<'static> {
    let semaphore = COMMAND_SEMAPHORE
        .get_or_init(|| async { Semaphore::new(command_line_args.jobs) })
        .await;

    semaphore.acquire().await.expect("semaphore.acquire error")
}

async fn spawn_commands(
    command_line_args: &CommandLineArgs,
) -> anyhow::Result<JoinSet<CommandInfo>> {
    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();
    let mut line_number = 0u64;
    let mut join_set = JoinSet::new();

    loop {
        line.clear();

        let bytes_read = reader
            .read_line(&mut line)
            .await
            .context("read_line error")?;
        if bytes_read == 0 {
            break;
        }

        line_number += 1;

        let trimmed_line = line.trim();

        debug!("read line {}", trimmed_line);

        if trimmed_line.is_empty() || trimmed_line.starts_with("#") {
            continue;
        }

        let permit = acquire_command_semaphore(&command_line_args).await;

        join_set.spawn(run_command(
            permit,
            CommandInfo {
                _line_number: line_number,
                command: trimmed_line.to_owned(),
                shell_enabled: command_line_args.shell_enabled,
            },
        ));
    }

    Ok(join_set)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    debug!("begin main");

    let command_line_args = CommandLineArgs::parse();

    debug!("command_line_args = {:?}", command_line_args);

    let mut join_set = spawn_commands(&command_line_args).await?;

    debug!("after spawn_commands join_set.len() = {}", join_set.len());

    while let Some(result) = join_set.join_next().await {
        debug!("join_next result = {:?}", result);
    }

    debug!("end main");

    Ok(())
}
