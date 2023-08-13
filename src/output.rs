mod task;

use anyhow::Context;

use tokio::{
    sync::mpsc::{channel, Sender},
    task::JoinHandle,
};

use tracing::{debug, warn};

use std::process::Output;

use crate::command_line_args::CommandLineArgs;

pub struct OutputSender {
    sender: Sender<Output>,
}

impl OutputSender {
    pub async fn send(self, output: Output) {
        if output.stdout.is_empty() && output.stderr.is_empty() {
            return;
        }
        if let Err(e) = self.sender.send(output).await {
            warn!("sender.send error: {}", e);
        }
    }
}

pub struct OutputWriter {
    sender: Sender<Output>,
    receiver_task_join_handle: JoinHandle<()>,
}

impl OutputWriter {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let (sender, receiver) = channel(command_line_args.channel_capacity);
        debug!(
            "created output channel with capacity {}",
            command_line_args.channel_capacity,
        );

        let receiver_task_join_handle = tokio::spawn(task::OutputReceiverTask::new(receiver).run());

        Self {
            sender,
            receiver_task_join_handle,
        }
    }

    pub fn sender(&self) -> OutputSender {
        OutputSender {
            sender: self.sender.clone(),
        }
    }

    pub async fn wait_for_completion(self) -> anyhow::Result<()> {
        drop(self.sender);

        self.receiver_task_join_handle
            .await
            .context("OutputWriter::wait_for_completion: receiver_task_join_handle.await error")?;

        Ok(())
    }
}
