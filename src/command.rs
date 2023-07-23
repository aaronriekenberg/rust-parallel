mod path_cache;

use anyhow::Context;

use tokio::sync::Semaphore;

use tracing::{debug, instrument, span_enabled, warn, Level, Span};

use std::sync::Arc;

use crate::{
    command_line_args::CommandLineArgs,
    common::OwnedCommandAndArgs,
    input::{InputLineNumber, InputMessage, InputProducer},
    output::{OutputSender, OutputWriter},
    process::ChildProcessFactory,
    progress::Progress,
};

use self::path_cache::CommandPathCache;

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
            cmd = ?self.command_and_args.command_path,
            args = ?self.command_and_args.args,
            line = %self.input_line_number,
            child_pid,
        ),
        level = "debug")]
    async fn run(self, child_process_factory: ChildProcessFactory, output_sender: OutputSender) {
        debug!("begin run");

        let OwnedCommandAndArgs { command_path, args } = &self.command_and_args;

        let child_process = match child_process_factory.spawn(command_path, args).await {
            Err(e) => {
                warn!("spawn error command: {}: {}", self, e);
                return;
            }
            Ok(child_process) => child_process,
        };

        if span_enabled!(Level::DEBUG) {
            let child_pid = child_process.id();
            Span::current().record("child_pid", child_pid);

            debug!("spawned child process, awaiting completion");
        }

        match child_process.await_completion().await {
            Err(e) => {
                warn!("child process error command: {} error: {}", self, e);
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
            "cmd={:?},args={:?},line={}",
            self.command_and_args.command_path, self.command_and_args.args, self.input_line_number,
        )
    }
}

pub struct CommandService {
    child_process_factory: ChildProcessFactory,
    command_semaphore: Arc<Semaphore>,
    output_writer: OutputWriter,
    command_path_cache: CommandPathCache,
    command_line_args: &'static CommandLineArgs,
    progress: Arc<Progress>,
}

impl CommandService {
    pub fn new(command_line_args: &'static CommandLineArgs) -> Self {
        Self {
            child_process_factory: ChildProcessFactory::new(command_line_args),
            command_semaphore: Arc::new(Semaphore::new(command_line_args.jobs)),
            output_writer: OutputWriter::new(command_line_args),
            command_path_cache: CommandPathCache::new(command_line_args),
            command_line_args,
            progress: Arc::new(Progress::new(command_line_args)),
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

        let progress_clone = Arc::clone(&self.progress);

        let permit = Arc::clone(&self.command_semaphore)
            .acquire_owned()
            .await
            .context("command_semaphore.acquire_owned error")?;

        tokio::spawn(async move {
            command.run(child_process_factory, output_sender).await;

            progress_clone.command_finished();

            drop(permit);
        });

        Ok(())
    }

    async fn process_inputs(&self) -> anyhow::Result<()> {
        let mut input_producer = InputProducer::new(self.command_line_args);

        let mut num_commands = 0u64;

        while let Some(InputMessage {
            command_and_args,
            input_line_number,
        }) = input_producer.receiver().recv().await
        {
            let Some(command_and_args) = self
                .command_path_cache
                .resolve_command_path(command_and_args)
                .await? else {
                continue;
            };

            num_commands += 1;

            self.progress.set_total_commands(num_commands);

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

        self.progress.finish();

        debug!("end run_commands");

        Ok(())
    }
}
