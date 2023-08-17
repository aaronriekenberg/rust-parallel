use anyhow::Context;
use regex::Regex;

use tracing::trace;

use crate::command_line_args::CommandLineArgs;

use std::borrow::Cow;

#[derive(Clone)]
pub struct RegexProcessor {
    regex: Option<Regex>,
}

impl RegexProcessor {
    pub fn new(command_line_args: &CommandLineArgs) -> anyhow::Result<Self> {
        let regex = match &command_line_args.regex {
            None => None,
            Some(regex) => {
                Some(Regex::new(regex).context("RegexProcessor::new error creating regex")?)
            }
        };
        Ok(Self { regex })
    }

    pub fn regex_mode(&self) -> bool {
        self.regex.is_some()
    }

    pub fn process_string<'a>(&self, argument: &'a str, input_data: &str) -> Cow<'a, str> {
        trace!(
            "in process_string argument = {:?} input_data = {:?}",
            argument,
            input_data
        );

        let regex = match &self.regex {
            None => return Cow::from(argument),
            Some(regex) => regex,
        };

        let captures = match regex.captures(input_data) {
            None => return Cow::from(argument),
            Some(captures) => captures,
        };

        trace!("captures = ${:?}", captures);

        // expand expects capture group references of the form ${ref}.
        // on the command line we take {ref} so replace { with ${ before calling expand.
        let argument = argument.replace('{', "${");

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
