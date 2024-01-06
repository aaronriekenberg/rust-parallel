use itertools::Itertools;

use std::collections::VecDeque;

use crate::{
    command_line_args::{CommandLineArgs, COMMANDS_FROM_ARGS_SEPARATOR},
    common::OwnedCommandAndArgs,
    parser::{regex::RegexProcessor, ShellCommandAndArgs},
};

#[derive(Debug)]
struct ArgumentGroups {
    first_command_and_args: Vec<String>,
    all_argument_groups: VecDeque<Vec<String>>,
}

pub struct CommandLineArgsParser {
    argument_groups: ArgumentGroups,
    shell_command_and_args: ShellCommandAndArgs,
    regex_processor: RegexProcessor,
}

impl CommandLineArgsParser {
    pub fn new(command_line_args: &CommandLineArgs, regex_processor: RegexProcessor) -> Self {
        let argument_groups = Self::build_argument_groups(command_line_args);

        let shell_command_and_args = ShellCommandAndArgs::new(command_line_args);

        Self {
            argument_groups,
            shell_command_and_args,
            regex_processor,
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

        let all_argument_groups = remaining_argument_groups
            .into_iter()
            .multi_cartesian_product()
            .collect();

        ArgumentGroups {
            first_command_and_args,
            all_argument_groups,
        }
    }

    fn parse_argument_group(&self, argument_group: Vec<String>) -> Option<OwnedCommandAndArgs> {
        let cmd_and_args = if !self.regex_processor.regex_mode() {
            [
                self.argument_groups.first_command_and_args.clone(),
                argument_group,
            ]
            .concat()
        } else {
            let input_line = argument_group.join(" ");

            self.regex_processor.apply_regex_to_arguments(
                &self.argument_groups.first_command_and_args,
                &input_line,
            )?
        };

        super::build_owned_command_and_args(&self.shell_command_and_args, cmd_and_args)
    }

    pub fn has_remaining_argument_groups(&self) -> bool {
        !self.argument_groups.all_argument_groups.is_empty()
    }

    pub fn parse_next_argument_group(&mut self) -> Option<OwnedCommandAndArgs> {
        match self.argument_groups.all_argument_groups.pop_front() {
            None => None,
            Some(argument_group) => self.parse_argument_group(argument_group),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{default::Default, path::PathBuf};

    fn collect_into_vec(mut parser: CommandLineArgsParser) -> Vec<OwnedCommandAndArgs> {
        let mut result = vec![];

        while parser.has_remaining_argument_groups() {
            let Some(cmd_and_args) = parser.parse_next_argument_group() else {
                continue;
            };

            result.push(cmd_and_args);
        }

        result
    }

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

        let parser = CommandLineArgsParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = collect_into_vec(parser);

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

        let parser = CommandLineArgsParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = collect_into_vec(parser);

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

        let parser = CommandLineArgsParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = collect_into_vec(parser);

        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_parse_command_line_args_invalid() {
        let command_line_args = CommandLineArgs {
            shell: false,
            command_and_initial_arguments: vec![":::", ":::"].into_iter().map_into().collect(),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = collect_into_vec(parser);

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
            shell_argument: "-c".to_owned(),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = collect_into_vec(parser);

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
            shell_argument: "-c".to_owned(),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = collect_into_vec(parser);

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

    #[test]
    fn test_regex_named_groups() {
        let command_line_args = CommandLineArgs {
            command_and_initial_arguments: vec![
                "echo",
                "got",
                "arg1={arg1}",
                "arg2={arg2}",
                "arg3={arg3}",
                ":::",
                "foo,bar,baz",
                "foo2,bar2,baz2",
            ]
            .into_iter()
            .map_into()
            .collect(),
            regex: Some("(?P<arg1>.*),(?P<arg2>.*),(?P<arg3>.*)".to_owned()),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = collect_into_vec(parser);

        assert_eq!(
            result,
            vec![
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["got", "arg1=foo", "arg2=bar", "arg3=baz"]
                        .into_iter()
                        .map_into()
                        .collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["got", "arg1=foo2", "arg2=bar2", "arg3=baz2"]
                        .into_iter()
                        .map_into()
                        .collect(),
                },
            ]
        );
    }

    #[test]
    fn test_regex_numbered_groups() {
        let command_line_args = CommandLineArgs {
            command_and_initial_arguments: vec![
                "echo",
                "got",
                "arg1={0}",
                "arg2={1}",
                "arg3={2}",
                ":::",
                "foo,bar,baz",
                "foo2,bar2,baz2",
            ]
            .into_iter()
            .map_into()
            .collect(),
            regex: Some("(.*),(.*),(.*)".to_owned()),
            ..Default::default()
        };

        let parser = CommandLineArgsParser::new(
            &command_line_args,
            RegexProcessor::new(&command_line_args).unwrap(),
        );

        let result = collect_into_vec(parser);

        assert_eq!(
            result,
            vec![
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["got", "arg1=foo,bar,baz", "arg2=foo", "arg3=bar"]
                        .into_iter()
                        .map_into()
                        .collect(),
                },
                OwnedCommandAndArgs {
                    command_path: PathBuf::from("echo"),
                    args: vec!["got", "arg1=foo2,bar2,baz2", "arg2=foo2", "arg3=bar2"]
                        .into_iter()
                        .map_into()
                        .collect(),
                },
            ]
        );
    }
}
