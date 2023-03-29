use anyhow::Context;

use tokio::{process::Command as TokioCommand, sync::Semaphore};

use tracing::{debug, instrument, warn};

use std::sync::Arc;

use crate::{
    command_line_args,
    input::{build_input_list, Input, InputLineNumber, InputReader},
    output::{OutputSender, OutputWriter},
    parser::InputLineParser,
};

#[derive(Debug)]
struct OwnedCommandAndArgs(Vec<String>);

impl From<Vec<&str>> for OwnedCommandAndArgs {
    fn from(v: Vec<&str>) -> OwnedCommandAndArgs {
        OwnedCommandAndArgs(v.into_iter().map(|s| s.to_owned()).collect())
    }
}

#[derive(Debug)]
struct Command {
    command_and_args: OwnedCommandAndArgs,
    input_line_number: InputLineNumber,
}

impl Command {
    #[instrument(skip_all, fields(command = %self), level = "debug")]
    async fn run_command(self, output_sender: OutputSender) {
        debug!("begin run_command");

        let [command, args @ ..] = self.command_and_args.0.as_slice() else {
            return;
        };

        let command_output = TokioCommand::new(command).args(args).output().await;

        match command_output {
            Err(e) => {
                warn!("error running command {}: {}", self, e);
            }
            Ok(output) => {
                debug!("command status = {}", output.status);
                output_sender.send(output).await;
            }
        };

        debug!("end run_command");
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "command_and_args={:?},input_line_number={}",
            self.command_and_args.0, self.input_line_number,
        )
    }
}

pub struct CommandService {
    input_line_parser: InputLineParser,
    command_semaphore: Arc<Semaphore>,
    output_writer: OutputWriter,
}

impl CommandService {
    pub fn new() -> Self {
        let command_line_args = command_line_args::instance();

        Self {
            input_line_parser: InputLineParser::new(command_line_args),
            command_semaphore: Arc::new(Semaphore::new(command_line_args.jobs)),
            output_writer: OutputWriter::new(),
        }
    }

    async fn spawn_command(
        &self,
        command_and_args: OwnedCommandAndArgs,
        input_line_number: InputLineNumber,
    ) -> anyhow::Result<()> {
        let command = Command {
            input_line_number,
            command_and_args,
        };

        let output_sender = self.output_writer.sender();

        let permit = Arc::clone(&self.command_semaphore)
            .acquire_owned()
            .await
            .context("command_semaphore.acquire_owned error")?;

        tokio::spawn(async move {
            command.run_command(output_sender).await;

            drop(permit);
        });

        Ok(())
    }

    #[instrument(skip_all, fields(input = %input), level = "debug")]
    async fn process_one_input(&self, input: Input) -> anyhow::Result<()> {
        debug!("begin process_one_input");

        let mut input_reader = InputReader::new(input).await?;

        while let Some((input_line_number, segment)) = input_reader
            .next_segment()
            .await
            .context("next_segment error")?
        {
            let Ok(input_line) = std::str::from_utf8(&segment) else {
                continue;
            };

            if let Some(command_and_args) = self.input_line_parser.parse_line(input_line) {
                self.spawn_command(command_and_args.into(), input_line_number)
                    .await?;
            }
        }

        debug!("end process_one_input");

        Ok(())
    }

    async fn process_inputs(&self) -> anyhow::Result<()> {
        for input in build_input_list() {
            self.process_one_input(input).await?;
        }
        Ok(())
    }

    #[instrument(skip_all, level = "debug")]
    pub async fn run_commands(self) -> anyhow::Result<()> {
        debug!("begin run_commands");

        self.process_inputs().await?;

        debug!("before output_writer.wait_for_completion",);

        self.output_writer.wait_for_completion().await;

        debug!("end run_commands");

        Ok(())
    }
}
