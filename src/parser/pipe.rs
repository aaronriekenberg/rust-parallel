use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, parser::ShellCommandAndArgs,
};

use tracing::trace;

use std::{cell::RefCell, sync::Arc};

pub struct PipeModeParser {
    block_size_bytes: usize,
    shell_command_and_args: ShellCommandAndArgs,
    command_and_initial_arguments: Vec<String>,
    buffered_data: RefCell<String>,
}

impl PipeModeParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let command_and_initial_arguments = command_line_args.command_and_initial_arguments.clone();

        let shell_command_and_args = ShellCommandAndArgs::new(command_line_args);

        let block_size_bytes = command_line_args.block_size;

        Self {
            block_size_bytes,
            shell_command_and_args,
            command_and_initial_arguments,
            buffered_data: RefCell::new(String::with_capacity(block_size_bytes)),
        }
    }

    pub fn parse_segment(&self, segment: Vec<u8>) -> Option<OwnedCommandAndArgs> {
        if let Ok(input_line) = std::str::from_utf8(&segment) {
            self.parse_line(input_line)
        } else {
            None
        }
    }

    fn parse_line(&self, input_line: &str) -> Option<OwnedCommandAndArgs> {
        let mut buffered_data = self.buffered_data.borrow_mut();
        buffered_data.push_str(input_line);
        buffered_data.push('\n');

        if buffered_data.len() < self.block_size_bytes {
            trace!(
                "buffered_data length {} is less than block_size_bytes {}, continuing to buffer",
                buffered_data.len(),
                self.block_size_bytes
            );
            None
        } else {
            drop(buffered_data); // Release the borrow
            let stdin = self
                .buffered_data
                .replace(String::with_capacity(self.block_size_bytes));

            self.build_owned_command_and_args(stdin)
        }
    }

    pub fn parse_last_command(self) -> Option<OwnedCommandAndArgs> {
        if !self.buffered_data.borrow().is_empty() {
            let stdin = self.buffered_data.take();

            self.build_owned_command_and_args(stdin)
        } else {
            None
        }
    }

    fn build_owned_command_and_args(&self, stdin: String) -> Option<OwnedCommandAndArgs> {
        super::build_owned_command_and_args(
            &self.shell_command_and_args,
            self.command_and_initial_arguments.clone(),
        )
        .map(|owned_command_and_args| owned_command_and_args.with_stdin(Arc::new(stdin)))
    }
}
