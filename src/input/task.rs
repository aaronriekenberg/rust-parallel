use anyhow::Context;

use tokio::sync::mpsc::Sender;

use tracing::{debug, instrument, warn};

use std::sync::Arc;

use crate::{
    command_line_args::CommandLineArgs,
    common::OwnedCommandAndArgs,
    parser::{buffered::BufferedInputLineParser, command_line::CommandLineArgsParser, Parser},
    progress::Progress,
};

use super::{
    buffered_reader::BufferedInputReader, BufferedInput, Input, InputLineNumber, InputList,
    InputMessage,
};

pub struct InputTask {
    sender: Sender<InputMessage>,
    command_line_args: &'static CommandLineArgs,
    progress: Arc<Progress>,
    parser: Parser,
}

impl InputTask {
    pub fn new(
        command_line_args: &'static CommandLineArgs,
        sender: Sender<InputMessage>,
        progress: &Arc<Progress>,
    ) -> anyhow::Result<Self> {
        let parser = Parser::new(command_line_args)?;
        Ok(Self {
            sender,
            command_line_args,
            progress: Arc::clone(progress),
            parser,
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
    fn process_buffered_input_line(
        &self,
        parser: &BufferedInputLineParser,
        input_line_number: &InputLineNumber,
        segment: Vec<u8>,
    ) -> Option<OwnedCommandAndArgs> {
        parser.parse_segment(segment)
    }

    async fn process_one_buffered_input(
        &self,
        buffered_input: BufferedInput,
    ) -> anyhow::Result<()> {
        debug!(
            "begin process_one_buffered_input buffered_input {}",
            buffered_input
        );

        let mut input_reader =
            BufferedInputReader::new(buffered_input, self.command_line_args).await?;

        let parser = self.parser.buffered_input_line_parser().await;

        loop {
            match input_reader
                .next_segment()
                .await
                .context("next_segment error")?
            {
                Some((input_line_number, segment)) => {
                    let Some(command_and_args) =
                        self.process_buffered_input_line(parser, &input_line_number, segment)
                    else {
                        continue;
                    };

                    self.send(InputMessage {
                        command_and_args,
                        input_line_number,
                    })
                    .await;
                }
                None => {
                    debug!("input_reader.next_segment EOF");
                    break;
                }
            }
        }

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            line=%input_line_number,
        )
        name = "process_next_command_line_arg",
    )]
    fn process_next_command_line_arg(
        &self,
        parser: &mut CommandLineArgsParser,
        input_line_number: &InputLineNumber,
    ) -> Option<OwnedCommandAndArgs> {
        parser.parse_next_argument_group()
    }

    async fn process_command_line_args_input(self) {
        debug!("begin process_command_line_args_input");

        let mut parser = self.parser.command_line_args_parser();

        let mut line_number = 0;

        while parser.has_remaining_argument_groups() {
            line_number += 1;

            let input_line_number = InputLineNumber {
                input: Input::CommandLineArgs,
                line_number,
            };

            let Some(command_and_args) =
                self.process_next_command_line_arg(&mut parser, &input_line_number)
            else {
                continue;
            };

            self.send(InputMessage {
                command_and_args,
                input_line_number,
            })
            .await;
        }
    }

    #[instrument(skip_all, name = "InputTask::run", level = "debug")]
    pub async fn run(self) {
        debug!("begin run");

        match super::build_input_list(self.command_line_args) {
            InputList::BufferedInputList(buffered_inputs) => {
                for buffered_input in buffered_inputs {
                    if let Err(e) = self.process_one_buffered_input(buffered_input).await {
                        warn!(
                            "process_one_buffered_input error buffered_input = {}: {}",
                            buffered_input, e
                        );
                    }
                }
            }
            InputList::CommandLineArgs => self.process_command_line_args_input().await,
        }

        debug!("end run");
    }
}
