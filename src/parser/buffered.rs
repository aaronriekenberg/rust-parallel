use tracing::debug;

use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs};

pub struct BufferedInputLineParser {
    split_whitespace: bool,
    prepend_command_and_args: Vec<String>,
}

impl BufferedInputLineParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        debug!("begin BufferedInputLineParser::new");

        let split_whitespace = !(command_line_args.null_separator || command_line_args.shell);

        let mut prepend_command_and_args = command_line_args.command_and_initial_arguments.clone();

        if command_line_args.shell {
            let shell_command_and_args =
                vec![command_line_args.shell_path.clone(), "-c".to_owned()];
            prepend_command_and_args = [shell_command_and_args, prepend_command_and_args].concat();
        }

        Self {
            split_whitespace,
            prepend_command_and_args,
        }
    }

    fn prepend_command_and_args(&self) -> Vec<String> {
        self.prepend_command_and_args
            .iter()
            .map(|s| s.to_owned())
            .collect()
    }

    pub fn parse_segment(&self, segment: Vec<u8>) -> Option<OwnedCommandAndArgs> {
        if let Ok(input_line) = std::str::from_utf8(&segment) {
            self.parse_line(input_line)
        } else {
            None
        }
    }

    pub fn parse_line(&self, input_line: &str) -> Option<OwnedCommandAndArgs> {
        let mut vec: Vec<String> = if self.split_whitespace {
            input_line
                .split_whitespace()
                .map(|s| s.to_owned())
                .collect()
        } else {
            vec![input_line.to_owned()]
        };

        if !self.prepend_command_and_args.is_empty() {
            vec = [self.prepend_command_and_args(), vec].concat();
        }

        if vec.is_empty() {
            None
        } else {
            Some(vec.into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::default::Default;

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

        assert_eq!(result, Some(vec!["echo", "hi", "there"].into()));

        let result = parser.parse_line(" echo  hi    there  ");

        assert_eq!(result, Some(vec!["echo", "hi", "there"].into()));

        let result = parser.parse_line(" /bin/echo ");

        assert_eq!(result, Some(vec!["/bin/echo"].into()));

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

        assert_eq!(result, Some(vec!["gzip", "-k", "file with spaces"].into()));
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
            Some(vec!["/bin/bash", "-c", "awesomebashfunction 1 2 3"].into()),
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
            Some(vec!["/bin/zsh", "-c", " awesomebashfunction 1 2 3 "].into()),
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

        assert_eq!(result, Some(vec!["md5", "-s", "stuff"].into()));

        let result = parser.parse_line(" stuff things ");

        assert_eq!(result, Some(vec!["md5", "-s", "stuff", "things"].into()));
    }
}
