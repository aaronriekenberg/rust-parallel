use anyhow::Context;

use tokio::{io::AsyncBufReadExt, process::Command, task::JoinSet};

use tracing::debug;

#[derive(Debug)]
struct CommandAndExitStatus {
    _command: String,
    _exit_status: Option<std::process::ExitStatus>,
}

async fn run_command(command: String) -> CommandAndExitStatus {
    let command_output = Command::new("/bin/sh")
        .args(["-c", &command])
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

            CommandAndExitStatus {
                _command: command,
                _exit_status: Some(output.status),
            }
        }
        Err(e) => {
            tracing::warn!("got error running command '{}': {}", command, e);
            CommandAndExitStatus {
                _command: command,
                _exit_status: None,
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    debug!("begin main!");

    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();
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

        let trimmed_line = line.trim().to_owned();

        debug!("read line {}", trimmed_line);

        if trimmed_line.is_empty() || trimmed_line.starts_with("#") {
            continue;
        }

        join_set.spawn(run_command(trimmed_line));
    }

    debug!("after loop join_set.len() = {}", join_set.len());

    while let Some(res) = join_set.join_next().await {
        debug!("got result {:?}", res);
    }

    Ok(())
}
