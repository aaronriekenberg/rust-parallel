use crate::command_line_args::CommandLineArgs;

pub type CommandAndArgs = Vec<String>;

pub struct InputLineParser {
    split_whitespace: bool,
    prepend_command_and_args: Vec<String>,
}

impl InputLineParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let split_whitespace = !(command_line_args.null_separator || command_line_args.shell);

        let mut prepend_command_and_args: Vec<String> = Vec::new();

        if command_line_args.command_and_initial_arguments.len() > 0 {
            prepend_command_and_args = command_line_args.command_and_initial_arguments.clone();
        }

        if command_line_args.shell {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
            let shell_command_and_args = vec![shell, "-c".to_owned()];
            prepend_command_and_args = [shell_command_and_args, prepend_command_and_args].concat();
        }

        Self {
            split_whitespace,
            prepend_command_and_args,
        }
    }

    pub fn parse_line(&self, input_line: String) -> Option<CommandAndArgs> {
        let mut vec = if self.split_whitespace {
            input_line
                .split_whitespace()
                .map(|s| s.to_owned())
                .collect()
        } else {
            vec![input_line]
        };

        if self.prepend_command_and_args.len() > 0 {
            vec = [self.prepend_command_and_args.clone(), vec].concat();
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

    #[test]
    fn test_route_key_equality() {
        let command_line_args = CommandLineArgs {
            input: vec![],
            jobs: 1,
            null_separator: false,
            shell: false,
            command_and_initial_arguments: vec![],
        };

        let parser = InputLineParser::new(&command_line_args);

        let result = parser.parse_line("echo hi there".to_owned());

        assert_eq!(
            result,
            Some(vec!["echo".to_owned(), "hi".to_owned(), "there".to_owned()],),
        )
    }
}
