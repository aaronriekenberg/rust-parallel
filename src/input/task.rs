use anyhow::Context;

use tokio::sync::mpsc::Sender;

use tracing::{debug, instrument, warn};

use std::sync::Arc;

use crate::{
    command_line_args::CommandLineArgs,
    parser::{Parsers, buffered::BufferedInputLineParser},
    progress::Progress,
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

    async fn send(&self, input_message: InputMessage) {
        self.progress.increment_total_commands(1);

        if let Err(e) = self.sender.send(input_message).await {
            warn!("input sender send error: {}", e);
        }
    }

    #[instrument(
        skip_all,
        fields(
            line=%input_line_number,
        )
        name = "process_buffered_input_line",
    )]
    async fn process_buffered_input_line(
        &self,
        parser: &BufferedInputLineParser,
        input_line_number: InputLineNumber,
        segment: Vec<u8>,
    ) {
        if let Some(command_and_args) = parser.parse_segment(segment) {
            self.send(InputMessage {
                command_and_args,
                input_line_number,
            })
            .await
        }
    }

    async fn process_buffered_input(&self, buffered_input: BufferedInput) -> anyhow::Result<()> {
        debug!(
            "begin process_buffered_input buffered_input {}",
            buffered_input
        );

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
                    let input_line_number = InputLineNumber {
                        input,
                        line_number: line_number.into(),
                    };
                    self.process_buffered_input_line(parser, input_line_number, segment)
                        .await
                }
                None => {
                    debug!("input_reader.next_segment EOF");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn process_command_line_args_input(self) {
        debug!("begin process_command_line_args_input");

        let parser = self.parsers.command_line_args_parser();

        let input = Input::CommandLineArgs;

        let num_argument_groups = parser.num_argument_groups();

        for line_number in 1..=num_argument_groups {
            let input_line_number = InputLineNumber {
                input,
                line_number: line_number.into(),
            };

            if let Some(command_and_args) = parser.parse_next_argument_group() {
                self.send(InputMessage {
                    command_and_args,
                    input_line_number,
                })
                .await;
            }
        }
    }

    async fn process_pipe_input(&self) -> anyhow::Result<()> {
        debug!("begin process_pipe_input");

        let input = Input::Buffered(BufferedInput::Stdin);

        let mut input_reader =
            BufferedInputReader::new(BufferedInput::Stdin, self.command_line_args).await?;

        let parser = self.parsers.pipe_mode_parser();

        let mut range_start_line_number = 1usize;
        let mut range_end_line_number = 1usize;

        loop {
            match input_reader
                .next_segment()
                .await
                .context("next_segment error")?
            {
                Some((_, line_number, segment)) => {
                    range_end_line_number = line_number;
                    let command_and_args_option = parser.parse_segment(segment);
                    if let Some(command_and_args) = command_and_args_option {
                        self.send(InputMessage {
                            command_and_args,
                            input_line_number: InputLineNumber {
                                input,
                                line_number: (range_start_line_number, range_end_line_number)
                                    .into(),
                            },
                        })
                        .await;
                        range_start_line_number = range_end_line_number + 1;
                    }
                }
                None => {
                    debug!("input_reader.next_segment EOF");
                    break;
                }
            }
        }

        if let Some(command_and_args) = parser.parse_last_command() {
            self.send(InputMessage {
                command_and_args,
                input_line_number: InputLineNumber {
                    input,
                    line_number: (range_start_line_number, range_end_line_number).into(),
                },
            })
            .await
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
