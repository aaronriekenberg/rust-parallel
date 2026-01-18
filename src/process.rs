use tokio::{
    io::AsyncWriteExt,
    process::{Child, Command},
    time::Duration,
};

use tracing::debug;

use std::{
    ffi::OsStr,
    process::{Output, Stdio},
};

use crate::command_line_args::{CommandLineArgs, DiscardOutput};

#[derive(thiserror::Error, Debug)]
pub enum ChildProcessExecutionError {
    #[error("timeout: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error("i/o error: {0}")]
    IOError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct ChildProcess {
    child: Child,
    discard_all_output: bool,
    timeout: Option<Duration>,
    stdin_option: Option<String>,
}

impl ChildProcess {
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    async fn await_output(mut self) -> Result<Output, ChildProcessExecutionError> {
        let output = if self.discard_all_output {
            Output {
                status: self.child.wait().await?,
                stdout: vec![],
                stderr: vec![],
            }
        } else {
            self.child.wait_with_output().await?
        };

        Ok(output)
    }

    pub async fn await_completion(mut self) -> Result<Output, ChildProcessExecutionError> {
        let _stdin_task_join_handle = if self.stdin_option.is_some()
            && let Some(mut child_stdin) = self.child.stdin.take()
            && let Some(stdin_data) = self.stdin_option.take()
        {
            debug!(
                "writing to child stdin, stdin_data.len={}",
                stdin_data.len()
            );
            Some(tokio::spawn(async move {
                child_stdin.write_all(stdin_data.as_bytes()).await
            }))
        } else {
            None
        };

        // TODO wait for stdin task to complete before awaiting output, handle errors
        match self.timeout {
            None => self.await_output().await,
            Some(timeout) => {
                let result = tokio::time::timeout(timeout, self.await_output()).await?;

                let output = result?;

                Ok(output)
            }
        }
    }
}

#[derive(Debug)]
pub struct ChildProcessFactory {
    discard_stdout: bool,
    discard_stderr: bool,
    timeout: Option<Duration>,
}

impl ChildProcessFactory {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        Self {
            discard_stdout: matches!(
                command_line_args.discard_output,
                Some(DiscardOutput::All) | Some(DiscardOutput::Stdout)
            ),
            discard_stderr: matches!(
                command_line_args.discard_output,
                Some(DiscardOutput::All) | Some(DiscardOutput::Stderr)
            ),
            timeout: command_line_args
                .timeout_seconds
                .map(Duration::from_secs_f64),
        }
    }

    fn stdout(&self) -> Stdio {
        if self.discard_stdout {
            Stdio::null()
        } else {
            Stdio::piped()
        }
    }

    fn stderr(&self) -> Stdio {
        if self.discard_stderr {
            Stdio::null()
        } else {
            Stdio::piped()
        }
    }

    fn discard_all_output(&self) -> bool {
        self.discard_stdout && self.discard_stderr
    }

    pub async fn spawn<C, AI, A>(
        &self,
        command: C,
        args: AI,
        stdin_option: &Option<String>,
    ) -> std::io::Result<ChildProcess>
    where
        C: AsRef<OsStr>,
        AI: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let child_stdin = match stdin_option {
            Some(_) => Stdio::piped(),
            None => Stdio::null(),
        };

        let child = Command::new(command)
            .args(args)
            .stdin(child_stdin)
            .stdout(self.stdout())
            .stderr(self.stderr())
            .kill_on_drop(self.timeout.is_some())
            .spawn()?;

        Ok(ChildProcess {
            child,
            discard_all_output: self.discard_all_output(),
            timeout: self.timeout,
            stdin_option: stdin_option.clone(),
        })
    }
}
