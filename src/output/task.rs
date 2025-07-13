use indicatif::ProgressBar;

use tokio::{io::{self, AsyncWrite}, sync::mpsc::Receiver, task, runtime::Handle};

use tracing::{debug, error, instrument, trace};

use std::{collections::BTreeMap, sync::Arc};

use super::OutputMessage;

async fn process_message(output_message: OutputMessage, progress_bar: Option<Arc<ProgressBar>>) {
    async fn copy(mut buffer: &[u8], output_stream: &mut (impl AsyncWrite + Unpin)) {
        let result = io::copy(&mut buffer, &mut *output_stream).await;
        trace!("copy result = {:?}", result);
    }

    task::spawn_blocking(move || {
        let mut stdout_local = io::stdout();
        let mut stderr_local = io::stderr();

        if let Some(pb) = progress_bar.as_ref() {
            pb.suspend(|| {
                let rt = Handle::current();
                if !output_message.stdout.is_empty() {
                    rt.block_on(copy(&output_message.stdout, &mut stdout_local));
                }
                if !output_message.stderr.is_empty() {
                    rt.block_on(copy(&output_message.stderr, &mut stderr_local));
                }
            });
        } else {
            let rt = Handle::current();
            if !output_message.stdout.is_empty() {
                rt.block_on(copy(&output_message.stdout, &mut stdout_local));
            }
            if !output_message.stderr.is_empty() {
                rt.block_on(copy(&output_message.stderr, &mut stderr_local));
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
    })
    .await
    .expect("spawn_blocking failed");
}

pub struct OutputTask {
    receiver: Receiver<OutputMessage>,
    keep_order: bool,
    progress_bar: Option<Arc<ProgressBar>>,
}

impl OutputTask {
    pub fn new(
        receiver: Receiver<OutputMessage>,
        keep_order: bool,
        progress_bar: Option<Arc<ProgressBar>>,
    ) -> Self {
        Self {
            receiver,
            keep_order,
            progress_bar,
        }
    }

    #[instrument(skip_all, name = "OutputTask::run", level = "debug")]
    pub async fn run(self) {
        debug!("begin run");

        let mut receiver = self.receiver;

        let progress_bar = self.progress_bar;

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
                    process_message(output_message, progress_bar.clone()).await;
                    next_line_number += 1;
                }
            }

            // Process any remaining buffered outputs
            for (_, output_message) in buffered_outputs.into_iter() {
                process_message(output_message, progress_bar.clone()).await;
            }
        } else {
            // When keep-order is disabled, process outputs as they arrive (original behavior)
            while let Some(output_message) = receiver.recv().await {
                process_message(output_message, progress_bar.clone()).await;
            }
        }

        debug!("end run");
    }
}
