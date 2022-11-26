use anyhow::Context;

use clap::Parser;

use tokio::{io::AsyncBufReadExt, process::Command, sync::Semaphore, task::JoinSet};

use tracing::{debug, warn};

use std::sync::Arc;

/// Run commands from stdin in parallel
#[derive(Parser, Debug)]
#[command(version, about)]
struct CommandLineArgs {
    /// Maximum number of commands to run in parallel, defauts to num cpus
    #[arg(short, long, default_value_t = num_cpus::get())]
    jobs: usize,
}

#[derive(Debug)]
struct CommandInfo {
    _line_number: u64,
    command: String,
}

async fn run_command(semaphore: Arc<Semaphore>, command_info: CommandInfo) -> CommandInfo {
    let permit = semaphore.acquire().await.expect("semaphore acquire error");

    let command_output = Command::new("/bin/sh")
        .args(["-c", &command_info.command])
        .output()
        .await;

    drop(permit);

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

async fn spawn_commands(semaphore: Arc<Semaphore>) -> anyhow::Result<JoinSet<CommandInfo>> {
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

        join_set.spawn(run_command(
            Arc::clone(&semaphore),
            CommandInfo {
                _line_number: line_number,
                command: trimmed_line.to_owned(),
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

    let semaphore = Arc::new(Semaphore::new(command_line_args.jobs));

    let mut join_set = spawn_commands(semaphore).await?;

    debug!("after spawn_commands join_set.len() = {}", join_set.len());

    while let Some(result) = join_set.join_next().await {
        debug!("join_next result = {:?}", result);
    }

    debug!("end main");

    Ok(())
}
