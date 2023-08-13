use itertools::Itertools;

use crate::{
    command_line_args::{CommandLineArgs, COMMANDS_FROM_ARGS_SEPARATOR},
    common::OwnedCommandAndArgs,
    parser::{regex::RegexProcessor, ShellCommandAndArgs},
};

#[derive(Debug)]
struct ArgumentGroups {
    first_command_and_args: Vec<String>,
    remaining_argument_groups: Vec<Vec<String>>,
}

pub struct CommandLineArgsParser {
    argument_groups: ArgumentGroups,
    shell_command_and_args: ShellCommandAndArgs,
    regex_processor: RegexProcessor,
}

impl CommandLineArgsParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let argument_groups = Self::build_argument_groups(command_line_args);

        let shell_command_and_args = super::build_shell_command_and_args(command_line_args);

        Self {
            argument_groups,
            shell_command_and_args,
            regex_processor: RegexProcessor::new(command_line_args),
        }
    }

    fn build_argument_groups(command_line_args: &CommandLineArgs) -> ArgumentGroups {
        let command_and_initial_arguments = &command_line_args.command_and_initial_arguments;

        let mut remaining_argument_groups = Vec::with_capacity(command_and_initial_arguments.len());

        let mut first = true;

        let mut first_command_and_args = vec![];

        for (separator, group) in &command_and_initial_arguments
            .iter()
            .group_by(|arg| *arg == COMMANDS_FROM_ARGS_SEPARATOR)
        {
            let group_vec = group.cloned().collect();

            if first {
                if !separator {
                    first_command_and_args = group_vec;
                }
                first = false;
            } else if !separator {
                remaining_argument_groups.push(group_vec);
            }
        }

        ArgumentGroups {
            first_command_and_args,
            remaining_argument_groups,
        }
    }

    pub fn parse_command_line_args(self) -> Vec<OwnedCommandAndArgs> {
        let ArgumentGroups {
            first_command_and_args,
            remaining_argument_groups,
        } = self.argument_groups;

        remaining_argument_groups
            .into_iter()
            .multi_cartesian_product()
            .filter_map(|current_args| {
                let cmd_and_args = if !self.regex_processor.regex_mode() {
                    [first_command_and_args.clone(), current_args].concat()
                } else {
                    let input_line = current_args.join(" ");

                    first_command_and_args
                        .iter()
                        .map(|arg| {
                            self.regex_processor
                                .process_string(&arg, &input_line)
                                .into()
                        })
                        .collect_vec()
                };
                super::build_owned_command_and_args(&self.shell_command_and_args, cmd_and_args)
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{default::Default, path::PathBuf};

    #[test]
    fn test_parse_command_line_args_with_intial_command() {
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
    fn test_parse_command_line_args_no_intial_command() {
        let command_line_args = CommandLineArgs {
            shell: false,
            command_and_initial_arguments: vec![
                ":::", "echo", "say", ":::", "arg1", "arg2", "arg3",
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
                    args: vec!["arg1"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["arg2"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["arg3"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("say"),
                    args: vec!["arg1"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("say"),
                    args: vec!["arg2"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("say"),
                    args: vec!["arg3"].into_iter().map_into().collect(),
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
    fn test_parse_command_line_args_shell_mode_with_initial_command() {
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

    #[test]
    fn test_parse_command_line_args_shell_mode_no_initial_command() {
        let command_line_args = CommandLineArgs {
            shell: true,
            command_and_initial_arguments: vec![":::", "say", "echo", ":::", "C", "D", "E"]
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
                    args: vec!["-c", "say C"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "say D"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "say E"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo C"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo D"].into_iter().map_into().collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("/bin/bash"),
                    args: vec!["-c", "echo E"].into_iter().map_into().collect(),
                },
            ]
        );
    }
}
