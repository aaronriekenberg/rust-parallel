use anyhow::Context;

use awaitgroup::WaitGroup;

use tokio::{
    io::{AsyncBufReadExt, AsyncRead},
    process::Command,
    sync::{OwnedSemaphorePermit, Semaphore},
};

use tracing::{debug, warn};

use std::sync::Arc;

use crate::command_line_args;

#[derive(Debug)]
struct CommandInfo {
    _input_name: String,
    _line_number: u64,
    command: String,
    shell_enabled: bool,
}

impl CommandInfo {
    async fn run(self, _permit: OwnedSemaphorePermit, _worker: awaitgroup::Worker) {
        debug!("begin run_command command_info = {:?}", self);

        let command_output = if self.shell_enabled {
            Command::new("/bin/sh")
                .args(["-c", &self.command])
                .output()
                .await
        } else {
            let split: Vec<&str> = self.command.split_whitespace().collect();

            let command = split.get(0).unwrap_or(&"");
            let args = &split[1..];

            Command::new(command).args(args).output().await
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
                warn!("got error running command {:?}: {}", self, e);
            }
        };

        debug!("end run_command command_info = {:?}", self);
    }
}

pub struct CommandService {
    command_semaphore: Arc<Semaphore>,
    wait_group: WaitGroup,
}

impl CommandService {
    pub fn new() -> Self {
        Self {
            command_semaphore: Arc::new(Semaphore::new(*command_line_args::instance().jobs())),
            wait_group: WaitGroup::new(),
        }
    }

    async fn process_one_input(
        &self,
        input_name: &str,
        mut reader: tokio::io::BufReader<impl AsyncRead + Unpin>,
    ) -> anyhow::Result<()> {
        debug!("begin process_one_input input_name = '{}'", input_name);

        let args = command_line_args::instance();

        let mut line = String::new();
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

            let trimmed_line = line.trim();

            debug!("read line {}", trimmed_line);

            if trimmed_line.is_empty() || trimmed_line.starts_with("#") {
                continue;
            }

            let permit = Arc::clone(&self.command_semaphore)
                .acquire_owned()
                .await
                .context("command_semaphore.acquire_owned error")?;

            let worker = self.wait_group.worker();

            let command_info = CommandInfo {
                _input_name: input_name.to_owned(),
                _line_number: line_number,
                command: trimmed_line.to_owned(),
                shell_enabled: *args.shell_enabled(),
            };

            tokio::spawn(command_info.run(permit, worker));
        }

        debug!("end process_one_input input_name = '{}'", input_name);

        Ok(())
    }


    pub async fn spawn_commands(self) -> anyhow::Result<WaitGroup> {
        const STDIN_INPUT: &'static str = "-";

        debug!("begin spawn_commands");

        let args = command_line_args::instance();

        let inputs = if args.inputs().is_empty() {
            vec![STDIN_INPUT.to_owned()]
        } else {
            args.inputs().clone()
        };

        for input_name in &inputs {
            if input_name == STDIN_INPUT {
                let reader = tokio::io::BufReader::new(tokio::io::stdin());

                self.process_one_input(&input_name, reader).await?;
            } else {
                let file = tokio::fs::File::open(input_name).await.with_context(|| {
                    format!("error opening input file input_name = '{}'", input_name)
                })?;
                let reader = tokio::io::BufReader::new(file);

                self.process_one_input(&input_name, reader).await?;
            }
        }

        Ok(self.wait_group)
    }
}
