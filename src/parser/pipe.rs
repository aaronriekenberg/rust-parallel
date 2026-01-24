use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, parser::ShellCommandAndArgs,
};

use tracing::trace;

use std::cell::RefCell;

const BLOCK_SIZE_BYTES: usize = 1_024 * 1_024; // 1 MB

pub struct PipeModeParser {
    // split_whitespace: bool,
    shell_command_and_args: ShellCommandAndArgs,
    command_and_initial_arguments: Vec<String>,
    buffered_data: RefCell<String>,
}

impl PipeModeParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        // let split_whitespace = !command_line_args.null_separator;

        let command_and_initial_arguments = command_line_args.command_and_initial_arguments.clone();

        let shell_command_and_args = ShellCommandAndArgs::new(command_line_args);

        Self {
            // split_whitespace,
            shell_command_and_args,
            command_and_initial_arguments,
            buffered_data: RefCell::new(String::with_capacity(BLOCK_SIZE_BYTES)),
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

        if buffered_data.len() < BLOCK_SIZE_BYTES {
            trace!(
                "buffered_data length {} is less than BLOCK_SIZE_BYTES {}, continuing to buffer",
                buffered_data.len(),
                BLOCK_SIZE_BYTES
            );
            None
        } else {
            drop(buffered_data); // Release the borrow
            let stdin = self
                .buffered_data
                .replace(String::with_capacity(BLOCK_SIZE_BYTES));

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
        .map(|owned_command_and_args| owned_command_and_args.with_stdin(stdin))
    }
}
