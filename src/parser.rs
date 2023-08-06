use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs};

pub mod buffered;
pub mod command_line;

fn build_shell_command_and_args(command_line_args: &CommandLineArgs) -> Option<Vec<String>> {
    if command_line_args.shell {
        Some(vec![command_line_args.shell_path.clone(), "-c".to_owned()])
    } else {
        None
    }
}

fn prepend_shell_command_and_args(
    shell_command_and_args: &[String],
    command_and_args: Vec<String>,
) -> Option<OwnedCommandAndArgs> {
    let merged_args = vec![command_and_args.join(" ")];
    let cmd_and_args = [shell_command_and_args.to_owned(), merged_args].concat();
    OwnedCommandAndArgs::try_from(cmd_and_args).ok()
}
