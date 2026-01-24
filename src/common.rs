use std::{collections::VecDeque, path::PathBuf, sync::Arc};

#[derive(Debug, Eq, PartialEq, Default)]
pub struct OwnedCommandAndArgs {
    pub command_path: PathBuf,
    pub args: Vec<String>,
    pub stdin: Option<Arc<String>>,
}

impl std::fmt::Display for OwnedCommandAndArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stdin = match &self.stdin {
            Some(_) => "Some",
            None => "None",
        };
        write!(
            f,
            "cmd={:?},args={:?},stdin={:?}",
            self.command_path, self.args, stdin
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum OwnedCommandAndArgsConversionError {
    #[error("empty input")]
    EmptyInput,
}

impl TryFrom<VecDeque<String>> for OwnedCommandAndArgs {
    type Error = OwnedCommandAndArgsConversionError;

    fn try_from(mut deque: VecDeque<String>) -> Result<Self, Self::Error> {
        let command = deque
            .pop_front()
            .ok_or(OwnedCommandAndArgsConversionError::EmptyInput)?;

        Ok(Self {
            command_path: PathBuf::from(command),
            args: deque.into(),
            stdin: None,
        })
    }
}

impl TryFrom<Vec<String>> for OwnedCommandAndArgs {
    type Error = OwnedCommandAndArgsConversionError;

    fn try_from(vec: Vec<String>) -> Result<Self, Self::Error> {
        Self::try_from(VecDeque::from(vec))
    }
}

impl OwnedCommandAndArgs {
    pub fn with_command_path(mut self, command_path: PathBuf) -> Self {
        self.command_path = command_path;
        self
    }

    pub fn with_stdin(mut self, stdin: Arc<String>) -> Self {
        self.stdin = Some(stdin);
        self
    }
}
