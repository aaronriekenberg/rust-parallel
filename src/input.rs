use anyhow::Context;

use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, BufReader, Split},
    sync::mpsc::Sender,
    task::JoinHandle,
};

use tracing::{debug, warn};

use crate::{command_line_args, parser::InputLineParser};

#[derive(Debug, Clone, Copy)]
pub enum Input {
    Stdin,

    File { file_name: &'static str },
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdin => write!(f, "stdin"),
            Self::File { file_name } => write!(f, "{}", file_name),
        }
    }
}

#[derive(Debug)]
pub struct InputLineNumber {
    pub input: Input,
    pub line_number: u64,
}

impl std::fmt::Display for InputLineNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.input, self.line_number)
    }
}

fn build_input_list() -> Vec<Input> {
    let command_line_args = command_line_args::instance();
    if command_line_args.input.is_empty() {
        vec![Input::Stdin]
    } else {
        command_line_args
            .input
            .iter()
            .map(|input_name| {
                if input_name == "-" {
                    Input::Stdin
                } else {
                    Input::File {
                        file_name: input_name,
                    }
                }
            })
            .collect()
    }
}

type AsyncBufReadBox = Box<dyn AsyncBufRead + Unpin + Send>;

pub struct InputReader {
    input: Input,
    split: Split<AsyncBufReadBox>,
    next_line_number: u64,
}

impl InputReader {
    pub async fn new(input: Input) -> anyhow::Result<Self> {
        let command_line_args = command_line_args::instance();

        let buf_reader = Self::create_buf_reader(input).await?;

        let line_separator = if command_line_args.null_separator {
            0u8
        } else {
            b'\n'
        };

        let split = buf_reader.split(line_separator);

        Ok(InputReader {
            input,
            split,
            next_line_number: 0,
        })
    }

    async fn create_buf_reader(input: Input) -> anyhow::Result<AsyncBufReadBox> {
        match input {
            Input::Stdin => {
                let buf_reader = BufReader::new(tokio::io::stdin());

                Ok(Box::new(buf_reader))
            }
            Input::File { file_name } => {
                let file = tokio::fs::File::open(file_name).await.with_context(|| {
                    format!("error opening input file file_name = '{}'", file_name)
                })?;
                let buf_reader = BufReader::new(file);

                Ok(Box::new(buf_reader))
            }
        }
    }

    pub async fn next_segment(&mut self) -> anyhow::Result<Option<(InputLineNumber, Vec<u8>)>> {
        let segment = self.split.next_segment().await?;

        match segment {
            None => Ok(None),
            Some(segment) => {
                self.next_line_number += 1;

                let input_line_number = InputLineNumber {
                    input: self.input,
                    line_number: self.next_line_number,
                };

                Ok(Some((input_line_number, segment)))
            }
        }
    }
}

#[derive(Debug)]
pub struct InputMessage {
    pub command_and_args: Vec<String>,
    pub input_line_number: InputLineNumber,
}

pub struct InputProducer {
    sender_task_join_handle: JoinHandle<()>,
}

impl InputProducer {
    pub fn new(input_line_parser: InputLineParser, sender: Sender<InputMessage>) -> Self {
        let sender_task_join_handle =
            tokio::spawn(InputSender::new(sender, input_line_parser).run());

        Self {
            sender_task_join_handle,
        }
    }

    pub async fn wait_for_completion(self) -> anyhow::Result<()> {
        self.sender_task_join_handle
            .await
            .context("sender_task_join_handle.await error")?;

        Ok(())
    }
}

struct InputSender {
    sender: Sender<InputMessage>,
    input_line_parser: InputLineParser,
}

impl InputSender {
    fn new(sender: Sender<InputMessage>, input_line_parser: InputLineParser) -> Self {
        Self {
            sender,
            input_line_parser,
        }
    }

    async fn run(self) {
        debug!("begin InputSender.run");

        let inputs = build_input_list();
        for input in inputs {
            debug!("processing input {}", input);
            let mut input_reader = match InputReader::new(input).await {
                Ok(input_reader) => input_reader,
                Err(e) => {
                    warn!("InputReader::new error input = {}: {}", input, e);
                    continue;
                }
            };

            loop {
                match input_reader.next_segment().await {
                    Ok(Some((input_line_number, segment))) => {
                        let Ok(input_line) = std::str::from_utf8(&segment) else {
                            continue;
                        };

                        if let Some(command_and_args) =
                            self.input_line_parser.parse_line(input_line)
                        {
                            let input_message = InputMessage {
                                command_and_args: command_and_args
                                    .into_iter()
                                    .map(|s| s.to_owned())
                                    .collect(),
                                input_line_number,
                            };
                            if let Err(e) = self.sender.send(input_message).await {
                                warn!("input sender send error: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("input_reader.next_segment EOF");
                        break;
                    }
                    Err(e) => {
                        warn!("input_reader.next_segment error: {}", e);
                        break;
                    }
                }
            }
        }

        debug!("end InputSender.run");
    }
}
