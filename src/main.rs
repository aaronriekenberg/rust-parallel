use anyhow::Context;

use tokio::{io::AsyncBufReadExt, process::Command, task::JoinSet};

use tracing::debug;

#[derive(Debug)]
struct CommandInfo {
    _line_number: u64,
    command: String,
}

async fn run_command(command_info: CommandInfo) -> CommandInfo {
    let command_output = Command::new("/bin/sh")
        .args(["-c", &command_info.command])
        .output()
        .await;

    match command_output {
        Ok(output) => {
            tracing::debug!("got command status = {}", output.status);
            if output.stdout.len() > 0 {
                print!("{}", &String::from_utf8_lossy(&output.stdout));
            }
            if output.stderr.len() > 0 {
                eprint!("{}", &String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            tracing::warn!("got error running command '{:?}': {}", command_info, e);
        }
    };

    command_info
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    debug!("begin main!");

    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();
    let mut join_set = JoinSet::new();
    let mut line_number = 0u64;

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

        let trimmed_line = line.trim().to_owned();

        debug!("read line {}", trimmed_line);

        if trimmed_line.is_empty() || trimmed_line.starts_with("#") {
            continue;
        }

        join_set.spawn(run_command(CommandInfo {
            _line_number: line_number,
            command: trimmed_line,
        }));
    }

    debug!("after loop join_set.len() = {}", join_set.len());

    while let Some(result) = join_set.join_next().await {
        debug!("join_next result = {:?}", result);
    }

    Ok(())
}
