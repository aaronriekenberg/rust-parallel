use anyhow::Context;

use tokio::{
    io::AsyncWrite,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};

use tracing::{debug, instrument, trace, warn};

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

        let receiver_task_join_handle = tokio::spawn(OutputReceiverTask::new(receiver).run());

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
            .context("receiver_task_join_handle.await error")?;

        Ok(())
    }
}

struct OutputReceiverTask {
    receiver: Receiver<Output>,
}

impl OutputReceiverTask {
    fn new(receiver: Receiver<Output>) -> Self {
        Self { receiver }
    }

    #[instrument(skip_all, name = "OutputReceiverTask::run", level = "debug")]
    async fn run(self) {
        debug!("begin run");

        async fn copy(mut buffer: &[u8], output_stream: &mut (impl AsyncWrite + Unpin)) {
            let result = tokio::io::copy(&mut buffer, &mut *output_stream).await;
            trace!("copy result = {:?}", result);
        }

        let mut stdout = tokio::io::stdout();
        let mut stderr = tokio::io::stderr();

        let mut receiver = self.receiver;

        while let Some(command_output) = receiver.recv().await {
            if !command_output.stdout.is_empty() {
                copy(&command_output.stdout, &mut stdout).await;
            }
            if !command_output.stderr.is_empty() {
                copy(&command_output.stderr, &mut stderr).await;
            }
        }

        debug!("end run");
    }
}
