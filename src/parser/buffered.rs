use itertools::Itertools;

use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, parser::ShellCommandAndArgs,
};

pub struct BufferedInputLineParser {
    split_whitespace: bool,
    shell_command_and_args: ShellCommandAndArgs,
    prepend_command_and_args: Vec<String>,
}

impl BufferedInputLineParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let split_whitespace = !command_line_args.null_separator;

        let prepend_command_and_args = command_line_args.command_and_initial_arguments.clone();

        let shell_command_and_args = super::build_shell_command_and_args(command_line_args);

        Self {
            split_whitespace,
            shell_command_and_args,
            prepend_command_and_args,
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
        let mut vec = if self.split_whitespace {
            input_line.split_whitespace().map_into().collect()
        } else {
            vec![input_line.to_owned()]
        };

        if !self.prepend_command_and_args.is_empty() {
            vec = [self.prepend_command_and_args.clone(), vec].concat();
        }

        super::build_owned_command_and_args(&self.shell_command_and_args, vec)
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

        let parser = BufferedInputLineParser::new(&command_line_args);

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

        let parser = BufferedInputLineParser::new(&command_line_args);

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

        let parser = BufferedInputLineParser::new(&command_line_args);

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

        let parser = BufferedInputLineParser::new(&command_line_args);

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

        let parser = BufferedInputLineParser::new(&command_line_args);

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
}
