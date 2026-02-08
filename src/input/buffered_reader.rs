use anyhow::Context;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader, Split};

use crate::command_line_args::CommandLineArgs;

use super::BufferedInput;

type AsyncBufReadBox = Box<dyn AsyncBufRead + Unpin + Send>;

pub struct BufferedInputReader {
    split: Split<AsyncBufReadBox>,
    next_line_number: usize,
}

impl BufferedInputReader {
    pub async fn new(
        buffered_input: BufferedInput,
        command_line_args: &CommandLineArgs,
    ) -> anyhow::Result<Self> {
        let buf_reader = Self::create_buf_reader(buffered_input).await?;

        let line_separator = if command_line_args.null_separator {
            0u8
        } else {
            b'\n'
        };

        let split = buf_reader.split(line_separator);

        Ok(Self {
            split,
            next_line_number: 0,
        })
    }

    async fn create_buf_reader(buffered_input: BufferedInput) -> anyhow::Result<AsyncBufReadBox> {
        match buffered_input {
            BufferedInput::Stdin => {
                let buf_reader = BufReader::new(tokio::io::stdin());

                Ok(Box::new(buf_reader))
            }
            BufferedInput::File { file_name } => {
                let file = tokio::fs::File::open(file_name).await.with_context(|| {
                    format!("error opening input file file_name = '{file_name}'")
                })?;
                let buf_reader = BufReader::new(file);

                Ok(Box::new(buf_reader))
            }
        }
    }

    pub async fn next_segment(&mut self) -> anyhow::Result<Option<(usize, Vec<u8>)>> {
        let segment = self.split.next_segment().await?;

        match segment {
            None => Ok(None),
            Some(segment) => {
                self.next_line_number += 1;

                Ok(Some((self.next_line_number, segment)))
            }
        }
    }
}
