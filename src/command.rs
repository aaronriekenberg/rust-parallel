mod metrics;
mod path_cache;

use anyhow::Context;

use tokio::sync::Semaphore;

use tracing::{Level, Span, debug, error, info, instrument, span_enabled, trace};

use std::sync::Arc;

use crate::{
    command_line_args::CommandLineArgs,
    common::OwnedCommandAndArgs,
    input::{InputLineNumber, InputMessage, InputProducer},
    output::{OutputSender, OutputWriter},
    process::ChildProcessFactory,
    progress::Progress,
};

use self::{metrics::CommandMetrics, path_cache::CommandPathCache};

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
    async fn run(self, context: &CommandRunContext, output_sender: OutputSender) {
        debug!("begin run");

        let command_metrics = &context.command_metrics;

        let OwnedCommandAndArgs { command_path, args } = &self.command_and_args;

        command_metrics.increment_commands_run();

        let child_process = match context
            .child_process_factory
            .spawn(command_path, args)
            .await
        {
            Err(e) => {
                error!("spawn error command: {}: {}", self, e);
                command_metrics.increment_spawn_errors();
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
                error!("child process error command: {} error: {}", self, e);
                command_metrics.handle_child_process_execution_error(e);
            }
            Ok(output) => {
                debug!("command exit status = {}", output.status);
                if !output.status.success() {
                    command_metrics.increment_exit_status_errors();
                }

                output_sender
                    .send(output, self.command_and_args, self.input_line_number)
                    .await;
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
    command_line_args: &'static CommandLineArgs,
    command_path_cache: CommandPathCache,
    command_semaphore: Arc<Semaphore>,
    context: Arc<CommandRunContext>,
    output_writer: OutputWriter,
}

impl CommandService {
    pub fn new(command_line_args: &'static CommandLineArgs, progress: Arc<Progress>) -> Self {
        let context = Arc::new(CommandRunContext {
            child_process_factory: ChildProcessFactory::new(command_line_args),
            command_metrics: CommandMetrics::default(),
            progress,
        });
        Self {
            command_line_args,
            command_path_cache: CommandPathCache::new(command_line_args),
            command_semaphore: Arc::new(Semaphore::new(command_line_args.jobs)),
            context,
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

        if self.command_line_args.dry_run {
            info!("{}", command);
            return Ok(());
        }

        if self.command_line_args.exit_on_error && self.context.command_metrics.error_occurred() {
            trace!("return from spawn_command due to exit_on_error");
            return Ok(());
        }

        let context_clone = Arc::clone(&self.context);

        let output_sender = self.output_writer.sender();

        let permit = Arc::clone(&self.command_semaphore)
            .acquire_owned()
            .await
            .context("command_semaphore.acquire_owned error")?;

        tokio::spawn(async move {
            command.run(&context_clone, output_sender).await;

            drop(permit);

            context_clone.progress.command_finished();
        });

        Ok(())
    }

    async fn process_input_message(&self, input_message: InputMessage) -> anyhow::Result<()> {
        let InputMessage {
            command_and_args,
            input_line_number,
        } = input_message;

        let Some(command_and_args) = self
            .command_path_cache
            .resolve_command_path(command_and_args)
            .await?
        else {
            return Ok(());
        };

        self.spawn_command(command_and_args, input_line_number)
            .await?;

        Ok(())
    }

    async fn process_inputs(&self) -> anyhow::Result<()> {
        let mut input_producer =
            InputProducer::new(self.command_line_args, &self.context.progress)?;

        while let Some(input_message) = input_producer.receiver().recv().await {
            self.process_input_message(input_message).await?;
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

        self.context.progress.finish();

        if self.context.command_metrics.error_occurred() {
            anyhow::bail!("command failures: {}", self.context.command_metrics);
        }

        debug!(
            "end run_commands command_metrics = {}",
            self.context.command_metrics
        );

        Ok(())
    }
}

struct CommandRunContext {
    child_process_factory: ChildProcessFactory,
    command_metrics: CommandMetrics,
    progress: Arc<Progress>,
}
