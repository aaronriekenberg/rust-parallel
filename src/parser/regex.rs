use anyhow::Context;

use itertools::Itertools;
use regex::Regex;

use std::{borrow::Cow, collections::HashMap};

use tracing::trace;

use crate::command_line_args::CommandLineArgs;

#[derive(Clone)]
struct InternalState {
    command_line_regex: Regex,
}

#[derive(Clone)]
pub struct RegexProcessor {
    internal_state: Option<InternalState>,
}

impl RegexProcessor {
    pub fn new(command_line_args: &CommandLineArgs) -> anyhow::Result<Self> {
        let internal_state = match &command_line_args.regex {
            None => None,
            Some(command_line_args_regex) => {
                let command_line_regex = Regex::new(command_line_args_regex)
                    .context("RegexProcessor::new: error creating command_line_regex")?;

                Some(InternalState { command_line_regex })
            }
        };
        Ok(Self { internal_state })
    }

    pub fn regex_mode(&self) -> bool {
        self.internal_state.is_some()
    }

    pub fn process_string<'a>(&self, argument: &'a str, input_data: &str) -> Cow<'a, str> {
        let internal_state = match &self.internal_state {
            None => return Cow::from(argument),
            Some(internal_state) => internal_state,
        };

        trace!("before replace argument = {}", argument);

        let group_names = internal_state
            .command_line_regex
            .capture_names()
            .flatten()
            .collect_vec();

        trace!("group_names = {:?}", group_names);

        let mut match_to_value: HashMap<String, Cow<'_, str>> = HashMap::new();

        match_to_value.insert("{0}".to_string(), Cow::from(input_data));

        for captures in internal_state.command_line_regex.captures_iter(input_data) {
            trace!("captures = {:?}", captures);
            for (i, match_option) in captures.iter().enumerate().skip(1) {
                trace!("got match i = {} match_option = {:?}", i, match_option);
                if let Some(match_object) = match_option {
                    let key = format!("{{{}}}", i);
                    match_to_value.insert(key, Cow::from(match_object.as_str()));
                }
            }

            for name in group_names.iter() {
                if let Some(match_object) = captures.name(name) {
                    let key = format!("{{{}}}", name);
                    match_to_value.insert(key, Cow::from(match_object.as_str()));
                }
            }
        }

        let match_to_value = match_to_value;

        trace!("After loop match_to_value = {:?}", match_to_value);

        let mut argument = argument.to_string();

        for (key, value) in match_to_value {
            argument = argument.replace(&key, &value);
        }

        trace!("After second loop argument = {:?}", argument);

        Cow::from(argument)
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

        assert_eq!(regex_processor.process_string("{0}", "input line"), "{0}");
    }

    #[test]
    fn test_regex_numbered_groups() {
        let command_line_args = CommandLineArgs {
            regex: Some("(.*),(.*)".to_string()),
            ..Default::default()
        };

        let regex_processor = RegexProcessor::new(&command_line_args).unwrap();

        assert_eq!(regex_processor.regex_mode(), true);

        assert_eq!(
            regex_processor.process_string("{1} {2}", "hello,world"),
            "hello world"
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

        assert_eq!(
            regex_processor.process_string("{arg1} {arg2}", "hello,world"),
            "hello world"
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

        assert_eq!(
            regex_processor.process_string(
                r#"{"id": 123, "$zero": "{0}", "one": "{1}", "two": "{2}"}"#,
                "hello,world",
            ),
            r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
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

        assert_eq!(
            regex_processor.process_string(
                r#"{"id": 123, "$zero": "{0}", "one": "{arg1}", "two": "{arg2}"}"#,
                "hello,world",
            ),
            r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
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

        assert_eq!(
            regex_processor.process_string(r#"{arg2}${FOO}{arg1}$BAR${BAR}{arg2}"#, "hello,world"),
            r#"world${FOO}hello$BAR${BAR}world"#,
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
