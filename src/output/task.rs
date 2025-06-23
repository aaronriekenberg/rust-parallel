use tokio::sync::mpsc::Receiver;

use tracing::{debug, error, instrument};

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::progress::Progress;

use super::OutputMessage;

pub struct OutputTask {
    receiver: Receiver<OutputMessage>,
    keep_order: bool,
    progress: Arc<Progress>,
}

impl OutputTask {
    pub fn new(receiver: Receiver<OutputMessage>, keep_order: bool, progress: Arc<Progress>) -> Self {
        Self {
            receiver,
            keep_order,
            progress,
        }
    }

    #[instrument(skip_all, name = "OutputTask::run", level = "debug")]
    pub async fn run(self) {
        debug!("begin run");

        fn process_output_message(output_message: OutputMessage, progress: &Progress) {
            if !output_message.stdout.is_empty() {
                let stdout_str = String::from_utf8_lossy(&output_message.stdout);
                for line in stdout_str.lines() {
                    progress.println(line);
                }
            }
            if !output_message.stderr.is_empty() {
                let stderr_str = String::from_utf8_lossy(&output_message.stderr);
                for line in stderr_str.lines() {
                    progress.eprintln(line);
                }
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

        let progress = &self.progress;
        let mut receiver = self.receiver;

        if self.keep_order {
            // When keep-order is enabled, buffer outputs and process them in order
            let mut buffered_outputs: BTreeMap<usize, OutputMessage> = BTreeMap::new();
            let mut next_line_number = 1;

            while let Some(output_message) = receiver.recv().await {
                let line_number = output_message.input_line_number.line_number;

                // Store the output message in the buffer
                buffered_outputs.insert(line_number, output_message);

                // Process any buffered outputs that are ready (in order)
                while let Some(output_message) = buffered_outputs.remove(&next_line_number) {
                    process_output_message(output_message, progress);
                    next_line_number += 1;
                }
            }

            // Process any remaining buffered outputs
            for (_, output_message) in buffered_outputs.into_iter() {
                process_output_message(output_message, progress);
            }
        } else {
            // When keep-order is disabled, process outputs as they arrive (original behavior)
            while let Some(output_message) = receiver.recv().await {
                process_output_message(output_message, progress);
            }
        }

        debug!("end run");
    }
}