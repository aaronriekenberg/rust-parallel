mod path_cache;

use anyhow::Context;

use indicatif::{ProgressBar, ProgressStyle};

use tokio::sync::Semaphore;

use tracing::{debug, instrument, span_enabled, warn, Level, Span};

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{
    command_line_args::CommandLineArgs,
    common::OwnedCommandAndArgs,
    input::{InputLineNumber, InputMessage, InputProducer},
    output::{OutputSender, OutputWriter},
    process::ChildProcessFactory,
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
    progress_bar: Arc<ProgressBar>,
    commands_spawned: Arc<AtomicUsize>,
    commands_finished: Arc<AtomicUsize>,
}

impl CommandService {
    pub fn new(command_line_args: &'static CommandLineArgs) -> Self {
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.enable_steady_tick(Duration::from_millis(200));
        progress_bar.set_style(
            ProgressStyle::with_template("{spinner:.dim.bold} [{elapsed_precise}] {wide_msg}")
                .unwrap()
                .tick_chars("/|\\- "),
        );

        Self {
            child_process_factory: ChildProcessFactory::new(command_line_args),
            command_semaphore: Arc::new(Semaphore::new(command_line_args.jobs)),
            output_writer: OutputWriter::new(command_line_args),
            command_path_cache: CommandPathCache::new(command_line_args),
            command_line_args,
            progress_bar: Arc::new(progress_bar),
            commands_spawned: Arc::new(AtomicUsize::new(0)),
            commands_finished: Arc::new(AtomicUsize::new(0)),
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

        self.commands_spawned.fetch_add(1, Ordering::Relaxed);

        let commands_spawned_clone = Arc::clone(&self.commands_spawned);

        let commands_finished_clone = Arc::clone(&self.commands_finished);

        self.progress_bar.set_message(format!(
            "commands spawned {} commands finished {}",
            commands_spawned_clone.load(Ordering::Relaxed),
            commands_finished_clone.load(Ordering::Relaxed)
        ));
        self.progress_bar.tick();

        let progress_bar_clone = Arc::clone(&self.progress_bar);

        tokio::spawn(async move {
            command.run(child_process_factory, output_sender).await;

            commands_finished_clone.fetch_add(1, Ordering::Relaxed);

            progress_bar_clone.set_message(format!(
                "commands spawned {} commands finished {}",
                commands_spawned_clone.load(Ordering::Relaxed),
                commands_finished_clone.load(Ordering::Relaxed)
            ));
            progress_bar_clone.tick();

            drop(permit);
        });

        Ok(())
    }

    async fn process_inputs(&self) -> anyhow::Result<()> {
        let mut input_producer = InputProducer::new(self.command_line_args);

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

        self.progress_bar
            .finish_with_message("all commands complete");

        Ok(())
    }
}
