use itertools::Itertools;

use tracing::debug;

use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs};

fn get_shell() -> String {
    const DEFAULT_SHELL: &str = "/bin/bash";

    match std::env::var("SHELL") {
        Ok(shell) => {
            debug!("using $SHELL from environment: '{}'", shell);
            shell
        }
        Err(_) => {
            debug!("using default shell '{}'", DEFAULT_SHELL);
            DEFAULT_SHELL.to_owned()
        }
    }
}

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
            let shell = get_shell();
            let shell_command_and_args = vec![shell, "-c".to_owned()];
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
    command_line_args: &'static CommandLineArgs,
    shell_enabled: bool,
    shell_command_and_args: Vec<String>,
}

impl CommandLineArgsParser {
    pub fn new(command_line_args: &'static CommandLineArgs) -> Self {
        let shell_command_and_args = if command_line_args.shell {
            let shell = get_shell();
            vec![shell, "-c".to_owned()]
        } else {
            vec![]
        };

        Self {
            command_line_args,
            shell_enabled: command_line_args.shell,
            shell_command_and_args,
        }
    }

    pub fn parse_command_line_args(&self) -> Vec<OwnedCommandAndArgs> {
        let mut split_commands: Vec<Vec<String>> = vec![];

        let mut current_vec: Vec<String> = vec![];

        for string in &self.command_line_args.command_and_initial_arguments {
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
            ..Default::default()
        };

        std::env::remove_var("SHELL");

        let parser = BufferedInputLineParser::new(&command_line_args);

        let result = parser.parse_line("awesomebashfunction 1 2 3");

        assert_eq!(
            result,
            Some(vec!["/bin/bash", "-c", "awesomebashfunction 1 2 3"].into()),
        );

        std::env::set_var("SHELL", "/bin/zsh");

        let parser = BufferedInputLineParser::new(&command_line_args);

        let result = parser.parse_line(" awesomebashfunction 1 2 3 ");

        assert_eq!(
            result,
            Some(vec!["/bin/zsh", "-c", " awesomebashfunction 1 2 3 "].into()),
        );

        std::env::remove_var("SHELL");
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
