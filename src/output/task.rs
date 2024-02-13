use tokio::{io::AsyncWrite, sync::mpsc::Receiver};

use tracing::{debug, error, instrument, trace};

use crate::command_line_args::CommandLineArgs;

use super::OutputMessage;

pub struct OutputTask {
    receiver: Receiver<OutputMessage>,
    exit_on_error: bool,
}

impl OutputTask {
    pub fn new(receiver: Receiver<OutputMessage>, command_line_args: &CommandLineArgs) -> Self {
        Self {
            receiver,
            exit_on_error: command_line_args.exit_on_error,
        }
    }

    #[instrument(skip_all, name = "OutputTask::run", level = "debug")]
    pub async fn run(self) {
        debug!("begin run");

        async fn copy(mut buffer: &[u8], output_stream: &mut (impl AsyncWrite + Unpin)) {
            let result = tokio::io::copy(&mut buffer, &mut *output_stream).await;
            trace!("copy result = {:?}", result);
        }

        let mut stdout = tokio::io::stdout();
        let mut stderr = tokio::io::stderr();

        let mut receiver = self.receiver;

        while let Some(output_message) = receiver.recv().await {
            if !output_message.stdout.is_empty() {
                copy(&output_message.stdout, &mut stdout).await;
            }
            if !output_message.stderr.is_empty() {
                copy(&output_message.stderr, &mut stderr).await;
            }
            if !output_message.exit_status.success() {
                error!(
                    "command failed: {} line={} exit_status={}",
                    output_message.command_and_args,
                    output_message.input_line_number,
                    output_message.exit_status.code().unwrap_or_default(),
                );

                if self.exit_on_error {
                    std::process::exit(1);
                }
            }
        }

        debug!("end run");
    }
}
