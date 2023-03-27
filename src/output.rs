use tokio::{
    io::AsyncWrite,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};

use tracing::{debug, trace, warn};

use std::process::Output;

async fn run_receiver_task(mut receiver: Receiver<Output>) {
    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();

    async fn copy<T>(mut buffer: &[u8], output_stream: &mut T)
    where
        T: AsyncWrite + Unpin,
    {
        let result = tokio::io::copy(&mut buffer, &mut *output_stream).await;
        trace!("write_command_output copy result = {:?}", result);
    }

    while let Some(command_output) = receiver.recv().await {
        if !command_output.stdout.is_empty() {
            copy(&command_output.stdout, &mut stdout).await;
        }
        if !command_output.stderr.is_empty() {
            copy(&command_output.stderr, &mut stderr).await;
        }
    }

    debug!("receiver task after loop, exiting");
}

pub struct OutputWriter {
    sender: Sender<Output>,
    receiver_task_join_handle: JoinHandle<()>,
}

impl OutputWriter {
    pub fn new() -> Self {
        let (sender, receiver) = channel::<Output>(1);
        debug!("created channel with capacity 1");

        let receiver_task_join_handle = tokio::task::spawn(run_receiver_task(receiver));

        Self {
            sender,
            receiver_task_join_handle,
        }
    }

    pub fn sender(&self) -> Sender<Output> {
        self.sender.clone()
    }

    pub async fn wait_for_completion(self) {
        drop(self.sender);

        if let Err(e) = self.receiver_task_join_handle.await {
            warn!("receiver_task_join_handle.await error: {}", e);
        }
    }
}
