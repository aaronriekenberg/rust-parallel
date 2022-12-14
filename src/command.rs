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
    command_and_args: Vec<String>,
    input_line_number: InputLineNumber,
}

impl Command {
    async fn run(self, output_writer: Arc<OutputWriter>) {
        debug!("begin run command = {:?}", self);

        let [command, args @ ..] = self.command_and_args.as_slice() else {
                panic!("invalid command_and_args '{:?}'", self.command_and_args);
            };

        let command_output = TokioCommand::new(command).args(args).output().await;

        match command_output {
            Err(e) => {
                warn!("got error running command: {}: {}", self, e);
            }
            Ok(output) => {
                debug!("got command status = {}", output.status);
                output_writer.write_command_output(&output).await;
            }
        };

        debug!("end run command = {:?}", self);
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "command_and_args={:?},input_line_number={}",
            self.command_and_args, self.input_line_number,
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
        let jobs: usize = command_line_args.jobs.try_into().unwrap();
        Self {
            command_line_args,
            command_semaphore: Arc::new(Semaphore::new(jobs)),
            output_writer: OutputWriter::new(),
        }
    }

    async fn spawn_command(
        &self,
        command_and_args: Vec<String>,
        input_line_number: InputLineNumber,
    ) -> anyhow::Result<()> {
        let permit = Arc::clone(&self.command_semaphore)
            .acquire_owned()
            .await
            .context("command_semaphore.acquire_owned error")?;

        let output_writer_clone = Arc::clone(&self.output_writer);

        let command = Command {
            input_line_number,
            command_and_args,
        };

        tokio::spawn(async move {
            command.run(output_writer_clone).await;

            drop(permit);
        });

        Ok(())
    }

    fn build_command_and_args(&self, line: &str) -> Vec<String> {
        let mut command_and_args: Vec<String> = if self.command_line_args.null_separator {
            vec![line.to_owned()]
        } else {
            line.split_whitespace().map(|s| s.to_owned()).collect()
        };

        let command_and_initial_arguments = &self.command_line_args.command_and_initial_arguments;

        if command_and_initial_arguments.len() > 0 {
            command_and_args = [command_and_initial_arguments.clone(), command_and_args].concat();
        }

        command_and_args
    }

    async fn process_one_input(
        &self,
        input: Input,
        input_reader: BufReader<impl AsyncRead + Unpin>,
    ) -> anyhow::Result<()> {
        debug!("begin process_one_input input = {:?}", input);

        let line_separator = if self.command_line_args.null_separator {
            0u8
        } else {
            b'\n'
        };

        let mut segments = input_reader.split(line_separator);
        let mut line_number = 0u64;

        while let Some(segment) = segments
            .next_segment()
            .await
            .context("next_segment error")?
        {
            let line = String::from_utf8_lossy(&segment);

            line_number += 1;

            let command_and_args = self.build_command_and_args(&line);

            if command_and_args.is_empty() {
                continue;
            }

            self.spawn_command(command_and_args, InputLineNumber { input, line_number })
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
        if self.command_line_args.input.is_empty() {
            vec![Input::Stdin]
        } else {
            self.command_line_args
                .input
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

        // At this point all commands have been spawned.
        // When all semaphore permits can be acquired
        // we know all commands have completed.
        debug!(
            "before acquire_many command_semaphore = {:?}",
            self.command_semaphore,
        );

        let _ = self
            .command_semaphore
            .acquire_many(self.command_line_args.jobs)
            .await
            .context("command_semaphore.acquire_many error")?;

        debug!("end run_commands");

        Ok(())
    }
}
