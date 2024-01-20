use anyhow::Context;

use tracing::warn;

use std::{borrow::Cow, sync::Arc};

use crate::command_line_args::CommandLineArgs;

pub struct RegexProcessor {
    command_line_regex: Option<CommandLineRegex>,
}

impl RegexProcessor {
    pub fn new(command_line_args: &CommandLineArgs) -> anyhow::Result<Arc<Self>> {
        let command_line_regex = match &command_line_args.regex {
            None => None,
            Some(command_line_args_regex) => Some(CommandLineRegex::new(command_line_args_regex)?),
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
    ) -> Option<Vec<String>> {
        let command_line_regex = match &self.command_line_regex {
            Some(command_line_regex) => command_line_regex,
            None => return None,
        };

        let mut results: Vec<String> = Vec::with_capacity(arguments.len());
        let mut found_match = false;

        for argument in arguments {
            match command_line_regex.expand(argument.into(), input_data) {
                Some(result) => {
                    results.push(result.to_string());
                    found_match = true;
                }
                None => {
                    results.push(argument.clone());
                }
            };
        }

        if !found_match {
            warn!("regex did not match input data: {}", input_data);
            None
        } else {
            Some(results)
        }
    }
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
            let match_key = format!("{{{}}}", i);
            numbered_group_match_keys.push(match_key);

            if let Some(capture_name) = capture_name_option {
                let match_key = format!("{{{}}}", capture_name);
                named_group_to_match_key.push((capture_name.to_owned(), match_key));
            }
        }

        Ok(Self {
            regex,
            numbered_group_match_keys,
            named_group_to_match_key,
        })
    }

    fn expand<'a>(&self, argument: Cow<'a, str>, input_data: &str) -> Option<Cow<'a, str>> {
        let captures = self.regex.captures(input_data)?;

        let mut argument = argument;

        let mut update_argument = |match_key, match_value| {
            if argument.contains(match_key) {
                argument = Cow::from(argument.replace(match_key, match_value));
            }
        };

        // numbered capture groups
        for (i, match_option) in captures.iter().enumerate() {
            if let (Some(match_value), Some(match_key)) =
                (match_option, self.numbered_group_match_keys.get(i))
            {
                update_argument(match_key, match_value.as_str());
            }
        }

        // named capture groups
        for (group_name, match_key) in self.named_group_to_match_key.iter() {
            if let Some(match_value) = captures.name(group_name) {
                update_argument(match_key, match_value.as_str());
            }
        }

        Some(argument)
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
            Some(vec!["hello world".to_string()])
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
            Some(vec!["hello world".to_string()])
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
            Some(vec![
                r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
                    .to_string(),
            ])
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
            Some(vec![
                r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
                    .to_string(),
            ])
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
            Some(vec![r#"world${FOO}hello$BAR${BAR}world"#.to_string()]),
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
            Some(vec!["world,hello".to_string()]),
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
}
