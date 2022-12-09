#[derive(Debug, Clone, Copy)]
pub enum Input {
    Stdin,

    File { file_name: &'static str },
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Input::Stdin => write!(f, "stdin"),
            Input::File { file_name } => write!(f, "file('{}')", file_name),
        }
    }
}
