use itertools::Itertools;

use tracing::trace;

use std::collections::VecDeque;

use crate::{
    command_line_args::{CommandLineArgs, COMMANDS_FROM_ARGS_SEPARATOR},
    common::OwnedCommandAndArgs,
};

pub struct CommandLineArgsParser {
    argument_groups: VecDeque<Vec<String>>,
    shell_command_and_args: Option<Vec<String>>,
}

impl CommandLineArgsParser {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let argument_groups = Self::build_argument_groups(command_line_args);

        let shell_command_and_args = if command_line_args.shell {
            Some(vec![command_line_args.shell_path.clone(), "-c".to_owned()])
        } else {
            None
        };

        Self {
            argument_groups,
            shell_command_and_args,
        }
    }

    fn build_argument_groups(command_line_args: &CommandLineArgs) -> VecDeque<Vec<String>> {
        let mut argument_groups =
            VecDeque::with_capacity(command_line_args.command_and_initial_arguments.len());

        let mut current_vec = vec![];

        for string in &command_line_args.command_and_initial_arguments {
            if string == COMMANDS_FROM_ARGS_SEPARATOR {
                if !current_vec.is_empty() {
                    argument_groups.push_back(current_vec);
                    current_vec = vec![];
                }
            } else {
                current_vec.push(string.clone());
            }
        }

        if !current_vec.is_empty() {
            argument_groups.push_back(current_vec);
        }

        argument_groups
    }

    pub fn parse_command_line_args(self) -> Vec<OwnedCommandAndArgs> {
        let mut argument_groups = self.argument_groups;

        let Some(first_command_and_args) = argument_groups.pop_front() else {
            return vec![];
        };

        let arguments_list: Vec<Vec<String>> = argument_groups
            .into_iter()
            .multi_cartesian_product()
            .collect();

        trace!(
            "first_command_and_args = {:?} arguments_list = {:?}",
            first_command_and_args,
            arguments_list,
        );

        let result = arguments_list
            .into_iter()
            .map(|current_args| match &self.shell_command_and_args {
                None => [first_command_and_args.clone(), current_args]
                    .concat()
                    .into(),
                Some(shell_command_and_args) => {
                    let merged_args = [first_command_and_args.clone(), current_args]
                        .concat()
                        .join(" ");
                    let merged_args = vec![merged_args];
                    [shell_command_and_args.clone(), merged_args]
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
            .map_into()
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
