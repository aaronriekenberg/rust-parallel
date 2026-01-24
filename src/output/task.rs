use tokio::{io::AsyncWrite, sync::mpsc::Receiver};

use tracing::{debug, error, instrument, trace};

use std::collections::BTreeMap;

use crate::input::LineNumberOrRange;

use super::OutputMessage;

pub struct OutputTask {
    receiver: Receiver<OutputMessage>,
    keep_order: bool,
}

impl OutputTask {
    pub fn new(receiver: Receiver<OutputMessage>, keep_order: bool) -> Self {
        Self {
            receiver,
            keep_order,
        }
    }

    #[instrument(skip_all, name = "OutputTask::run", level = "debug")]
    pub async fn run(self) {
        debug!("begin run");

        async fn copy(mut buffer: &[u8], output_stream: &mut (impl AsyncWrite + Unpin)) {
            let result = tokio::io::copy(&mut buffer, &mut *output_stream).await;
            trace!("copy result = {:?}", result);
        }

        async fn process_output_message(
            output_message: OutputMessage,
            stdout: &mut (impl AsyncWrite + Unpin),
            stderr: &mut (impl AsyncWrite + Unpin),
        ) {
            if !output_message.stdout.is_empty() {
                copy(&output_message.stdout, stdout).await;
            }
            if !output_message.stderr.is_empty() {
                copy(&output_message.stderr, stderr).await;
            }
            if !output_message.exit_status.success() {
                error!(
                    "command failed: {},line={} exit_status={}",
                    output_message.command_and_args,
                    output_message.input_line_number,
                    output_message.exit_status.code().unwrap_or_default(),
                );
            }
        }

        let mut stdout = tokio::io::stdout();
        let mut stderr = tokio::io::stderr();

        let mut receiver = self.receiver;

        if self.keep_order {
            // When keep-order is enabled, buffer outputs and process them in order
            let mut buffered_outputs: BTreeMap<usize, OutputMessage> = BTreeMap::new();
            let mut next_line_number = 1;

            while let Some(output_message) = receiver.recv().await {
                let line_number = match output_message.input_line_number.line_number {
                    LineNumberOrRange::Single(n) => n,
                    LineNumberOrRange::Range(start, _) => start,
                };

                // Store the output message in the buffer
                buffered_outputs.insert(line_number, output_message);

                // Process any buffered outputs that are ready (in order)
                while let Some(output_message) = buffered_outputs.remove(&next_line_number) {
                    process_output_message(output_message, &mut stdout, &mut stderr).await;
                    next_line_number += 1;
                }
            }

            // Process any remaining buffered outputs
            for (_, output_message) in buffered_outputs.into_iter() {
                process_output_message(output_message, &mut stdout, &mut stderr).await;
            }
        } else {
            // When keep-order is disabled, process outputs as they arrive (original behavior)
            while let Some(output_message) = receiver.recv().await {
                process_output_message(output_message, &mut stdout, &mut stderr).await;
            }
        }

        debug!("end run");
    }
}
