use anyhow::Context;

use tracing::trace;

use std::borrow::Cow;

use crate::command_line_args::CommandLineArgs;

#[derive(Clone)]
pub struct RegexProcessor {
    command_line_regex: Option<CommandLineRegex>,
}

impl RegexProcessor {
    pub fn new(command_line_args: &CommandLineArgs) -> anyhow::Result<Self> {
        let command_line_regex = match &command_line_args.regex {
            None => None,
            Some(command_line_args_regex) => Some(CommandLineRegex::new(command_line_args_regex)?),
        };
        Ok(Self { command_line_regex })
    }

    pub fn regex_mode(&self) -> bool {
        self.command_line_regex.is_some()
    }

    pub fn process_string<'a>(&self, argument: &'a str, input_data: &'a str) -> Cow<'a, str> {
        let argument = Cow::from(argument);

        match &self.command_line_regex {
            None => argument,
            Some(command_line_regex) => command_line_regex.expand(argument, input_data),
        }
    }
}

#[derive(Clone)]
struct CommandLineRegex {
    regex: regex::Regex,
    numbered_group_match_keys: Vec<String>,
    named_group_to_match_key: Vec<(String, String)>,
}

type MatchKeyAndValuesVec<'a> = Vec<(Cow<'a, str>, Cow<'a, str>)>;

impl CommandLineRegex {
    pub fn new(command_line_args_regex: &str) -> anyhow::Result<Self> {
        let regex = regex::Regex::new(command_line_args_regex)
            .context("CommandLineRegex::new: error creating regex")?;

        let mut numbered_group_match_keys = vec![];

        let mut named_group_to_match_key = vec![];

        for (i, capture_name_option) in regex.capture_names().enumerate() {
            let match_key = format!("{{{}}}", i);
            numbered_group_match_keys.push(match_key);

            if let Some(capture_name) = capture_name_option {
                let match_key = format!("{{{}}}", capture_name);
                named_group_to_match_key.push((capture_name.to_owned(), match_key));
            }
        }

        trace!(
            "numbered_group_match_keys = {:?} named_group_to_match_key = {:?}",
            numbered_group_match_keys,
            named_group_to_match_key,
        );

        Ok(Self {
            regex,
            numbered_group_match_keys,
            named_group_to_match_key,
        })
    }

    fn build_match_key_and_values<'a>(
        &'a self,
        captures: regex::Captures<'a>,
    ) -> MatchKeyAndValuesVec<'a> {
        let mut match_key_and_values = MatchKeyAndValuesVec::with_capacity(
            self.numbered_group_match_keys.len() + self.named_group_to_match_key.len(),
        );

        for (i, match_option) in captures.iter().enumerate() {
            trace!("got match i = {} match_option = {:?}", i, match_option);
            if let Some(match_value) = match_option {
                if let Some(match_key) = self.numbered_group_match_keys.get(i) {
                    match_key_and_values.push((match_key.into(), match_value.as_str().into()));
                }
            }
        }

        for (group_name, match_key) in self.named_group_to_match_key.iter() {
            if let Some(match_value) = captures.name(group_name) {
                match_key_and_values.push((match_key.into(), match_value.as_str().into()));
            }
        }

        match_key_and_values
    }

    fn expand<'a>(&self, argument: Cow<'a, str>, input_data: &'a str) -> Cow<'a, str> {
        let captures = match self.regex.captures(input_data) {
            None => return argument,
            Some(captures) => captures,
        };

        trace!("captures = {:?}", captures,);

        let match_key_and_values = self.build_match_key_and_values(captures);

        trace!(
            "after build_match_key_and_values match_key_and_values = {:?}",
            match_key_and_values
        );

        let mut argument = argument;

        for (match_key, value) in match_key_and_values {
            let match_key = &*match_key;
            if argument.contains(match_key) {
                argument = Cow::from(argument.replace(match_key, &value));
            }
        }

        trace!("After second loop argument = {:?}", argument);

        argument
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
