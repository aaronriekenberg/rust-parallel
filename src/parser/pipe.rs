use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, common::StdinData,
    parser::ShellCommandAndArgs,
};

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
        .map(|owned_command_and_args| {
            owned_command_and_args.with_stdin(StdinData(Some(Arc::new(stdin))))
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{default::Default, path::PathBuf};

    #[test]
    fn test_two_segments_last_command_only() {
        let command_line_args = CommandLineArgs {
            command_and_initial_arguments: vec!["echo".to_string(), "hello".to_string()],
            block_size: 100,
            ..Default::default()
        };

        let parser = PipeModeParser::new(&command_line_args);

        let segment1 = b"Hello, World!".to_vec();
        let segment2 = b"This is a test.".to_vec();

        assert!(parser.parse_segment(segment1).is_none());
        assert!(parser.parse_segment(segment2).is_none());

        let owned_command_and_args = parser.parse_last_command().unwrap();
        assert_eq!(owned_command_and_args.command_path, PathBuf::from("echo"));
        assert_eq!(owned_command_and_args.args, vec!["hello".to_string()]);
        assert_eq!(
            owned_command_and_args.stdin.0.unwrap().as_str(),
            "Hello, World!\nThis is a test.\n"
        );
    }

    #[test]
    fn test_two_segments_one_command() {
        let command_line_args = CommandLineArgs {
            command_and_initial_arguments: vec!["echo".to_string(), "hello".to_string()],
            block_size: 20,
            ..Default::default()
        };

        let parser = PipeModeParser::new(&command_line_args);

        let segment1 = b"Hello, World!".to_vec();
        let segment2 = b"This is a test.".to_vec();

        assert!(parser.parse_segment(segment1).is_none());

        let owned_command_and_args = parser.parse_segment(segment2).unwrap();
        assert_eq!(owned_command_and_args.command_path, PathBuf::from("echo"));
        assert_eq!(owned_command_and_args.args, vec!["hello".to_string()]);
        assert_eq!(
            owned_command_and_args.stdin.0.unwrap().as_str(),
            "Hello, World!\nThis is a test.\n"
        );

        assert!(parser.parse_last_command().is_none());
    }

    #[test]
    fn test_two_segments_two_commands() {
        let command_line_args = CommandLineArgs {
            command_and_initial_arguments: vec!["echo".to_string(), "hello".to_string()],
            block_size: 10,
            ..Default::default()
        };

        let parser = PipeModeParser::new(&command_line_args);

        let segment1 = b"Hello, World!".to_vec();
        let segment2 = b"This is a test.".to_vec();

        let owned_command_and_args1 = parser.parse_segment(segment1).unwrap();
        assert_eq!(owned_command_and_args1.command_path, PathBuf::from("echo"));
        assert_eq!(owned_command_and_args1.args, vec!["hello".to_string()]);
        assert_eq!(
            owned_command_and_args1.stdin.0.unwrap().as_str(),
            "Hello, World!\n"
        );

        let owned_command_and_args2 = parser.parse_segment(segment2).unwrap();
        assert_eq!(owned_command_and_args2.command_path, PathBuf::from("echo"));
        assert_eq!(owned_command_and_args2.args, vec!["hello".to_string()]);
        assert_eq!(
            owned_command_and_args2.stdin.0.unwrap().as_str(),
            "This is a test.\n"
        );

        assert!(parser.parse_last_command().is_none());
    }
}
