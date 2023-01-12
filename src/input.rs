use crate::command_line_args;

#[derive(Debug, Clone, Copy)]
pub enum Input {
    Stdin,

    File { file_name: &'static str },
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Input::Stdin => write!(f, "stdin"),
            Input::File { file_name } => write!(f, "{}", file_name),
        }
    }
}

#[derive(Debug)]
pub struct InputLineNumber {
    pub input: Input,
    pub line_number: u64,
}

impl std::fmt::Display for InputLineNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.input, self.line_number)
    }
}

pub fn build_input_list() -> Vec<Input> {
    let command_line_args = command_line_args::instance();
    if command_line_args.input.is_empty() {
        vec![Input::Stdin]
    } else {
        command_line_args
            .input
            .iter()
            .map(|input_name| {
                if input_name == "-" {
                    Input::Stdin
                } else {
                    Input::File {
                        file_name: input_name,
                    }
                }
            })
            .collect()
    }
}
