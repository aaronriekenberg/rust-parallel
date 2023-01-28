use anyhow::Context;

use tokio::{process::Command as TokioCommand, sync::Semaphore};

use tracing::{debug, warn};

use std::{collections::VecDeque, sync::Arc};

use crate::{
    command_line_args,
    command_line_args::CommandLineArgs,
    input::{Input, InputLineNumber, InputReader},
    output::OutputWriter,
};

type CommandAndArgs = (String, VecDeque<String>);

#[derive(Debug)]
struct Command {
    command_and_args: CommandAndArgs,
    input_line_number: InputLineNumber,
}

impl Command {
    async fn run(self, output_writer: Arc<OutputWriter>) {
        debug!("begin run command = {:?}", self);

        let (command, args) = &self.command_and_args;

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
            "command={},args={:?},input_line_number={}",
            self.command_and_args.0, self.command_and_args.1, self.input_line_number,
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
            command.run(output_writer_clone).await;

            drop(permit);
        });

        Ok(())
    }

    fn build_command_and_args(&self, input_line: String) -> Option<CommandAndArgs> {
        let mut deque: VecDeque<String> = if self.command_line_args.null_separator {
            VecDeque::from([input_line])
        } else {
            input_line
                .split_whitespace()
                .map(|s| s.to_owned())
                .collect()
        };

        let command_and_initial_arguments = &self.command_line_args.command_and_initial_arguments;

        if command_and_initial_arguments.len() > 0 {
            deque.reserve_exact(command_and_initial_arguments.len());

            command_and_initial_arguments
                .iter()
                .rev()
                .cloned()
                .for_each(|s| deque.push_front(s));
        }

        if deque.is_empty() {
            None
        } else {
            let command = deque.pop_front().unwrap();
            let args = deque;
            Some((command, args))
        }
    }

    async fn process_one_input(&self, input: Input) -> anyhow::Result<()> {
        debug!("begin process_one_input input = {:?}", input);

        let mut input_reader = InputReader::new(input).await?;

        while let Some((input_line_number, segment)) = input_reader
            .next_segment()
            .await
            .context("next_segment error")?
        {
            let Ok(input_line) = String::from_utf8(segment) else {
                continue;
            };

            if let Some(command_and_args) = self.build_command_and_args(input_line) {
                self.spawn_command(command_and_args, input_line_number)
                    .await?;
            }
        }

        debug!("end process_one_input input = {:?}", input);

        Ok(())
    }

    async fn process_inputs(&self) -> anyhow::Result<()> {
        for input in crate::input::build_input_list() {
            self.process_one_input(input).await?;
        }
        Ok(())
    }

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
