pub mod buffered;
pub mod command_line;
pub mod pipe;
mod regex;

use std::{sync::Arc, sync::OnceLock};

use crate::{
    command_line_args::CommandLineArgs, common::OwnedCommandAndArgs, parser::pipe::PipeModeParser,
};

use self::{
    buffered::BufferedInputLineParser, command_line::CommandLineArgsParser, regex::RegexProcessor,
};

struct ShellCommandAndArgs(Option<Vec<String>>);

impl ShellCommandAndArgs {
    fn new(command_line_args: &CommandLineArgs) -> Self {
        Self(if command_line_args.shell {
            Some(vec![
                command_line_args.shell_path.clone(),
                command_line_args.shell_argument.clone(),
            ])
        } else {
            None
        })
    }
}

fn build_owned_command_and_args(
    shell_command_and_args: &ShellCommandAndArgs,
    command_and_args: Vec<String>,
) -> Option<OwnedCommandAndArgs> {
    match &shell_command_and_args.0 {
        None => OwnedCommandAndArgs::try_from(command_and_args).ok(),
        Some(shell_command_and_args) => {
            let mut result = Vec::with_capacity(shell_command_and_args.len() + 1);

            result.extend(shell_command_and_args.iter().cloned());
            result.push(command_and_args.join(" "));

            OwnedCommandAndArgs::try_from(result).ok()
        }
    }
}

pub struct Parsers {
    buffered_input_line_parser: OnceLock<BufferedInputLineParser>,
    regex_processor: Arc<RegexProcessor>,
    command_line_args: &'static CommandLineArgs,
}

impl Parsers {
    pub fn new(command_line_args: &'static CommandLineArgs) -> anyhow::Result<Self> {
        let regex_processor = RegexProcessor::new(command_line_args)?;

        Ok(Self {
            buffered_input_line_parser: OnceLock::new(),
            regex_processor,
            command_line_args,
        })
    }

    pub fn buffered_input_line_parser(&self) -> &BufferedInputLineParser {
        self.buffered_input_line_parser.get_or_init(|| {
            BufferedInputLineParser::new(self.command_line_args, &self.regex_processor)
        })
    }

    pub fn pipe_mode_parser(&self) -> PipeModeParser {
        PipeModeParser::new(self.command_line_args)
    }

    pub fn command_line_args_parser(&self) -> CommandLineArgsParser {
        CommandLineArgsParser::new(self.command_line_args, &self.regex_processor)
    }
}
