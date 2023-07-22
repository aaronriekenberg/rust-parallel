use anyhow::Context;

use tokio::sync::{mpsc::Sender, OnceCell};

use tracing::{debug, instrument, warn};

use crate::{
    command_line_args::CommandLineArgs,
    parser::{buffered::BufferedInputLineParser, command_line::CommandLineArgsParser},
};

use super::{
    buffered_reader::BufferedInputReader, BufferedInput, Input, InputLineNumber, InputList,
    InputMessage,
};

pub struct InputSenderTask {
    sender: Sender<InputMessage>,
    command_line_args: &'static CommandLineArgs,
    buffered_input_line_parser: OnceCell<BufferedInputLineParser>,
}

impl InputSenderTask {
    pub fn new(command_line_args: &'static CommandLineArgs, sender: Sender<InputMessage>) -> Self {
        Self {
            sender,
            command_line_args,
            buffered_input_line_parser: OnceCell::new(),
        }
    }

    async fn send(&self, input_message: InputMessage) {
        if let Err(e) = self.sender.send(input_message).await {
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

        let parser = self
            .buffered_input_line_parser
            .get_or_init(|| async move { BufferedInputLineParser::new(self.command_line_args) })
            .await;

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

                    let input_message = InputMessage {
                        command_and_args,
                        input_line_number,
                    };

                    self.send(input_message).await;
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

        let parser = CommandLineArgsParser::new(self.command_line_args);

        for (i, command_and_args) in parser.parse_command_line_args().into_iter().enumerate() {
            let input_message = InputMessage {
                command_and_args,
                input_line_number: InputLineNumber {
                    input: Input::CommandLineArgs,
                    line_number: i,
                },
            };
            self.send(input_message).await;
        }
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
