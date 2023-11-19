use itertools::Itertools;

use crate::{
    command_line_args::CommandLineArgs,
    common::OwnedCommandAndArgs,
    parser::{regex::RegexProcessor, ShellCommandAndArgs},
};

pub struct BufferedInputLineParser {
    split_whitespace: bool,
    shell_command_and_args: ShellCommandAndArgs,
    command_and_initial_arguments: Vec<String>,
    regex_processor: RegexProcessor,
}

impl BufferedInputLineParser {
    pub fn new(command_line_args: &CommandLineArgs, regex_processor: RegexProcessor) -> Self {
        let split_whitespace = !command_line_args.null_separator;

        let command_and_initial_arguments = command_line_args.command_and_initial_arguments.clone();

        let shell_command_and_args = ShellCommandAndArgs::new(command_line_args);

        Self {
            split_whitespace,
            shell_command_and_args,
            command_and_initial_arguments,
            regex_processor,
        }
    }

    pub fn parse_segment(&self, segment: Vec<u8>) -> Option<OwnedCommandAndArgs> {
        if let Ok(input_line) = std::str::from_utf8(&segment) {
            self.parse_line(input_line)
        } else {
            None
        }
    }

    pub fn parse_line(&self, input_line: &str) -> Option<OwnedCommandAndArgs> {
        let cmd_and_args = if !self.regex_processor.regex_mode() {
            let mut cmd_and_args = if self.split_whitespace {
                input_line.split_whitespace().map_into().collect()
            } else {
                vec![input_line.into()]
            };

            if !self.command_and_initial_arguments.is_empty() {
                cmd_and_args = [self.command_and_initial_arguments.clone(), cmd_and_args].concat();
            }

            cmd_and_args
        } else {
            self.command_and_initial_arguments
                .iter()
                .map(|arg| self.regex_processor.process_string(arg, input_line).into())
                .collect_vec()
        };

        super::build_owned_command_and_args(&self.shell_command_and_args, cmd_and_args)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{default::Default, path::PathBuf};

    #[test]
    fn test_split_whitespace() {
        let command_line_args = CommandLineArgs {
            null_separator: false,
            shell: false,
            command_and_initial_arguments: vec![],
            ..Default::default()
        };

        let parser = BufferedInputLineParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = parser.parse_line("echo hi there");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("echo"),
                args: vec!["hi", "there"].into_iter().map_into().collect(),
            })
        );

        let result = parser.parse_line(" echo  hi    there  ");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("echo"),
                args: vec!["hi", "there"].into_iter().map_into().collect(),
            })
        );

        let result = parser.parse_line(" /bin/echo ");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("/bin/echo"),
                args: vec![],
            })
        );

        let result = parser.parse_line("");

        assert_eq!(result, None);
    }

    #[test]
    fn test_null_separator() {
        let command_line_args = CommandLineArgs {
            null_separator: true,
            shell: false,
            command_and_initial_arguments: vec!["gzip".to_owned(), "-k".to_owned()],
            ..Default::default()
        };

        let parser = BufferedInputLineParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = parser.parse_line("file with spaces");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("gzip"),
                args: vec!["-k", "file with spaces"]
                    .into_iter()
                    .map_into()
                    .collect(),
            })
        );
    }

    #[test]
    fn test_shell() {
        let command_line_args = CommandLineArgs {
            null_separator: false,
            shell: true,
            command_and_initial_arguments: vec![],
            shell_path: "/bin/bash".to_owned(),
            ..Default::default()
        };

        let parser = BufferedInputLineParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = parser.parse_line("awesomebashfunction 1 2 3");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("/bin/bash"),
                args: vec!["-c", "awesomebashfunction 1 2 3"]
                    .into_iter()
                    .map_into()
                    .collect(),
            })
        );

        let command_line_args = CommandLineArgs {
            null_separator: false,
            shell: true,
            command_and_initial_arguments: vec![],
            shell_path: "/bin/zsh".to_owned(),
            ..Default::default()
        };

        let parser = BufferedInputLineParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = parser.parse_line(" awesomebashfunction 1 2 3 ");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("/bin/zsh"),
                args: vec!["-c", "awesomebashfunction 1 2 3"]
                    .into_iter()
                    .map_into()
                    .collect(),
            })
        );
    }

    #[test]
    fn test_command_and_initial_arguments() {
        let command_line_args = CommandLineArgs {
            null_separator: false,
            shell: false,
            command_and_initial_arguments: vec!["md5".to_owned(), "-s".to_owned()],
            ..Default::default()
        };

        let parser = BufferedInputLineParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = parser.parse_line("stuff");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("md5"),
                args: vec!["-s", "stuff"].into_iter().map_into().collect(),
            })
        );

        let result = parser.parse_line(" stuff things ");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("md5"),
                args: vec!["-s", "stuff", "things"]
                    .into_iter()
                    .map_into()
                    .collect(),
            })
        );
    }

    #[test]
    fn test_regex_named_groups() {
        let command_line_args = CommandLineArgs {
            command_and_initial_arguments: vec![
                "echo".to_owned(),
                "got arg1={arg1} arg2={arg2}".to_owned(),
            ],
            regex: Some("(?P<arg1>.*),(?P<arg2>.*)".to_owned()),
            ..Default::default()
        };

        let parser = BufferedInputLineParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = parser.parse_line("foo,bar");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("echo"),
                args: vec!["got arg1=foo arg2=bar"]
                    .into_iter()
                    .map_into()
                    .collect(),
            })
        );
    }

    #[test]
    fn test_regex_numbered_groups() {
        let command_line_args = CommandLineArgs {
            command_and_initial_arguments: vec![
                "echo".to_owned(),
                "got arg1={2} arg2={1} arg3={0}".to_owned(),
            ],
            regex: Some("(.*),(.*)".to_owned()),
            ..Default::default()
        };

        let parser = BufferedInputLineParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = parser.parse_line("foo,bar");

        assert_eq!(
            result,
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from("echo"),
                args: vec!["got arg1=bar arg2=foo arg3=foo,bar"]
                    .into_iter()
                    .map_into()
                    .collect(),
            })
        );
    }
}
