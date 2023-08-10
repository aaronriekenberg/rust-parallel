use regex::Regex;

use tracing::info;

use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs};

use std::{borrow::Cow, collections::HashMap};

pub struct RegexProcessor {
    regex: Option<regex::Regex>,
}

impl RegexProcessor {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let regex = match &command_line_args.regex {
            None => None,
            Some(regex) => Some(Regex::new(regex).unwrap()),
        };
        Self { regex }
    }

    pub fn process_command_and_args(
        &self,
        command_and_args: OwnedCommandAndArgs,
    ) -> OwnedCommandAndArgs {
        info!(
            "in process_command_and_args command_and_args = {:?}",
            command_and_args
        );
        let regex = match &self.regex {
            None => return command_and_args,
            Some(regex) => regex,
        };

        let args = command_and_args.args;

        let args = args
            .into_iter()
            .map(|arg| {
                let mut numbered_groups = vec![];

                let mut named_map = HashMap::new();

                let group_names = regex
                    .capture_names()
                    .filter_map(|x| x)
                    .collect::<Vec<&str>>();

                for caps in regex.captures_iter(&arg) {
                    for cap_wrapper in caps.iter() {
                        if let Some(mat) = cap_wrapper {
                            numbered_groups.push(Cow::Borrowed(mat.as_str()));
                        }
                    }

                    for name in group_names.iter() {
                        if let Some(mat) = caps.name(name) {
                            named_map.insert(name.to_string(), Cow::Borrowed(mat.as_str()));
                        }
                    }

                    info!(
                        "arg = {:?} numbered_groups = {:?} named_map = {:?}",
                        arg, numbered_groups, named_map
                    );
                }

                // numbered_groups.iter().enumerate().for_each(|(s, i)| {

                // });

                arg
            })
            .collect();

        OwnedCommandAndArgs {
            command_path: command_and_args.command_path,
            args,
        }
    }
}
