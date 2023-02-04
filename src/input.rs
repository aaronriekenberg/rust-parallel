use anyhow::Context;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader, Split};

use crate::command_line_args;

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

pub fn build_input_list() -> Vec<Input> {
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

type AsyncBufReadBox = Box<dyn AsyncBufRead + Unpin>;

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
