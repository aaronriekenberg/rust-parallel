mod task;

use anyhow::Context;

use tokio::{
    sync::mpsc::{channel, Sender},
    task::JoinHandle,
};

use tracing::{debug, warn};

use std::process::ExitStatus;

use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, input::InputLineNumber,
};

#[derive(Debug)]
pub struct OutputMessage {
    pub exit_status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub command_and_args: OwnedCommandAndArgs,
    pub input_line_number: InputLineNumber,
}

pub struct OutputSender {
    sender: Sender<OutputMessage>,
}

impl OutputSender {
    pub async fn send(self, output_message: OutputMessage) {
        if output_message.exit_status.success()
            && output_message.stdout.is_empty()
            && output_message.stderr.is_empty()
        {
            return;
        }
        if let Err(e) = self.sender.send(output_message).await {
            warn!("sender.send error: {}", e);
        }
    }
}

pub struct OutputWriter {
    sender: Sender<OutputMessage>,
    output_task_join_handle: JoinHandle<usize>,
}

impl OutputWriter {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let (sender, receiver) = channel(command_line_args.channel_capacity);
        debug!(
            "created output channel with capacity {}",
            command_line_args.channel_capacity,
        );

        let output_task_join_handle = tokio::spawn(task::OutputTask::new(receiver).run());

        Self {
            sender,
            output_task_join_handle,
        }
    }

    pub fn sender(&self) -> OutputSender {
        OutputSender {
            sender: self.sender.clone(),
        }
    }

    pub async fn wait_for_completion(self) -> anyhow::Result<usize> {
        drop(self.sender);

        let failed = self
            .output_task_join_handle
            .await
            .context("OutputWriter::wait_for_completion: output_task_join_handle.await error")?;

        Ok(failed)
    }
}
