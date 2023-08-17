use anyhow::Context;

use itertools::Itertools;

use tokio::sync::mpsc::Sender;

use tracing::{debug, instrument, warn};

use std::sync::Arc;

use crate::{command_line_args::CommandLineArgs, parser::Parser, progress::Progress};

use super::{
    buffered_reader::BufferedInputReader, BufferedInput, Input, InputLineNumber, InputList,
    InputMessage, InputMessageList,
};

pub struct InputSenderTask {
    sender: Sender<InputMessageList>,
    command_line_args: &'static CommandLineArgs,
    progress: Arc<Progress>,
    parser: Parser,
}

impl InputSenderTask {
    pub fn new(
        command_line_args: &'static CommandLineArgs,
        sender: Sender<InputMessageList>,
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

    async fn send(&self, input_message_list: InputMessageList) {
        self.progress
            .increment_total_commands(input_message_list.message_list.len());

        if let Err(e) = self.sender.send(input_message_list).await {
            warn!("input sender send error: {}", e);
        }
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
                    let Some(command_and_args) = parser.parse_segment(segment) else {
                        continue;
                    };

                    self.send(
                        InputMessage {
                            command_and_args,
                            input_line_number,
                        }
                        .into(),
                    )
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

    async fn process_command_line_args_input(self) {
        debug!("begin process_command_line_args_input");

        let parser = self.parser.command_line_args_parser().await;

        let message_list = parser
            .parse_command_line_args()
            .into_iter()
            .enumerate()
            .map(|(i, command_and_args)| InputMessage {
                command_and_args,
                input_line_number: InputLineNumber {
                    input: Input::CommandLineArgs,
                    line_number: i,
                },
            })
            .collect_vec();

        self.send(message_list.into()).await;
    }

    #[instrument(skip_all, name = "InputSenderTask::run", level = "debug")]
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
