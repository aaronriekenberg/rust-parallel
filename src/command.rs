use anyhow::Context;

use tokio::{process::Command as TokioCommand, sync::Semaphore};

use tracing::{debug, instrument, warn};

use std::sync::Arc;

use crate::{
    command_line_args,
    command_line_args::CommandLineArgs,
    input::{build_input_list, Input, InputLineNumber, InputReader},
    output::OutputWriter,
    parser::{CommandAndArgs, InputLineParser},
};

#[derive(Debug)]
struct Command {
    command_and_args: CommandAndArgs,
    input_line_number: InputLineNumber,
}

impl Command {
    #[instrument(skip_all, fields(command = %self), level = "debug")]
    async fn run_command(self, output_writer: Arc<OutputWriter>) {
        debug!("begin run_command");

        let [command, args @ ..] = self.command_and_args.as_slice() else {
            return;
        };

        let command_output = TokioCommand::new(command).args(args).output().await;

        match command_output {
            Err(e) => {
                warn!("error running command {}: {}", self, e);
            }
            Ok(output) => {
                debug!("command status = {}", output.status);
                output_writer.write_command_output(&output).await;
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
            self.command_and_args, self.input_line_number,
        )
    }
}

pub struct CommandService {
    command_line_args: &'static CommandLineArgs,
    input_line_parser: InputLineParser,
    command_semaphore: Arc<Semaphore>,
    output_writer: Arc<OutputWriter>,
}

impl CommandService {
    pub fn new() -> Self {
        let command_line_args = command_line_args::instance();
        let semaphore_permits: usize = command_line_args.jobs.try_into().unwrap();
        Self {
            command_line_args,
            input_line_parser: InputLineParser::new(command_line_args),
            command_semaphore: Arc::new(Semaphore::new(semaphore_permits)),
            output_writer: OutputWriter::new(),
        }
    }

    async fn spawn_command(
        &self,
        command_and_args: CommandAndArgs,
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
            command.run_command(output_writer_clone).await;

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
            let Ok(input_line) = String::from_utf8(segment) else {
                continue;
            };

            if let Some(command_and_args) = self.input_line_parser.parse_line(input_line) {
                self.spawn_command(command_and_args, input_line_number)
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
