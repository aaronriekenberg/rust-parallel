mod buffered_reader;
mod task;

use anyhow::Context;

use tokio::{
    sync::mpsc::{Receiver, channel},
    task::JoinHandle,
};

use tracing::debug;

use std::sync::Arc;

use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, progress::Progress};

#[derive(Debug, Clone, Copy)]
pub enum BufferedInput {
    Stdin,

    File { file_name: &'static str },
}

impl std::fmt::Display for BufferedInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdin => write!(f, "stdin"),
            Self::File { file_name } => write!(f, "{file_name}"),
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
            Self::Buffered(b) => write!(f, "{b}"),
            Self::CommandLineArgs => write!(f, "command_line_args"),
        }
    }
}

#[derive(Debug)]
pub enum LineNumberOrRange {
    Single(usize),
    Range(usize, usize),
}

impl std::fmt::Display for LineNumberOrRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(line) => write!(f, "{line}"),
            Self::Range(start, end) => write!(f, "{start}-{end}"),
        }
    }
}

impl From<usize> for LineNumberOrRange {
    fn from(line_number: usize) -> Self {
        LineNumberOrRange::Single(line_number)
    }
}

impl From<(usize, usize)> for LineNumberOrRange {
    fn from(range: (usize, usize)) -> Self {
        LineNumberOrRange::Range(range.0, range.1)
    }
}

#[derive(Debug)]
pub struct InputLineNumber {
    pub input: Input,
    pub line_number: LineNumberOrRange,
}

impl std::fmt::Display for InputLineNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.input, self.line_number)
    }
}

impl From<(Input, LineNumberOrRange)> for InputLineNumber {
    fn from(value: (Input, LineNumberOrRange)) -> Self {
        Self {
            input: value.0,
            line_number: value.1,
        }
    }
}

enum InputList {
    Buffered(Vec<BufferedInput>),

    CommandLineArgs,

    Pipe,
}

fn build_input_list(command_line_args: &'static CommandLineArgs) -> InputList {
    if command_line_args.pipe {
        InputList::Pipe
    } else if command_line_args.commands_from_args_mode() {
        InputList::CommandLineArgs
    } else if command_line_args.input_file.is_empty() {
        InputList::Buffered(vec![BufferedInput::Stdin])
    } else {
        InputList::Buffered(
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

#[derive(Debug)]
pub struct InputMessage {
    pub command_and_args: OwnedCommandAndArgs,
    pub input_line_number: InputLineNumber,
}

impl From<(OwnedCommandAndArgs, Input, LineNumberOrRange)> for InputMessage {
    fn from(value: (OwnedCommandAndArgs, Input, LineNumberOrRange)) -> Self {
        Self {
            command_and_args: value.0,
            input_line_number: (value.1, value.2).into(),
        }
    }
}

pub struct InputProducer {
    input_task_join_handle: JoinHandle<()>,
    receiver: Receiver<InputMessage>,
}

impl InputProducer {
    pub fn new(
        command_line_args: &'static CommandLineArgs,
        progress: &Arc<Progress>,
    ) -> anyhow::Result<Self> {
        let (sender, receiver) = channel(command_line_args.channel_capacity);
        debug!(
            "created input channel with capacity {}",
            command_line_args.channel_capacity
        );

        let input_sender_task = task::InputTask::new(command_line_args, sender, progress)?;

        let input_task_join_handle = tokio::spawn(input_sender_task.run());

        Ok(Self {
            input_task_join_handle,
            receiver,
        })
    }

    pub fn receiver(&mut self) -> &mut Receiver<InputMessage> {
        &mut self.receiver
    }

    pub async fn wait_for_completion(self) -> anyhow::Result<()> {
        self.input_task_join_handle
            .await
            .context("InputProducer::wait_for_completion: input_task_join_handle.await error")?;

        Ok(())
    }
}
