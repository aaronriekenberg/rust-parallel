use tokio::{
    io::{AsyncWrite, Stderr, Stdout},
    sync::Mutex,
};

use tracing::trace;

use std::{process::Output, sync::Arc};

pub struct OutputWriter {
    stdout: Mutex<Stdout>,
    stderr: Mutex<Stderr>,
}

impl OutputWriter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            stdout: Mutex::new(tokio::io::stdout()),
            stderr: Mutex::new(tokio::io::stderr()),
        })
    }

    pub async fn write_command_output(&self, command_output: &Output) {
        async fn write(mut buffer: &[u8], output_stream_mutex: &Mutex<impl AsyncWrite + Unpin>) {
            let mut output_stream = output_stream_mutex.lock().await;

            let result = tokio::io::copy(&mut buffer, &mut *output_stream).await;
            trace!("write_command_output copy result = {:?}", result);
        }

        if !command_output.stdout.is_empty() {
            write(&command_output.stdout, &self.stdout).await;
        }
        if !command_output.stderr.is_empty() {
            write(&command_output.stderr, &self.stderr).await;
        }
    }
}
