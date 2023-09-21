use anyhow::Context;

use regex::Regex;

use tracing::trace;

use crate::command_line_args::CommandLineArgs;

use std::borrow::Cow;

#[derive(Clone)]
struct InternalState {
    command_line_regex: Regex,
    replace_capture_groups_regex: Regex,
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

                let replace_capture_groups_regex = Regex::new(r"\{[a-zA-Z0-9_]+\}")
                    .context("RegexProcessor::new: error creating replace_capture_groups_regex")?;

                Some(InternalState {
                    command_line_regex,
                    replace_capture_groups_regex,
                })
            }
        };
        Ok(Self { internal_state })
    }

    pub fn regex_mode(&self) -> bool {
        self.internal_state.is_some()
    }

    pub fn process_string<'a>(&self, argument: &'a str, input_data: &str) -> Cow<'a, str> {
        trace!(
            "in process_string argument = {:?} input_data = {:?}",
            argument,
            input_data
        );

        let internal_state = match &self.internal_state {
            None => return Cow::from(argument),
            Some(internal_state) => internal_state,
        };

        let captures = match internal_state.command_line_regex.captures(input_data) {
            None => return Cow::from(argument),
            Some(captures) => captures,
        };

        trace!("captures = ${:?}", captures);

        // escape all $ characters in argument so they do not get expanded.
        let argument = argument.replace('$', "$$");

        // expand expects capture group references of the form ${ref}.
        // On the command line we take {ref} so prepend all {[a-zA-Z0-9_]+} with $ before calling expand.
        // The replace_capture_groups_regex is used so we do not replace other { or } characters
        // in argument that should not be expanded.
        let argument = internal_state
            .replace_capture_groups_regex
            .replace_all(&argument, r"$$${0}");

        trace!("after replace_all argument = {:?}", argument);

        let mut dest = String::new();

        captures.expand(&argument, &mut dest);

        trace!(
            "after expand argument = {:?} input_data = {:?} dest = {:?}",
            argument,
            input_data,
            dest
        );

        Cow::from(dest)
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
    fn test_regex_invalid() {
        let command_line_args = CommandLineArgs {
            regex: Some("(?Parg1>.*),(?P<arg2>.*)".to_string()),
            ..Default::default()
        };

        let result = RegexProcessor::new(&command_line_args);

        assert!(result.is_err());
    }
}
