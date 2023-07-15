use crate::command_line_args::CommandLineArgs;

pub mod buffered;
pub mod command_line;

fn build_shell_command_and_args(command_line_args: &CommandLineArgs) -> Option<Vec<String>> {
    if command_line_args.shell {
        Some(vec![command_line_args.shell_path.clone(), "-c".to_owned()])
    } else {
        None
    }
}
