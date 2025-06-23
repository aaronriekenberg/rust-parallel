mod task;

use anyhow::Context;

use tokio::{
    sync::mpsc::{Sender, channel},
    task::JoinHandle,
};

use tracing::{debug, warn};

use std::process::{ExitStatus, Output};

use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, input::InputLineNumber,
    progress::Progress,
};
use std::sync::Arc;

#[derive(Debug)]
struct OutputMessage {
    exit_status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    command_and_args: OwnedCommandAndArgs,
    input_line_number: InputLineNumber,
}

pub struct OutputSender {
    sender: Sender<OutputMessage>,
}

impl OutputSender {
    pub async fn send(
        self,
        output: Output,
        command_and_args: OwnedCommandAndArgs,
        input_line_number: InputLineNumber,
    ) {
        if output.status.success() && output.stdout.is_empty() && output.stderr.is_empty() {
            return;
        }

        let output_message = OutputMessage {
            exit_status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
            command_and_args,
            input_line_number,
        };

        if let Err(e) = self.sender.send(output_message).await {
            warn!("sender.send error: {}", e);
        }
    }
}

pub struct OutputWriter {
    sender: Sender<OutputMessage>,
    output_task_join_handle: JoinHandle<()>,
}

impl OutputWriter {
    pub fn new(command_line_args: &CommandLineArgs, progress: Arc<Progress>) -> Self {
        let (sender, receiver) = channel(command_line_args.channel_capacity);
        debug!(
            "created output channel with capacity {}",
            command_line_args.channel_capacity,
        );

        let output_task_join_handle =
            tokio::spawn(task::OutputTask::new(receiver, command_line_args.keep_order, progress).run());

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

    pub async fn wait_for_completion(self) -> anyhow::Result<()> {
        drop(self.sender);

        self.output_task_join_handle
            .await
            .context("OutputWriter::wait_for_completion: output_task_join_handle.await error")?;

        Ok(())
    }
}
