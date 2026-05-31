pub mod buffered;
pub mod command_line;
pub mod pipe;
mod regex;

use std::sync::{Arc, OnceLock};

use crate::{
    command_line_args::CommandLineArgs,
    common::{OwnedCommandAndArgs, StdinData},
    parser::pipe::PipeModeParser,
};

use self::{
    buffered::BufferedInputLineParser, command_line::CommandLineArgsParser, regex::RegexProcessor,
};

/// Intermediate result from parsing, before shell wrapping is applied.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedCommand {
    pub command_and_args: Vec<String>,
    pub stdin: Option<String>,
}

impl ParsedCommand {
    pub fn new(command_and_args: Vec<String>) -> Self {
        Self {
            command_and_args,
            stdin: None,
        }
    }

    pub fn with_stdin(mut self, stdin: String) -> Self {
        self.stdin = Some(stdin);
        self
    }
}

/// Applies shell wrapping (if configured) and converts to OwnedCommandAndArgs.
pub struct CommandBuilder {
    shell_command_and_args: Option<Vec<String>>,
}

impl CommandBuilder {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let shell_command_and_args = if command_line_args.shell {
            Some(vec![
                command_line_args.shell_path.clone(),
                command_line_args.shell_argument.clone(),
            ])
        } else {
            None
        };
        Self {
            shell_command_and_args,
        }
    }

    pub fn build(&self, parsed: ParsedCommand) -> Option<OwnedCommandAndArgs> {
        let command_and_args = match &self.shell_command_and_args {
            None => parsed.command_and_args,
            Some(shell_prefix) => {
                let mut result = Vec::with_capacity(shell_prefix.len() + 1);
                result.extend(shell_prefix.iter().cloned());
                result.push(parsed.command_and_args.join(" "));
                result
            }
        };

        let mut owned = OwnedCommandAndArgs::try_from(command_and_args).ok()?;

        if let Some(stdin) = parsed.stdin {
            owned = owned.with_stdin(StdinData(Some(Arc::new(stdin))));
        }

        Some(owned)
    }
}

pub struct Parsers {
    buffered_input_line_parser: OnceLock<BufferedInputLineParser>,
    command_line_args_parser: OnceLock<CommandLineArgsParser>,
    pipe_mode_parser: OnceLock<PipeModeParser>,
    regex_processor: Arc<RegexProcessor>,
    command_line_args: &'static CommandLineArgs,
}

impl Parsers {
    pub fn new(command_line_args: &'static CommandLineArgs) -> anyhow::Result<Self> {
        let regex_processor = RegexProcessor::new(command_line_args)?;

        Ok(Self {
            buffered_input_line_parser: OnceLock::new(),
            command_line_args_parser: OnceLock::new(),
            pipe_mode_parser: OnceLock::new(),
            regex_processor,
            command_line_args,
        })
    }

    pub fn buffered_input_line_parser(&self) -> &BufferedInputLineParser {
        self.buffered_input_line_parser.get_or_init(|| {
            BufferedInputLineParser::new(self.command_line_args, &self.regex_processor)
        })
    }

    pub fn command_line_args_parser(&self) -> &CommandLineArgsParser {
        self.command_line_args_parser.get_or_init(|| {
            CommandLineArgsParser::new(self.command_line_args, &self.regex_processor)
        })
    }

    pub fn pipe_mode_parser(&self) -> &PipeModeParser {
        self.pipe_mode_parser
            .get_or_init(|| PipeModeParser::new(self.command_line_args))
    }
}
