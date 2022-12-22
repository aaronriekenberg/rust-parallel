use anyhow::Context;

use awaitgroup::WaitGroup;

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command as TokioCommand,
    sync::{OwnedSemaphorePermit, Semaphore},
};

use tracing::{debug, warn};

use std::sync::Arc;

use crate::{command_line_args, input::Input, output::OutputWriter};

#[derive(Debug)]
struct Command {
    input: Input,
    line_number: u64,
    command: String,
    shell_enabled: bool,
}

impl Command {
    async fn run(
        self,
        worker: awaitgroup::Worker,
        permit: OwnedSemaphorePermit,
        output_writer: Arc<OutputWriter>,
    ) {
        debug!(
            "begin run command = {:?} worker = {:?} permit = {:?}",
            self, worker, permit
        );

        let command_output = if self.shell_enabled {
            TokioCommand::new("/bin/sh")
                .args(["-c", &self.command])
                .output()
                .await
        } else {
            let split: Vec<_> = self.command.split_whitespace().collect();

            let [command, args @ ..] = split.as_slice() else {
                panic!("invalid command '{}'", self.command);
            };

            TokioCommand::new(command).args(args).output().await
        };

        match command_output {
            Ok(output) => {
                debug!("got command status = {}", output.status);
                output_writer.write_command_output(&output).await;
            }
            Err(e) => {
                warn!("got error running command ({}): {}", self, e);
            }
        };

        debug!(
            "end run command = {:?} worker = {:?} permit = {:?}",
            self, worker, permit
        );
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "input={},line_number={},command='{}',shell_enabled={}",
            self.input, self.line_number, self.command, self.shell_enabled,
        )
    }
}

pub struct CommandService {
    command_semaphore: Arc<Semaphore>,
    wait_group: WaitGroup,
    output_writer: Arc<OutputWriter>,
}

impl CommandService {
    pub fn new() -> Self {
        Self {
            command_semaphore: Arc::new(Semaphore::new(*command_line_args::instance().jobs())),
            wait_group: WaitGroup::new(),
            output_writer: OutputWriter::new(),
        }
    }

    async fn process_one_input_line(
        &self,
        input: Input,
        line: &str,
        line_number: u64,
    ) -> anyhow::Result<()> {
        let trimmed_line = line.trim();

        debug!("process_one_input_line {}", trimmed_line);

        if trimmed_line.is_empty() || trimmed_line.starts_with("#") {
            return Ok(());
        }

        let args = command_line_args::instance();

        let permit = Arc::clone(&self.command_semaphore)
            .acquire_owned()
            .await
            .context("command_semaphore.acquire_owned error")?;

        let command = Command {
            input,
            line_number,
            command: trimmed_line.to_owned(),
            shell_enabled: *args.shell_enabled(),
        };

        tokio::spawn(command.run(
            self.wait_group.worker(),
            permit,
            Arc::clone(&self.output_writer),
        ));

        Ok(())
    }

    async fn process_one_input(
        &self,
        input: Input,
        mut input_reader: BufReader<impl AsyncRead + Unpin>,
    ) -> anyhow::Result<()> {
        debug!("begin process_one_input input = {:?}", input);

        let mut line = String::new();
        let mut line_number = 0u64;

        loop {
            line.clear();

            let bytes_read = input_reader
                .read_line(&mut line)
                .await
                .context("read_line error")?;
            if bytes_read == 0 {
                break;
            }

            line_number += 1;

            self.process_one_input_line(input, &line, line_number)
                .await?;
        }

        debug!("end process_one_input input = {:?}", input);

        Ok(())
    }

    async fn process_inputs(&self, inputs: Vec<Input>) -> anyhow::Result<()> {
        for input in inputs {
            match input {
                Input::Stdin => {
                    let input_reader = BufReader::new(tokio::io::stdin());

                    self.process_one_input(input, input_reader).await?;
                }
                Input::File { file_name } => {
                    let file = tokio::fs::File::open(file_name).await.with_context(|| {
                        format!("error opening input file file_name = '{}'", file_name)
                    })?;
                    let input_reader = BufReader::new(file);

                    self.process_one_input(input, input_reader).await?;
                }
            }
        }
        Ok(())
    }

    fn build_inputs(&self) -> Vec<Input> {
        let args = command_line_args::instance();

        if args.inputs().is_empty() {
            vec![Input::Stdin]
        } else {
            args.inputs()
                .iter()
                .map(|input_name| {
                    if input_name == "-" {
                        Input::Stdin
                    } else {
                        Input::File {
                            file_name: input_name,
                        }
                    }
                })
                .collect()
        }
    }

    pub async fn run_commands(self) -> anyhow::Result<()> {
        debug!("begin run_commands");

        let inputs = self.build_inputs();

        self.process_inputs(inputs).await?;

        debug!("before wait_group.wait wait_group = {:?}", self.wait_group);

        let mut mut_self = self;

        mut_self.wait_group.wait().await;

        debug!("end run_commands");

        Ok(())
    }
}
