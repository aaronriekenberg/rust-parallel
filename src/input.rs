mod task;

use anyhow::Context;

use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, BufReader, Split},
    sync::mpsc::{channel, Receiver},
    task::JoinHandle,
};

use tracing::debug;

use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs};

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

enum InputList {
    BufferedInputList(Vec<BufferedInput>),

    CommandLineArgs,
}

fn build_input_list(command_line_args: &'static CommandLineArgs) -> InputList {
    if command_line_args.commands_from_args_mode() {
        InputList::CommandLineArgs
    } else if command_line_args.input_file.is_empty() {
        InputList::BufferedInputList(vec![BufferedInput::Stdin])
    } else {
        InputList::BufferedInputList(
            command_line_args
                .input_file
                .iter()
                .map(|input_name| {
                    if input_name == "-" {
                        BufferedInput::Stdin
                    } else {
                        BufferedInput::File {
                            file_name: input_name,
                        }
                    }
                })
                .collect(),
        )
    }
}

type AsyncBufReadBox = Box<dyn AsyncBufRead + Unpin + Send>;

struct BufferedInputReader {
    buffered_input: BufferedInput,
    split: Split<AsyncBufReadBox>,
    next_line_number: usize,
}

impl BufferedInputReader {
    async fn new(
        buffered_input: BufferedInput,
        command_line_args: &CommandLineArgs,
    ) -> anyhow::Result<Self> {
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

    async fn next_segment(&mut self) -> anyhow::Result<Option<(InputLineNumber, Vec<u8>)>> {
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
    receiver: Receiver<InputMessage>,
}

impl InputProducer {
    pub fn new(command_line_args: &'static CommandLineArgs) -> Self {
        let (sender, receiver) = channel(command_line_args.channel_capacity);
        debug!(
            "created input channel with capacity {}",
            command_line_args.channel_capacity
        );

        let sender_task_join_handle =
            tokio::spawn(task::InputSenderTask::new(command_line_args, sender).run());

        Self {
            sender_task_join_handle,
            receiver,
        }
    }

    pub fn receiver(&mut self) -> &mut Receiver<InputMessage> {
        &mut self.receiver
    }

    pub async fn wait_for_completion(self) -> anyhow::Result<()> {
        self.sender_task_join_handle
            .await
            .context("sender_task_join_handle.await error")?;

        Ok(())
    }
}
