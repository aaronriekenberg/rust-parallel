use anyhow::Context;

use tokio::{process::Command as TokioCommand, sync::Semaphore};

use tracing::{debug, instrument, span_enabled, warn, Level, Span};

use std::{
    process::{Output, Stdio},
    sync::Arc,
};

use crate::{
    command_line_args::{self, CommandLineArgs, DiscardOutputMode},
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

impl std::fmt::Display for OwnedCommandAndArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug)]
struct CommandOutputMode {
    discard_stdout: bool,
    discard_stderr: bool,
}

impl CommandOutputMode {
    fn new(command_line_args: &CommandLineArgs) -> Self {
        Self {
            discard_stdout: match command_line_args.discard_output_mode {
                DiscardOutputMode::All | DiscardOutputMode::Stdout => true,
                _ => false,
            },
            discard_stderr: match command_line_args.discard_output_mode {
                DiscardOutputMode::All | DiscardOutputMode::Stdout => true,
                _ => false,
            },
        }
    }

    fn stdout(&self) -> Stdio {
        if self.discard_stdout {
            Stdio::null()
        } else {
            Stdio::piped()
        }
    }

    fn stderr(&self) -> Stdio {
        if self.discard_stderr {
            Stdio::null()
        } else {
            Stdio::piped()
        }
    }

    fn discard_all_output(&self) -> bool {
        self.discard_stdout && self.discard_stderr
    }
}

#[derive(Debug)]
struct Command {
    command_and_args: OwnedCommandAndArgs,
    input_line_number: InputLineNumber,
    command_output_mode: CommandOutputMode,
}

impl Command {
    async fn spawn_child_process(&self, command: &str, args: &[String]) -> std::io::Result<Output> {
        let mut child = TokioCommand::new(command)
            .args(args)
            .stdout(self.command_output_mode.stdout())
            .stderr(self.command_output_mode.stderr())
            .spawn()?;

        if span_enabled!(Level::DEBUG) {
            let child_pid = child.id();
            Span::current().record("child_pid", child_pid);

            debug!("spawned child process, awaiting output");
        }

        let output = if self.command_output_mode.discard_all_output() {
            Output {
                status: child.wait().await?,
                stdout: vec![],
                stderr: vec![],
            }
        } else {
            child.wait_with_output().await?
        };

        Ok(output)
    }

    #[instrument(
        name = "Command::run",
        skip_all,
        fields(
            cmd_args = %self.command_and_args,
            line = %self.input_line_number,
            child_pid,
        ),
        level = "debug")]
    async fn run(self, output_sender: OutputSender) {
        debug!("begin run");

        let [command, args @ ..] = self.command_and_args.0.as_slice() else {
            return;
        };

        match self.spawn_child_process(command, args).await {
            Err(e) => {
                warn!("error running command: {}: {}", self, e);
            }
            Ok(output) => {
                debug!("command status = {}", output.status);
                output_sender.send(output).await;
            }
        };

        debug!("end run");
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cmd_args={},line={}",
            self.command_and_args, self.input_line_number,
        )
    }
}

pub struct CommandService {
    command_line_args: &'static CommandLineArgs,
    input_line_parser: InputLineParser,
    command_semaphore: Arc<Semaphore>,
    output_writer: OutputWriter,
}

impl CommandService {
    pub fn new() -> Self {
        let command_line_args = command_line_args::instance();

        Self {
            command_line_args,
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
            command_and_args,
            input_line_number,
            command_output_mode: CommandOutputMode::new(self.command_line_args),
        };

        let output_sender = self.output_writer.sender();

        let permit = Arc::clone(&self.command_semaphore)
            .acquire_owned()
            .await
            .context("command_semaphore.acquire_owned error")?;

        tokio::spawn(async move {
            command.run(output_sender).await;

            drop(permit);
        });

        Ok(())
    }

    #[instrument(
        name = "CommandService::process_one_input",
        skip_all,
        fields(input = %input),
        level = "debug")]
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

    #[instrument(name = "CommandService::run_commands", skip_all, level = "debug")]
    pub async fn run_commands(self) -> anyhow::Result<()> {
        debug!("begin run_commands");

        self.process_inputs().await?;

        debug!("before output_writer.wait_for_completion",);

        self.output_writer.wait_for_completion().await?;

        debug!("end run_commands");

        Ok(())
    }
}
