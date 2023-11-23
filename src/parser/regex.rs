use anyhow::Context;

use itertools::Itertools;

use regex::Regex;

use tracing::trace;

use std::borrow::Cow;

use crate::command_line_args::CommandLineArgs;

#[derive(Clone)]
struct InternalState {
    command_line_regex: Regex,
    group_names: Vec<String>,
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

                let group_names = command_line_regex
                    .capture_names()
                    .flatten()
                    .map_into()
                    .collect_vec();

                Some(InternalState {
                    command_line_regex,
                    group_names,
                })
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

        let captures = match internal_state.command_line_regex.captures(input_data) {
            None => return Cow::from(argument),
            Some(captures) => captures,
        };

        trace!(
            "captures = {:?} group_names = {:?}",
            captures,
            internal_state.group_names
        );

        let match_and_values = self.build_match_and_values(internal_state, captures, input_data);

        trace!("After loop match_and_values = {:?}", match_and_values);

        let mut argument = Cow::from(argument);

        for (key, value) in match_and_values {
            let key = &*key;
            if argument.contains(key) {
                argument = Cow::from(argument.replace(key, &value));
            }
        }

        trace!("After second loop argument = {:?}", argument);

        argument
    }

    fn build_match_and_values<'a>(
        &self,
        internal_state: &InternalState,
        captures: regex::Captures<'a>,
        input_data: &'a str,
    ) -> Vec<(Cow<'a, str>, Cow<'a, str>)> {
        let mut match_and_values =
            Vec::with_capacity(captures.len() + internal_state.group_names.len());

        match_and_values.push(("{0}".into(), input_data.into()));

        for (i, match_option) in captures.iter().enumerate().skip(1) {
            trace!("got match i = {} match_option = {:?}", i, match_option);
            if let Some(match_object) = match_option {
                let key = format!("{{{}}}", i);
                match_and_values.push((key.into(), match_object.as_str().into()));
            }
        }

        for name in internal_state.group_names.iter() {
            if let Some(match_object) = captures.name(name) {
                let key = format!("{{{}}}", name);
                match_and_values.push((key.into(), match_object.as_str().into()));
            }
        }

        match_and_values
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
