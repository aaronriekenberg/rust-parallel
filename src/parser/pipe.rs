use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, parser::ShellCommandAndArgs,
};

use tracing::trace;

const BLOCK_SIZE_BYTES: usize = 1_024 * 1_024; // 1 MB

pub struct PipeModeParser {
    // split_whitespace: bool,
    shell_command_and_args: ShellCommandAndArgs,
    command_and_initial_arguments: Vec<String>,
    buffered_data: String,
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
            buffered_data: String::with_capacity(BLOCK_SIZE_BYTES),
        }
    }

    pub fn parse_segment(&mut self, segment: Vec<u8>) -> Option<OwnedCommandAndArgs> {
        if let Ok(input_line) = std::str::from_utf8(&segment) {
            self.parse_line(input_line)
        } else {
            None
        }
    }

    fn parse_line(&mut self, input_line: &str) -> Option<OwnedCommandAndArgs> {
        self.buffered_data.push_str(input_line);
        self.buffered_data.push('\n');

        if self.buffered_data.len() < BLOCK_SIZE_BYTES {
            trace!(
                "buffered_data length {} is less than BLOCK_SIZE_BYTES {}, continuing to buffer",
                self.buffered_data.len(),
                BLOCK_SIZE_BYTES
            );
            None
        } else {
            let stdin = self.buffered_data.clone();
            self.buffered_data.clear();

            let owned_command_and_args_option = super::build_owned_command_and_args(
                &self.shell_command_and_args,
                self.command_and_initial_arguments.clone(),
            );

            owned_command_and_args_option
                .map(|owned_command_and_args| owned_command_and_args.with_stdin(stdin))
        }
    }

    pub fn parse_last_command(self) -> Option<OwnedCommandAndArgs> {
        if !self.buffered_data.is_empty() {
            let stdin = self.buffered_data;

            let owned_command_and_args_option = super::build_owned_command_and_args(
                &self.shell_command_and_args,
                self.command_and_initial_arguments.clone(),
            );

            owned_command_and_args_option
                .map(|owned_command_and_args| owned_command_and_args.with_stdin(stdin))
        } else {
            None
        }
    }
}
