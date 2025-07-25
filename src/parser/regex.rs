use anyhow::Context;

use itertools::Itertools;

use tracing::warn;

use std::{borrow::Cow, sync::Arc};

use crate::command_line_args::{COMMANDS_FROM_ARGS_SEPARATOR, CommandLineArgs};

#[derive(Debug, Eq, PartialEq)]
pub struct ApplyRegexToArgumentsResult {
    pub arguments: Vec<String>,
    pub modified_arguments: bool,
}

pub struct RegexProcessor {
    command_line_regex: Option<CommandLineRegex>,
}

impl RegexProcessor {
    pub fn new(command_line_args: &CommandLineArgs) -> anyhow::Result<Arc<Self>> {
        let auto_regex = AutoCommandLineArgsRegex::new(command_line_args);

        let command_line_regex = match (auto_regex, &command_line_args.regex) {
            (Some(auto_regex), _) => Some(CommandLineRegex::new(&auto_regex.0)?),
            (_, Some(cla_regex)) => Some(CommandLineRegex::new(cla_regex)?),
            _ => None,
        };

        Ok(Arc::new(Self { command_line_regex }))
    }

    pub fn regex_mode(&self) -> bool {
        self.command_line_regex.is_some()
    }

    pub fn apply_regex_to_arguments(
        &self,
        arguments: &Vec<String>,
        input_data: &str,
    ) -> Option<ApplyRegexToArgumentsResult> {
        let command_line_regex = self.command_line_regex.as_ref()?;

        let mut results: Vec<String> = Vec::with_capacity(arguments.len());
        let mut found_input_data_match = false;
        let mut modified_arguments = false;

        for argument in arguments {
            match command_line_regex.expand(argument.into(), input_data) {
                Ok(result) => {
                    results.push(result.argument.to_string());
                    found_input_data_match = true;
                    modified_arguments = modified_arguments || result.modified_argument;
                }
                Err(ExpandError::RegexDoesNotMatchInputData) => {
                    results.push(argument.clone());
                }
            };
        }

        if !found_input_data_match {
            warn!("regex did not match input data: {}", input_data);
            None
        } else {
            Some(ApplyRegexToArgumentsResult {
                arguments: results,
                modified_arguments,
            })
        }
    }
}

#[derive(Debug)]
struct ExpandResult<'a> {
    argument: Cow<'a, str>,
    modified_argument: bool,
}

#[derive(thiserror::Error, Debug)]
enum ExpandError {
    #[error("regex does not match input data")]
    RegexDoesNotMatchInputData,
}

struct CommandLineRegex {
    regex: regex::Regex,
    numbered_group_match_keys: Vec<String>,
    named_group_to_match_key: Vec<(String, String)>,
}

impl CommandLineRegex {
    fn new(command_line_args_regex: &str) -> anyhow::Result<Self> {
        let regex = regex::Regex::new(command_line_args_regex)
            .context("CommandLineRegex::new: error creating regex")?;

        let capture_names = regex.capture_names();

        let mut numbered_group_match_keys = Vec::with_capacity(capture_names.len());

        let mut named_group_to_match_key = Vec::with_capacity(capture_names.len());

        for (i, capture_name_option) in capture_names.enumerate() {
            let match_key = format!("{{{i}}}");
            numbered_group_match_keys.push(match_key);

            if let Some(capture_name) = capture_name_option {
                let match_key = format!("{{{capture_name}}}");
                named_group_to_match_key.push((capture_name.to_owned(), match_key));
            }
        }

        Ok(Self {
            regex,
            numbered_group_match_keys,
            named_group_to_match_key,
        })
    }

    fn expand<'a>(
        &self,
        argument: Cow<'a, str>,
        input_data: &str,
    ) -> Result<ExpandResult<'a>, ExpandError> {
        let captures = self
            .regex
            .captures(input_data)
            .ok_or(ExpandError::RegexDoesNotMatchInputData)?;

        let mut argument = argument;
        let mut modified_argument = false;

        let mut update_argument = |match_key, match_value| {
            if argument.contains(match_key) {
                argument = Cow::from(argument.replace(match_key, match_value));
                modified_argument = true;
            }
        };

        // numbered capture groups
        for (i, match_option) in captures.iter().enumerate() {
            if let (Some(match_value), Some(match_key)) =
                (match_option, self.numbered_group_match_keys.get(i))
            {
                // make {} have the same behavior as {0}
                if i == 0 {
                    update_argument("{}", match_value.as_str());
                }
                update_argument(match_key, match_value.as_str());
            }
        }

        // named capture groups
        for (group_name, match_key) in self.named_group_to_match_key.iter() {
            if let Some(match_value) = captures.name(group_name) {
                update_argument(match_key, match_value.as_str());
            }
        }

        Ok(ExpandResult {
            argument,
            modified_argument,
        })
    }
}

#[derive(Debug)]
struct AutoCommandLineArgsRegex(String);

impl AutoCommandLineArgsRegex {
    fn new(command_line_args: &CommandLineArgs) -> Option<Self> {
        if command_line_args.regex.is_none() && command_line_args.commands_from_args_mode() {
            Self::new_auto_interpolate_commands_from_args(command_line_args)
        } else {
            None
        }
    }

    fn new_auto_interpolate_commands_from_args(
        command_line_args: &CommandLineArgs,
    ) -> Option<Self> {
        let mut first = true;
        let mut argument_group_count = 0;

        for (separator, _group) in &command_line_args
            .command_and_initial_arguments
            .iter()
            .chunk_by(|arg| *arg == COMMANDS_FROM_ARGS_SEPARATOR)
        {
            if first {
                if separator {
                    return None;
                }
                first = false;
            } else if !separator {
                argument_group_count += 1;
            }
        }

        let argument_group_count = argument_group_count;

        let mut generated_regex = String::with_capacity(argument_group_count * 5);

        for i in 0..argument_group_count {
            if i != 0 {
                generated_regex.push(' ');
            }
            generated_regex.push_str("(.*)");
        }

        Some(Self(generated_regex))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_regex_disabled() {
        let command_line_args = CommandLineArgs {
            regex: None,
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), false);

        let arguments = vec!["{0}".to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "input line"),
            None,
        );
    }

    #[test]
    fn test_regex_numbered_groups() {
        let command_line_args = CommandLineArgs {
            regex: Some("(.*),(.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        let arguments = vec!["{1} {2}".to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello,world"),
            Some(ApplyRegexToArgumentsResult {
                arguments: vec!["hello world".to_string()],
                modified_arguments: true,
            })
        );
    }

    #[test]
    fn test_regex_named_groups() {
        let command_line_args = CommandLineArgs {
            regex: Some("(?P<arg1>.*),(?P<arg2>.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        let arguments = vec!["{arg1} {arg2}".to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello,world"),
            Some(ApplyRegexToArgumentsResult {
                arguments: vec!["hello world".to_string()],
                modified_arguments: true,
            })
        );
    }

    #[test]
    fn test_regex_numbered_groups_json() {
        let command_line_args = CommandLineArgs {
            regex: Some("(.*),(.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        let arguments =
            vec![r#"{"id": 123, "$zero": "{0}", "one": "{1}", "two": "{2}"}"#.to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello,world",),
            Some(ApplyRegexToArgumentsResult {
                arguments: vec![
                    r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
                        .to_string(),
                ],
                modified_arguments: true,
            })
        );
    }

    #[test]
    fn test_regex_numbered_groups_json_empty_group() {
        let command_line_args = CommandLineArgs {
            regex: Some("(.*),(.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        let arguments =
            vec![r#"{"id": 123, "$zero": "{}", "one": "{1}", "two": "{2}"}"#.to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello,world",),
            Some(ApplyRegexToArgumentsResult {
                arguments: vec![
                    r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
                        .to_string(),
                ],
                modified_arguments: true,
            })
        );
    }

    #[test]
    fn test_regex_named_groups_json() {
        let command_line_args = CommandLineArgs {
            regex: Some("(?P<arg1>.*),(?P<arg2>.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        let arguments =
            vec![r#"{"id": 123, "$zero": "{0}", "one": "{arg1}", "two": "{arg2}"}"#.to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello,world",),
            Some(ApplyRegexToArgumentsResult {
                arguments: vec![
                    r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
                        .to_string()
                ],
                modified_arguments: true,
            })
        );
    }

    #[test]
    fn test_regex_named_groups_json_empty_group() {
        let command_line_args = CommandLineArgs {
            regex: Some("(?P<arg1>.*),(?P<arg2>.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        let arguments =
            vec![r#"{"id": 123, "$zero": "{}", "one": "{arg1}", "two": "{arg2}"}"#.to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello,world",),
            Some(ApplyRegexToArgumentsResult {
                arguments: vec![
                    r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
                        .to_string()
                ],
                modified_arguments: true,
            })
        );
    }

    #[test]
    fn test_regex_string_containing_dollar_curly_brace_variable() {
        let command_line_args = CommandLineArgs {
            regex: Some("(?P<arg1>.*),(?P<arg2>.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        let arguments = vec![r#"{arg2}${FOO}{arg1}$BAR${BAR}{arg2}"#.to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello,world"),
            Some(ApplyRegexToArgumentsResult {
                arguments: vec![r#"world${FOO}hello$BAR${BAR}world"#.to_string()],
                modified_arguments: true,
            })
        );
    }

    #[test]
    fn test_regex_not_matching_input_data() {
        let command_line_args = CommandLineArgs {
            regex: Some("(?P<arg1>.*),(?P<arg2>.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        let arguments = vec!["{arg2},{arg1}".to_string()];
        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello,world"),
            Some(ApplyRegexToArgumentsResult {
                arguments: vec!["world,hello".to_string()],
                modified_arguments: true,
            }),
        );

        assert_eq!(
            regex_processor.apply_regex_to_arguments(&arguments, "hello world"),
            None,
        );
    }

    #[test]
    fn test_regex_invalid() {
        let command_line_args = CommandLineArgs {
            regex: Some("(?Parg1>.*),(?P<arg2>.*)".to_string()),
            ..Default::default()
        };

        let result = RegexProcessor::new(&command_line_args);

        assert!(result.is_err());
    }

    #[test]
    fn test_auto_regex_command_line_regex() {
        let command_line_args = CommandLineArgs {
            regex: Some("(?Parg1>.*),(?P<arg2>.*)".to_string()),
            ..Default::default()
        };

        let auto_regex = AutoCommandLineArgsRegex::new(&command_line_args);

        assert!(auto_regex.is_none());
    }

    #[test]
    fn test_auto_regex_not_command_line_args_mode() {
        let command_line_args = CommandLineArgs {
            regex: None,
            command_and_initial_arguments: ["echo"].into_iter().map_into().collect(),
            ..Default::default()
        };

        let auto_regex = AutoCommandLineArgsRegex::new(&command_line_args);

        assert!(auto_regex.is_none());
    }

    #[test]
    fn test_auto_regex() {
        let command_line_args = CommandLineArgs {
            regex: None,
            command_and_initial_arguments: ["echo", ":::", "A", "B", ":::", "C", "D"]
                .into_iter()
                .map_into()
                .collect(),
            ..Default::default()
        };

        let auto_regex = AutoCommandLineArgsRegex::new(&command_line_args);

        assert!(auto_regex.is_some());
        assert_eq!(auto_regex.unwrap().0, "(.*) (.*)");
    }
}
