use crate::command_line_args::CommandLineArgs;

pub type CommandAndArgs = Vec<String>;

pub struct InputLineParser {
    split_whitespace: bool,
    prepend_command_and_args: CommandAndArgs,
}

impl InputLineParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let split_whitespace = if command_line_args.null_separator || command_line_args.shell {
            false
        } else {
            true
        };

        let mut prepend_command_and_args = CommandAndArgs::new();

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
        } else if !input_line.is_empty() {
            vec![input_line]
        } else {
            vec![]
        };

        if vec.is_empty() {
            return None;
        }

        if self.prepend_command_and_args.len() > 0 {
            vec = [self.prepend_command_and_args.clone(), vec].concat();
        }

        Some(vec)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::default::Default;

    #[test]
    fn test_split_whitespace() {
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
            Some(vec!["echo".to_owned(), "hi".to_owned(), "there".to_owned()]),
        );

        let result = parser.parse_line(" echo  hi    there  ".to_owned());

        assert_eq!(
            result,
            Some(vec!["echo".to_owned(), "hi".to_owned(), "there".to_owned()]),
        );

        let result = parser.parse_line(" /bin/echo ".to_owned());

        assert_eq!(result, Some(vec!["/bin/echo".to_owned()]));

        let result = parser.parse_line("".to_owned());

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

        let result = parser.parse_line("file with spaces".to_owned());

        assert_eq!(
            result,
            Some(vec![
                "gzip".to_owned(),
                "-k".to_owned(),
                "file with spaces".to_owned()
            ]),
        );

        let result = parser.parse_line("".to_owned());

        assert_eq!(result, None);
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

        let result = parser.parse_line("awesomebashfunction 1 2 3".to_owned());

        assert_eq!(
            result,
            Some(vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                "awesomebashfunction 1 2 3".to_owned(),
            ]),
        );

        std::env::set_var("SHELL", "/bin/bash");

        let parser = InputLineParser::new(&command_line_args);

        let result = parser.parse_line(" awesomebashfunction 1 2 3 ".to_owned());

        assert_eq!(
            result,
            Some(vec![
                "/bin/bash".to_owned(),
                "-c".to_owned(),
                " awesomebashfunction 1 2 3 ".to_owned(),
            ]),
        );

        let result = parser.parse_line("".to_owned());

        assert_eq!(result, None);

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

        let result = parser.parse_line("stuff".to_owned());

        assert_eq!(
            result,
            Some(vec!["md5".to_owned(), "-s".to_owned(), "stuff".to_owned()]),
        );

        let result = parser.parse_line(" stuff things ".to_owned());

        assert_eq!(
            result,
            Some(vec![
                "md5".to_owned(),
                "-s".to_owned(),
                "stuff".to_owned(),
                "things".to_owned(),
            ]),
        );

        let result = parser.parse_line("".to_owned());

        assert_eq!(result, None);
    }
}
