use itertools::Itertools;

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

pub struct CommandLineArgsParser {
    command_and_initial_arguments: Vec<String>,
    shell_enabled: bool,
    shell_command_and_args: Vec<String>,
}

impl CommandLineArgsParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let shell_command_and_args = if command_line_args.shell {
            vec![command_line_args.shell_path.clone(), "-c".to_owned()]
        } else {
            vec![]
        };

        Self {
            command_and_initial_arguments: command_line_args.command_and_initial_arguments.clone(),
            shell_enabled: command_line_args.shell,
            shell_command_and_args,
        }
    }

    fn build_split_commands(&self) -> Vec<Vec<String>> {
        let mut split_commands: Vec<Vec<String>> = vec![];

        let mut current_vec: Vec<String> = vec![];

        for string in &self.command_and_initial_arguments {
            if string == ":::" {
                if !current_vec.is_empty() {
                    split_commands.push(current_vec);
                    current_vec = vec![];
                }
            } else {
                current_vec.push(string.clone());
            }
        }

        if !current_vec.is_empty() {
            split_commands.push(current_vec);
        }

        split_commands
    }

    pub fn parse_command_line_args(&self) -> Vec<OwnedCommandAndArgs> {
        let mut split_commands = self.build_split_commands();

        debug!(
            "process_command_line_args_input split_commands = {:?}",
            split_commands
        );

        if split_commands.is_empty() {
            return vec![];
        }

        let first_command_and_args = split_commands.remove(0);

        let split_args: Vec<Vec<String>> = split_commands
            .into_iter()
            .multi_cartesian_product()
            .collect();

        debug!(
            "first_command_and_args = {:?} split_commands = {:?}",
            first_command_and_args, split_args,
        );

        let result = split_args
            .into_iter()
            .map(|args| {
                if self.shell_enabled {
                    let merged_args = [first_command_and_args.clone(), args].concat().join(" ");
                    let merged_args = vec![merged_args];
                    [self.shell_command_and_args.clone(), merged_args]
                        .concat()
                        .into()
                } else {
                    [first_command_and_args.clone(), args].concat().into()
                }
            })
            .collect();

        debug!("result = {:?}", result);

        result
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

    #[test]
    fn test_parse_command_line_args() {
        let command_line_args = CommandLineArgs {
            commands_from_args: true,
            shell: false,
            command_and_initial_arguments: vec![
                "echo", "-n", ":::", "A", "B", ":::", "C", "D", "E",
            ]
            .into_iter()
            .map(|s| s.to_owned())
            .collect(),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(&command_line_args);

        let result = parser.parse_command_line_args();

        assert_eq!(
            result,
            vec![
                vec!["echo", "-n", "A", "C"].into(),
                vec!["echo", "-n", "A", "D"].into(),
                vec!["echo", "-n", "A", "E"].into(),
                vec!["echo", "-n", "B", "C"].into(),
                vec!["echo", "-n", "B", "D"].into(),
                vec!["echo", "-n", "B", "E"].into(),
            ]
        );
    }

    #[test]
    fn test_parse_command_line_args_empty() {
        let command_line_args = CommandLineArgs {
            commands_from_args: true,
            shell: false,
            command_and_initial_arguments: vec![],
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(&command_line_args);

        let result = parser.parse_command_line_args();

        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_parse_command_line_args_shell_mode() {
        let command_line_args = CommandLineArgs {
            commands_from_args: true,
            shell: true,
            command_and_initial_arguments: vec![
                "echo", "-n", ":::", "A", "B", ":::", "C", "D", "E",
            ]
            .into_iter()
            .map(|s| s.to_owned())
            .collect(),
            shell_path: "/bin/bash".to_owned(),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(&command_line_args);

        let result = parser.parse_command_line_args();

        assert_eq!(
            result,
            vec![
                vec!["/bin/bash", "-c", "echo -n A C"].into(),
                vec!["/bin/bash", "-c", "echo -n A D"].into(),
                vec!["/bin/bash", "-c", "echo -n A E"].into(),
                vec!["/bin/bash", "-c", "echo -n B C"].into(),
                vec!["/bin/bash", "-c", "echo -n B D"].into(),
                vec!["/bin/bash", "-c", "echo -n B E"].into(),
            ]
        );
    }
}
