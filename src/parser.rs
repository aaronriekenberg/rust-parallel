use crate::command_line_args::CommandLineArgs;

use tracing::debug;

const DEFAULT_SHELL: &str = "/bin/bash";

pub struct InputLineParser {
    split_whitespace: bool,
    prepend_command_and_args: Vec<String>,
}

impl InputLineParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let split_whitespace = if command_line_args.null_separator || command_line_args.shell {
            false
        } else {
            true
        };

        let mut prepend_command_and_args = vec![];

        if command_line_args.command_and_initial_arguments.len() > 0 {
            prepend_command_and_args = command_line_args.command_and_initial_arguments.clone();
        }

        if command_line_args.shell {
            let shell = match std::env::var("SHELL") {
                Ok(shell) => {
                    debug!("using $SHELL from environment: '{}'", shell);
                    shell
                }
                Err(_) => {
                    debug!("using default shell '{}'", DEFAULT_SHELL);
                    DEFAULT_SHELL.to_owned()
                }
            };
            let shell_command_and_args = vec![shell, "-c".to_owned()];
            prepend_command_and_args = [shell_command_and_args, prepend_command_and_args].concat();
        }

        Self {
            split_whitespace,
            prepend_command_and_args,
        }
    }

    fn get_prepend_command_and_args(&self) -> Vec<&str> {
        self.prepend_command_and_args
            .iter()
            .map(|s| s.as_ref())
            .collect()
    }

    pub fn parse_line<'a>(&'a self, input_line: &'a str) -> Option<Vec<&'a str>> {
        let mut vec = if self.split_whitespace {
            input_line.split_whitespace().collect()
        } else {
            vec![input_line]
        };

        if self.prepend_command_and_args.len() > 0 {
            vec = [self.get_prepend_command_and_args(), vec].concat();
        }

        if vec.is_empty() {
            None
        } else {
            Some(vec)
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

        let parser = InputLineParser::new(&command_line_args);

        let result = parser.parse_line("echo hi there");

        assert_eq!(result, Some(vec!["echo", "hi", "there"]));

        let result = parser.parse_line(" echo  hi    there  ");

        assert_eq!(result, Some(vec!["echo", "hi", "there"]));

        let result = parser.parse_line(" /bin/echo ");

        assert_eq!(result, Some(vec!["/bin/echo"]));

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

        let parser = InputLineParser::new(&command_line_args);

        let result = parser.parse_line("file with spaces");

        assert_eq!(result, Some(vec!["gzip", "-k", "file with spaces"]));
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

        let parser = InputLineParser::new(&command_line_args);

        let result = parser.parse_line("awesomebashfunction 1 2 3");

        assert_eq!(
            result,
            Some(vec!["/bin/bash", "-c", "awesomebashfunction 1 2 3"]),
        );

        std::env::set_var("SHELL", "/bin/zsh");

        let parser = InputLineParser::new(&command_line_args);

        let result = parser.parse_line(" awesomebashfunction 1 2 3 ");

        assert_eq!(
            result,
            Some(vec!["/bin/zsh", "-c", " awesomebashfunction 1 2 3 "]),
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

        let parser = InputLineParser::new(&command_line_args);

        let result = parser.parse_line("stuff");

        assert_eq!(result, Some(vec!["md5", "-s", "stuff"]));

        let result = parser.parse_line(" stuff things ");

        assert_eq!(result, Some(vec!["md5", "-s", "stuff", "things"]));
    }
}
