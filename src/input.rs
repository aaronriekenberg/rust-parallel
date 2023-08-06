mod buffered_reader;
mod task;

use anyhow::Context;

use tokio::{
    sync::mpsc::{channel, Receiver},
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

#[derive(Debug)]
pub struct InputMessage {
    pub command_and_args: OwnedCommandAndArgs,
    pub input_line_number: InputLineNumber,
}

#[derive(Debug)]
pub struct InputMessageList {
    pub message_list: Vec<InputMessage>,
}

impl From<InputMessage> for InputMessageList {
    fn from(input_message: InputMessage) -> Self {
        Self {
            message_list: vec![input_message],
        }
    }
}

impl From<Vec<InputMessage>> for InputMessageList {
    fn from(message_list: Vec<InputMessage>) -> Self {
        Self { message_list }
    }
}

pub struct InputProducer {
    sender_task_join_handle: JoinHandle<()>,
    receiver: Receiver<InputMessageList>,
}

impl InputProducer {
    pub fn new(command_line_args: &'static CommandLineArgs, progress: &Arc<Progress>) -> Self {
        let (sender, receiver) = channel(command_line_args.channel_capacity);
        debug!(
            "created input channel with capacity {}",
            command_line_args.channel_capacity
        );

        let sender_task_join_handle =
            tokio::spawn(task::InputSenderTask::new(command_line_args, sender, progress).run());

        Self {
            sender_task_join_handle,
            receiver,
        }
    }

    pub fn receiver(&mut self) -> &mut Receiver<InputMessageList> {
        &mut self.receiver
    }

    pub async fn wait_for_completion(self) -> anyhow::Result<()> {
        self.sender_task_join_handle
            .await
            .context("sender_task_join_handle.await error")?;

        Ok(())
    }
}
