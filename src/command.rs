use anyhow::Context;

use tokio::sync::{mpsc::channel, Semaphore};

use tracing::{debug, instrument, span_enabled, warn, Level, Span};

use std::sync::Arc;

use crate::{
    command_line_args,
    common::OwnedCommandAndArgs,
    input::{InputLineNumber, InputMessage, InputProducer},
    output::{OutputSender, OutputWriter},
    process::ChildProcessFactory,
};

#[derive(Debug)]
struct Command {
    command_and_args: OwnedCommandAndArgs,
    input_line_number: InputLineNumber,
}

impl Command {
    #[instrument(
        name = "Command::run",
        skip_all,
        fields(
            cmd_args = %self.command_and_args,
            line = %self.input_line_number,
            child_pid,
        ),
        level = "debug")]
    async fn run(self, child_process_factory: ChildProcessFactory, output_sender: OutputSender) {
        debug!("begin run");

        let [command, args @ ..] = self.command_and_args.as_slice() else {
            return;
        };

        let child_process = match child_process_factory.spawn(command, args).await {
            Err(e) => {
                warn!("spawn error command: {}: {}", self, e);
                return;
            }
            Ok(child_process) => child_process,
        };

        if span_enabled!(Level::DEBUG) {
            let child_pid = child_process.id();
            Span::current().record("child_pid", child_pid);

            debug!("spawned child process, awaiting output");
        }

        match child_process.await_output().await {
            Err(e) => {
                warn!("await_output error command: {}: {}", self, e);
            }
            Ok(output) => {
                debug!("command exit status = {}", output.status);
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
    child_process_factory: ChildProcessFactory,
    command_semaphore: Arc<Semaphore>,
    output_writer: OutputWriter,
}

impl CommandService {
    pub async fn new() -> Self {
        let command_line_args = command_line_args::instance();

        Self {
            child_process_factory: ChildProcessFactory::new(command_line_args),
            command_semaphore: Arc::new(Semaphore::new(command_line_args.jobs)),
            output_writer: OutputWriter::new(command_line_args),
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
        };

        let child_process_factory = self.child_process_factory.clone();

        let output_sender = self.output_writer.sender();

        let permit = Arc::clone(&self.command_semaphore)
            .acquire_owned()
            .await
            .context("command_semaphore.acquire_owned error")?;

        tokio::spawn(async move {
            command.run(child_process_factory, output_sender).await;

            drop(permit);
        });

        Ok(())
    }

    async fn process_inputs(&self) -> anyhow::Result<()> {
        let (sender, mut receiver) = channel(1);
        let input_producer = InputProducer::new(sender);

        while let Some(InputMessage {
            command_and_args,
            input_line_number,
        }) = receiver.recv().await
        {
            self.spawn_command(command_and_args, input_line_number)
                .await?;
        }

        input_producer.wait_for_completion().await?;

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
