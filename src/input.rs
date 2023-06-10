use anyhow::Context;

use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, BufReader, Split},
    sync::{mpsc::Sender, OnceCell},
    task::JoinHandle,
};

use tracing::{debug, warn};

use crate::{
    command_line_args,
    command_line_args::CommandLineArgs,
    common::OwnedCommandAndArgs,
    parser::{BufferedInputLineParser, CommandLineArgsParser},
};

#[derive(Debug, Clone, Copy)]
pub enum BufferedInput {
    Stdin,

    File { file_name: &'static str },
}

impl std::fmt::Display for BufferedInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdin => write!(f, "stdin"),
            Self::File { file_name } => write!(f, "{}", file_name),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Input {
    Buffered(BufferedInput),

    CommandLineArgs,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buffered(b) => write!(f, "{}", b),
            Self::CommandLineArgs => write!(f, "command_line_args"),
        }
    }
}

#[derive(Debug)]
pub struct InputLineNumber {
    pub input: Input,
    pub line_number: usize,
}

impl std::fmt::Display for InputLineNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.input, self.line_number)
    }
}

fn build_input_list(command_line_args: &'static CommandLineArgs) -> Vec<Input> {
    if command_line_args.commands_from_args {
        vec![Input::CommandLineArgs]
    } else if command_line_args.input.is_empty() {
        vec![Input::Buffered(BufferedInput::Stdin)]
    } else {
        command_line_args
            .input
            .iter()
            .map(|input_name| {
                if input_name == "-" {
                    Input::Buffered(BufferedInput::Stdin)
                } else {
                    Input::Buffered(BufferedInput::File {
                        file_name: input_name,
                    })
                }
            })
            .collect()
    }
}

type AsyncBufReadBox = Box<dyn AsyncBufRead + Unpin + Send>;

pub struct BufferedInputReader {
    buffered_input: BufferedInput,
    split: Split<AsyncBufReadBox>,
    next_line_number: usize,
}

impl BufferedInputReader {
    pub async fn new(buffered_input: BufferedInput) -> anyhow::Result<Self> {
        let command_line_args = command_line_args::instance();

        let buf_reader = Self::create_buf_reader(buffered_input).await?;

        let line_separator = if command_line_args.null_separator {
            0u8
        } else {
            b'\n'
        };

        let split = buf_reader.split(line_separator);

        Ok(Self {
            buffered_input,
            split,
            next_line_number: 0,
        })
    }

    async fn create_buf_reader(buffered_input: BufferedInput) -> anyhow::Result<AsyncBufReadBox> {
        match buffered_input {
            BufferedInput::Stdin => {
                let buf_reader = BufReader::new(tokio::io::stdin());

                Ok(Box::new(buf_reader))
            }
            BufferedInput::File { file_name } => {
                let file = tokio::fs::File::open(file_name).await.with_context(|| {
                    format!("error opening input file file_name = '{}'", file_name)
                })?;
                let buf_reader = BufReader::new(file);

                Ok(Box::new(buf_reader))
            }
        }
    }

    pub async fn next_segment(&mut self) -> anyhow::Result<Option<(InputLineNumber, Vec<u8>)>> {
        let segment = self.split.next_segment().await?;

        match segment {
            None => Ok(None),
            Some(segment) => {
                self.next_line_number += 1;

                let input_line_number = InputLineNumber {
                    input: Input::Buffered(self.buffered_input),
                    line_number: self.next_line_number,
                };

                Ok(Some((input_line_number, segment)))
            }
        }
    }
}

#[derive(Debug)]
pub struct InputMessage {
    pub command_and_args: OwnedCommandAndArgs,
    pub input_line_number: InputLineNumber,
}

pub struct InputProducer {
    sender_task_join_handle: JoinHandle<()>,
}

impl InputProducer {
    pub fn new(sender: Sender<InputMessage>) -> Self {
        let sender_task_join_handle = tokio::spawn(InputSender::new(sender).run());

        Self {
            sender_task_join_handle,
        }
    }

    pub async fn wait_for_completion(self) -> anyhow::Result<()> {
        self.sender_task_join_handle
            .await
            .context("sender_task_join_handle.await error")?;

        Ok(())
    }
}

struct InputSender {
    sender: Sender<InputMessage>,
    command_line_args: &'static CommandLineArgs,
    buffered_input_line_parser: OnceCell<BufferedInputLineParser>,
    command_line_args_parser: OnceCell<CommandLineArgsParser>,
}

impl InputSender {
    fn new(sender: Sender<InputMessage>) -> Self {
        let command_line_args = crate::command_line_args::instance();

        Self {
            sender,
            command_line_args,
            buffered_input_line_parser: OnceCell::new(),
            command_line_args_parser: OnceCell::new(),
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

        let parser = self
            .buffered_input_line_parser
            .get_or_init(|| async move { BufferedInputLineParser::new(self.command_line_args) })
            .await;

        let mut input_reader = BufferedInputReader::new(buffered_input).await?;

        loop {
            match input_reader
                .next_segment()
                .await
                .context("next_segment error")?
            {
                Some((input_line_number, segment)) => {
                    let Ok(input_line) = std::str::from_utf8(&segment) else {
                        continue;
                    };

                    let Some(command_and_args) = parser.parse_line(input_line) else {
                        continue;
                     };

                    let input_message = InputMessage {
                        command_and_args,
                        input_line_number,
                    };

                    if let Err(e) = self.sender.send(input_message).await {
                        warn!("input sender send error: {}", e);
                    }
                }
                None => {
                    debug!("input_reader.next_segment EOF");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn process_command_line_args_input(&self) {
        debug!("begin process_command_line_args_input");

        let parser = self
            .command_line_args_parser
            .get_or_init(|| async move { CommandLineArgsParser::new(self.command_line_args) })
            .await;

        for (i, command_and_args) in parser.parse_command_line_args().into_iter().enumerate() {
            let input_message = InputMessage {
                command_and_args,
                input_line_number: InputLineNumber {
                    input: Input::CommandLineArgs,
                    line_number: i,
                },
            };
            if let Err(e) = self.sender.send(input_message).await {
                warn!("input sender send error: {}", e);
            }
        }
    }

    async fn run(self) {
        debug!("begin InputSender.run");

        let inputs = build_input_list(self.command_line_args);
        for input in inputs {
            match input {
                Input::Buffered(buffered_input) => {
                    if let Err(e) = self.process_one_buffered_input(buffered_input).await {
                        warn!(
                            "process_one_buffered_input error buffered_input = {}: {}",
                            buffered_input, e
                        );
                    }
                }
                Input::CommandLineArgs => self.process_command_line_args_input().await,
            }
        }

        debug!("end InputSender.run");
    }
}
