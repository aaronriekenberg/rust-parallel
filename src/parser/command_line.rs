use itertools::Itertools;

use tracing::trace;

use std::collections::VecDeque;

use crate::{
    command_line_args::{CommandLineArgs, COMMANDS_FROM_ARGS_SEPARATOR},
    common::OwnedCommandAndArgs,
};

pub struct CommandLineArgsParser {
    split_commands: VecDeque<Vec<String>>,
    shell_enabled: bool,
    shell_command_and_args: Vec<String>,
}

impl CommandLineArgsParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let split_commands = Self::build_split_commands(command_line_args);

        let shell_command_and_args = if command_line_args.shell {
            vec![command_line_args.shell_path.clone(), "-c".to_owned()]
        } else {
            vec![]
        };

        Self {
            split_commands,
            shell_enabled: command_line_args.shell,
            shell_command_and_args,
        }
    }

    fn build_split_commands(command_line_args: &CommandLineArgs) -> VecDeque<Vec<String>> {
        let mut split_commands =
            VecDeque::with_capacity(command_line_args.command_and_initial_arguments.len());

        let mut current_vec: Vec<String> = vec![];

        for string in &command_line_args.command_and_initial_arguments {
            if string == COMMANDS_FROM_ARGS_SEPARATOR {
                if !current_vec.is_empty() {
                    split_commands.push_back(current_vec);
                    current_vec = vec![];
                }
            } else {
                current_vec.push(string.clone());
            }
        }

        if !current_vec.is_empty() {
            split_commands.push_back(current_vec);
        }

        split_commands
    }

    pub fn parse_command_line_args(self) -> Vec<OwnedCommandAndArgs> {
        let mut split_commands = self.split_commands;

        let Some(first_command_and_args) = split_commands.pop_front() else {
            return vec![];
        };

        let split_args: Vec<Vec<String>> = split_commands
            .into_iter()
            .multi_cartesian_product()
            .collect();

        trace!(
            "first_command_and_args = {:?} split_args = {:?}",
            first_command_and_args,
            split_args,
        );

        let result = split_args
            .into_iter()
            .map(|current_args| {
                if self.shell_enabled {
                    let merged_args = [first_command_and_args.clone(), current_args]
                        .concat()
                        .join(" ");
                    let merged_args = vec![merged_args];
                    [self.shell_command_and_args.clone(), merged_args]
                        .concat()
                        .into()
                } else {
                    [first_command_and_args.clone(), current_args]
                        .concat()
                        .into()
                }
            })
            .collect();

        trace!("result = {:?}", result);

        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::default::Default;

    #[test]
    fn test_parse_command_line_args() {
        let command_line_args = CommandLineArgs {
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
            shell: false,
            command_and_initial_arguments: vec![],
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(&command_line_args);

        let result = parser.parse_command_line_args();

        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_parse_command_line_args_invalid() {
        let command_line_args = CommandLineArgs {
            shell: false,
            command_and_initial_arguments: vec![":::", ":::"]
                .into_iter()
                .map(|s| s.to_owned())
                .collect(),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(&command_line_args);

        let result = parser.parse_command_line_args();

        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_parse_command_line_args_shell_mode() {
        let command_line_args = CommandLineArgs {
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
