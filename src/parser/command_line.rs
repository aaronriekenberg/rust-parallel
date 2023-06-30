use itertools::Itertools;

use std::{collections::VecDeque, path::PathBuf};

use crate::{
    command_line_args::{CommandLineArgs, COMMANDS_FROM_ARGS_SEPARATOR},
    common::OwnedCommandAndArgs,
};

pub struct CommandLineArgsParser {
    argument_groups: Vec<Vec<String>>,
    shell_command_and_args: Option<OwnedCommandAndArgs>,
}

impl CommandLineArgsParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let argument_groups = Self::build_argument_groups(command_line_args);

        let shell_command_and_args = if command_line_args.shell {
            Some(OwnedCommandAndArgs {
                command_path: PathBuf::from(&command_line_args.shell_path),
                args: vec!["-c".to_owned()],
            })
        } else {
            None
        };

        Self {
            argument_groups,
            shell_command_and_args,
        }
    }

    fn build_argument_groups(command_line_args: &CommandLineArgs) -> Vec<Vec<String>> {
        let command_and_initial_arguments = &command_line_args.command_and_initial_arguments;

        let mut argument_groups = Vec::with_capacity(command_and_initial_arguments.len());

        for (key, group) in &command_and_initial_arguments
            .iter()
            .group_by(|arg| *arg == COMMANDS_FROM_ARGS_SEPARATOR)
        {
            if !key {
                argument_groups.push(group.cloned().collect());
            }
        }

        argument_groups
    }

    pub fn parse_command_line_args(self) -> Vec<OwnedCommandAndArgs> {
        let mut argument_groups = VecDeque::from(self.argument_groups);

        let Some((first_command_path, first_command_args)) = argument_groups.pop_front().and_then(|first_group| {
            let mut first_group = VecDeque::from(first_group);
            first_group
                .pop_front()
                .map(|command_path| (command_path, Vec::from(first_group)))
        }) else {
            return vec![];
        };

        argument_groups
            .into_iter()
            .multi_cartesian_product()
            .map(|current_args| {
                let all_args = [first_command_args.clone(), current_args].concat();
                match &self.shell_command_and_args {
                    None => OwnedCommandAndArgs {
                        command_path: PathBuf::from(&first_command_path),
                        args: all_args,
                    },
                    Some(shell_command_and_args) => {
                        let merged_args = [vec![first_command_path.clone()], all_args]
                            .concat()
                            .join(" ");
                        let merged_args = vec![merged_args];
                        OwnedCommandAndArgs {
                            command_path: shell_command_and_args.command_path.clone(),
                            args: [shell_command_and_args.args.clone(), merged_args].concat(),
                        }
                    }
                }
            })
            .collect()
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
            .map_into()
            .collect(),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(&command_line_args);

        let result = parser.parse_command_line_args();

        assert_eq!(
            result,
            vec![
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["-n", "A", "C"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["-n", "A", "D"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["-n", "A", "E"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["-n", "B", "C"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["-n", "B", "D"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["-n", "B", "E"].into_iter().map_into().collect(),
                },
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
            command_and_initial_arguments: vec![":::", ":::"].into_iter().map_into().collect(),
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
            .map_into()
            .collect(),
            shell_path: "/bin/bash".to_owned(),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(&command_line_args);

        let result = parser.parse_command_line_args();

        assert_eq!(
            result,
            vec![
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo -n A C"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo -n A D"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo -n A E"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo -n B C"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo -n B D"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo -n B E"].into_iter().map_into().collect(),
                },
            ]
        );
    }
}
