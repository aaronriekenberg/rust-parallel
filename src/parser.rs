use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs};

pub mod buffered;
pub mod command_line;
mod regex;

struct ShellCommandAndArgs(Option<Vec<String>>);

fn build_shell_command_and_args(command_line_args: &CommandLineArgs) -> ShellCommandAndArgs {
    ShellCommandAndArgs(if command_line_args.shell {
        Some(vec![command_line_args.shell_path.clone(), "-c".to_owned()])
    } else {
        None
    })
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
