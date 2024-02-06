use tokio::{io::AsyncWrite, sync::mpsc::Receiver};

use tracing::{debug, error, instrument, trace};

use std::process::Output;

pub struct OutputTask {
    receiver: Receiver<Output>,
}

impl OutputTask {
    pub fn new(receiver: Receiver<Output>) -> Self {
        Self { receiver }
    }

    #[instrument(skip_all, name = "OutputTask::run", level = "debug")]
    pub async fn run(self) -> usize {
        debug!("begin run");

        async fn copy(mut buffer: &[u8], output_stream: &mut (impl AsyncWrite + Unpin)) {
            let result = tokio::io::copy(&mut buffer, &mut *output_stream).await;
            trace!("copy result = {:?}", result);
        }

        let mut stdout = tokio::io::stdout();
        let mut stderr = tokio::io::stderr();

        let mut receiver = self.receiver;

        let mut failed = 0;
        while let Some(command_output) = receiver.recv().await {
            if !command_output.stdout.is_empty() {
                copy(&command_output.stdout, &mut stdout).await;
            }
            if !command_output.stderr.is_empty() {
                copy(&command_output.stderr, &mut stderr).await;
            }
            if !command_output.status.success() {
                error!("command failed: {:?}", command_output.status);
                failed += 1;
            }
        }

        debug!("end run");
        failed
    }
}
