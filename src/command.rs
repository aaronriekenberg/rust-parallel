use anyhow::Context;

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command as TokioCommand,
    sync::Semaphore,
};

use tracing::{debug, warn};

use std::sync::Arc;

use crate::{
    command_line_args,
    command_line_args::CommandLineArgs,
    input::{Input, InputLineNumber},
    output::OutputWriter,
};

#[derive(Debug)]
struct Command {
    input_line_number: InputLineNumber,
    command: String,
    shell_enabled: bool,
}

impl Command {
    async fn run(self, output_writer: Arc<OutputWriter>) {
        debug!("begin run command = {:?}", self);

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
                warn!("got error running command: {}: {}", self, e);
            }
        };

        debug!("end run command = {:?}", self);
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "'{}' [line={},shell={}]",
            self.command, self.input_line_number, self.shell_enabled,
        )
    }
}

pub struct CommandService {
    command_line_args: &'static CommandLineArgs,
    command_semaphore: Arc<Semaphore>,
    output_writer: Arc<OutputWriter>,
}

impl CommandService {
    pub fn new() -> Self {
        let command_line_args = command_line_args::instance();
        Self {
            command_line_args,
            command_semaphore: Arc::new(Semaphore::new(*command_line_args.jobs())),
            output_writer: OutputWriter::new(),
        }
    }

    async fn spawn_command(
        &self,
        trimmed_line: &str,
        input_line_number: InputLineNumber,
    ) -> anyhow::Result<()> {
        let permit = Arc::clone(&self.command_semaphore)
            .acquire_owned()
            .await
            .context("command_semaphore.acquire_owned error")?;

        let output_writer_clone = Arc::clone(&self.output_writer);

        let command = Command {
            input_line_number,
            command: trimmed_line.to_owned(),
            shell_enabled: *self.command_line_args.shell_enabled(),
        };

        tokio::spawn(async move {
            command.run(output_writer_clone).await;

            drop(permit);
        });

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

            let trimmed_line = line.trim();

            debug!("trimmed_line {}", trimmed_line);

            if trimmed_line.is_empty() || trimmed_line.starts_with("#") {
                continue;
            }

            self.spawn_command(&trimmed_line, InputLineNumber { input, line_number })
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
        if self.command_line_args.inputs().is_empty() {
            vec![Input::Stdin]
        } else {
            self.command_line_args
                .inputs()
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

        debug!(
            "before acquire_many command_semaphore = {:?}",
            self.command_semaphore
        );

        // At this point all commands have been spawned.
        // When all semaphore permits can be acquired
        // we know all commands have completed.
        let _ = self
            .command_semaphore
            .acquire_many(*self.command_line_args.jobs() as u32)
            .await
            .context("command_semaphore.acquire_many error")?;

        debug!("end run_commands");

        Ok(())
    }
}
