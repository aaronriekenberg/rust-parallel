use anyhow::Context;

use tokio::sync::mpsc::Sender;

use tracing::{debug, instrument, warn};

use std::sync::Arc;

use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, input::LineNumberOrRange,
    parser::Parsers, progress::Progress,
};

use super::{
    BufferedInput, Input, InputLineNumber, InputList, InputMessage,
    buffered_reader::BufferedInputReader,
};

pub struct InputTask {
    sender: Sender<InputMessage>,
    command_line_args: &'static CommandLineArgs,
    progress: Arc<Progress>,
    parsers: Parsers,
}

impl InputTask {
    pub fn new(
        command_line_args: &'static CommandLineArgs,
        sender: Sender<InputMessage>,
        progress: &Arc<Progress>,
    ) -> anyhow::Result<Self> {
        let parsers = Parsers::new(command_line_args)?;
        Ok(Self {
            sender,
            command_line_args,
            progress: Arc::clone(progress),
            parsers,
        })
    }

    async fn send(
        &self,
        command_and_args: OwnedCommandAndArgs,
        input: Input,
        line_number: LineNumberOrRange,
    ) {
        self.progress.increment_total_commands(1);

        let input_message = InputMessage {
            command_and_args,
            input_line_number: InputLineNumber { input, line_number },
        };

        if let Err(e) = self.sender.send(input_message).await {
            warn!("input sender send error: {}", e);
        }
    }

    #[instrument(
        name = "InputTask::process_buffered_input",
        skip_all,
        fields(
            buffered_input = %buffered_input
        ),
        level = "debug"
    )]
    async fn process_buffered_input(&self, buffered_input: BufferedInput) -> anyhow::Result<()> {
        debug!("begin process_buffered_input");

        let mut input_reader =
            BufferedInputReader::new(buffered_input, self.command_line_args).await?;

        let parser = self.parsers.buffered_input_line_parser();

        loop {
            match input_reader
                .next_segment()
                .await
                .context("next_segment error")?
            {
                Some((input, line_number, segment)) => {
                    if let Some(command_and_args) = parser.parse_segment(segment) {
                        self.send(command_and_args, input, line_number.into()).await
                    }
                }
                None => {
                    debug!("input_reader.next_segment EOF");
                    break;
                }
            }
        }

        debug!("end process_buffered_input");

        Ok(())
    }

    #[instrument(
        name = "InputTask::process_command_line_args_input",
        skip_all,
        level = "debug"
    )]
    async fn process_command_line_args_input(&self) {
        debug!("begin process_command_line_args_input");

        let parser = self.parsers.command_line_args_parser();

        let input = Input::CommandLineArgs;

        let num_argument_groups = parser.num_argument_groups();

        for i in 0..num_argument_groups {
            let line_number = i + 1;

            if let Some(command_and_args) = parser.parse_next_argument_group() {
                self.send(command_and_args, input, line_number.into()).await;
            }
        }

        debug!("end process_command_line_args_input");
    }

    #[instrument(name = "InputTask::process_pipe_input", skip_all, level = "debug")]
    async fn process_pipe_input(&self) -> anyhow::Result<()> {
        debug!("begin process_pipe_input");

        let input = Input::Buffered(BufferedInput::Stdin);

        let mut input_reader =
            BufferedInputReader::new(BufferedInput::Stdin, self.command_line_args).await?;

        let parser = self.parsers.pipe_mode_parser();

        let mut range_start = 1usize;
        let mut range_end = range_start;

        loop {
            match input_reader
                .next_segment()
                .await
                .context("next_segment error")?
            {
                Some((_, line_number, segment)) => {
                    range_end = line_number;
                    if let Some(command_and_args) = parser.parse_segment(segment) {
                        self.send(command_and_args, input, (range_start, range_end).into())
                            .await;
                        range_start = range_end + 1;
                    }
                }
                None => {
                    debug!("input_reader.next_segment EOF");
                    if let Some(command_and_args) = parser.parse_last_command() {
                        self.send(command_and_args, input, (range_start, range_end).into())
                            .await;
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    #[instrument(skip_all, name = "InputTask::run", level = "debug")]
    pub async fn run(self) {
        debug!("begin run");

        match super::build_input_list(self.command_line_args) {
            InputList::Buffered(buffered_inputs) => {
                for buffered_input in buffered_inputs {
                    self.process_buffered_input(buffered_input)
                        .await
                        .unwrap_or_else(|e| {
                            warn!(
                                "process_buffered_input error buffered_input = {}: {}",
                                buffered_input, e
                            );
                        });
                }
            }
            InputList::CommandLineArgs => self.process_command_line_args_input().await,
            InputList::Pipe => self.process_pipe_input().await.unwrap_or_else(|e| {
                warn!("process_pipe_input error: {}", e);
            }),
        }

        debug!("end run");
    }
}
