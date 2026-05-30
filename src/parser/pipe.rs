use crate::command_line_args::CommandLineArgs;

use std::sync::Mutex;

use super::ParsedCommand;

pub struct PipeModeParser {
    block_size_bytes: usize,
    command_and_initial_arguments: Vec<String>,
    buffered_data: Mutex<String>,
}

impl PipeModeParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let command_and_initial_arguments = command_line_args.command_and_initial_arguments.clone();

        let block_size_bytes = command_line_args.block_size;

        Self {
            block_size_bytes,
            command_and_initial_arguments,
            buffered_data: Mutex::new(String::with_capacity(block_size_bytes)),
        }
    }

    pub fn parse_segment(&self, segment: Vec<u8>) -> Option<ParsedCommand> {
        if let Ok(input_line) = std::str::from_utf8(&segment) {
            self.parse_line(input_line)
        } else {
            None
        }
    }

    fn parse_line(&self, input_line: &str) -> Option<ParsedCommand> {
        let mut buffered_data = self.buffered_data.lock().unwrap();
        buffered_data.push_str(input_line);
        buffered_data.push('\n');

        if buffered_data.len() < self.block_size_bytes {
            None
        } else {
            let stdin = buffered_data.clone();
            buffered_data.clear();

            Some(ParsedCommand::new(self.command_and_initial_arguments.clone()).with_stdin(stdin))
        }
    }

    pub fn parse_last_command(&self) -> Option<ParsedCommand> {
        let mut buffered_data = self.buffered_data.lock().unwrap();

        if !buffered_data.is_empty() {
            let stdin = buffered_data.clone();
            buffered_data.clear();

            Some(ParsedCommand::new(self.command_and_initial_arguments.clone()).with_stdin(stdin))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::common::OwnedCommandAndArgs;
    use crate::parser::CommandBuilder;

    use std::{default::Default, path::PathBuf};

    #[test]
    fn test_two_segments_last_command_only() {
        let command_line_args = CommandLineArgs {
            command_and_initial_arguments: vec!["echo".to_string(), "hello".to_string()],
            block_size: 100,
            ..Default::default()
        };

        let parser = PipeModeParser::new(&command_line_args);
        let builder = CommandBuilder::new(&command_line_args);

        let segment1 = b"Hello, World!".to_vec();
        let segment2 = b"This is a test.".to_vec();

        assert!(parser.parse_segment(segment1).is_none());
        assert!(parser.parse_segment(segment2).is_none());

        let owned_command_and_args = builder.build(parser.parse_last_command().unwrap()).unwrap();
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
        let builder = CommandBuilder::new(&command_line_args);

        let segment1 = b"Hello, World!".to_vec();
        let segment2 = b"This is a test.".to_vec();

        assert!(parser.parse_segment(segment1).is_none());

        let owned_command_and_args = builder
            .build(parser.parse_segment(segment2).unwrap())
            .unwrap();
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
        let builder = CommandBuilder::new(&command_line_args);

        let segment1 = b"Hello, World!".to_vec();
        let segment2 = b"This is a test.".to_vec();

        let owned_command_and_args1 = builder
            .build(parser.parse_segment(segment1).unwrap())
            .unwrap();
        assert_eq!(owned_command_and_args1.command_path, PathBuf::from("echo"));
        assert_eq!(owned_command_and_args1.args, vec!["hello".to_string()]);
        assert_eq!(
            owned_command_and_args1.stdin.0.unwrap().as_str(),
            "Hello, World!\n"
        );

        let owned_command_and_args2 = builder
            .build(parser.parse_segment(segment2).unwrap())
            .unwrap();
        assert_eq!(owned_command_and_args2.command_path, PathBuf::from("echo"));
        assert_eq!(owned_command_and_args2.args, vec!["hello".to_string()]);
        assert_eq!(
            owned_command_and_args2.stdin.0.unwrap().as_str(),
            "This is a test.\n"
        );

        assert!(parser.parse_last_command().is_none());
    }
}
