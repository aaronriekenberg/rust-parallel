use anyhow::Context;

use tokio::{io::AsyncBufReadExt, process::Command, task::JoinSet};

#[derive(Debug)]
struct CommandAndExitStatus {
    _command: String,
    _exit_status: Option<std::process::ExitStatus>,
}

async fn run_command(command: String) -> CommandAndExitStatus {
    tracing::info!("begin run_command command = {}", command);

    // tokio::time::sleep(Duration::from_millis(500)).await;

    let command_output = Command::new("/bin/sh")
        .args(["-c", &command])
        .output()
        .await;

    match command_output {
        Ok(output) => {
            tracing::info!("got command status = {}", output.status);
            if output.stdout.len() > 0 {
                tracing::info!(
                    "got command stdout:\n{}",
                    &String::from_utf8_lossy(&output.stdout)
                );
            }
            if output.stderr.len() > 0 {
                tracing::info!(
                    "got command stderr:\n{}",
                    &String::from_utf8_lossy(&output.stderr)
                );
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

    tracing::info!("begin main!");

    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();
    let mut join_set = JoinSet::new();

    loop {
        line.clear();

        let bytes = reader
            .read_line(&mut line)
            .await
            .context("read_line error")?;
        if bytes == 0 {
            break;
        }

        let trimmed_line = line.trim().to_owned();

        tracing::info!("read line {}", trimmed_line);

        join_set.spawn(run_command(trimmed_line));
    }

    tracing::info!("after loop");

    while let Some(res) = join_set.join_next().await {
        tracing::info!("got result {:?}", res);
    }

    Ok(())
}
