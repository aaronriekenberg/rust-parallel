use anyhow::Context;

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

    pub fn process_string<'a>(&self, argument: &'a str, input_data: &str) -> Option<Cow<'a, str>> {
        let argument = Cow::from(argument);

        match &self.command_line_regex {
            None => Some(argument),
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
        let captures = match self.regex.captures(input_data) {
            None => return None,
            Some(captures) => captures,
        };

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

        assert_eq!(
            regex_processor.process_string("{0}", "input line"),
            Some(Cow::from("{0}"))
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

        assert_eq!(
            regex_processor.process_string("{1} {2}", "hello,world"),
            Some(Cow::from("hello world"))
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
            Some(Cow::from("hello world"))
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
            Some(Cow::from(
                r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
            ))
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
            Some(Cow::from(
                r#"{"id": 123, "$zero": "hello,world", "one": "hello", "two": "world"}"#
            ))
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
            Some(Cow::from(r#"world${FOO}hello$BAR${BAR}world"#)),
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

        assert_eq!(
            regex_processor.process_string("{arg2},{arg1}", "hello,world"),
            Some(Cow::from("world,hello")),
        );

        assert_eq!(
            regex_processor.process_string("{arg2},{arg1}", "hello world"),
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
