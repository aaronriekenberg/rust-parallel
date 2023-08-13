use regex::Regex;

use tracing::debug;

use crate::command_line_args::CommandLineArgs;

use std::borrow::Cow;

// Examples:
// cat test | RUST_LOG=debug ./target/debug/rust-parallel -r '(?P<url>.*),(?P<filename>.*)' echo got url={url} filename={filename}
// ./target/debug/rust-parallel -r '(?P<url>.*),(?P<filename>.*)' echo  got url={url} filename={filename} ::: URL1,filename1  URL2,filename2

pub struct RegexProcessor {
    regex: Option<regex::Regex>,
}

impl RegexProcessor {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        Self {
            regex: command_line_args
                .regex
                .as_ref()
                .map(|regex| Regex::new(regex).expect("RegexProcessor::new error creating regex")),
        }
    }

    pub fn regex_mode(&self) -> bool {
        self.regex.is_some()
    }

    pub fn process_string<'a>(&self, argument: &'a str, input_data: &'a str) -> Cow<'a, str> {
        debug!(
            "in process_string argument = {:?} input_data = {:?}",
            argument, input_data
        );

        let regex = match &self.regex {
            None => return Cow::from(argument),
            Some(regex) => regex,
        };

        let captures = match regex.captures(input_data) {
            None => return Cow::from(argument),
            Some(captures) => captures,
        };

        debug!("captures = ${:?}", captures);

        // expand expects capture group references of the form ${ref}.
        // on the command line we take {ref} so replace { with ${ before calling expand.
        let argument = argument.replace('{', "${");

        let mut dest = String::new();

        captures.expand(&argument, &mut dest);

        debug!(
            "after expand argument = {:?} input_data = {:?} dest = {:?}",
            argument, input_data, dest
        );

        Cow::from(dest)
    }
}
