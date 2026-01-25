use bytesize::ByteSize;

use std::{collections::VecDeque, path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct StdinData(pub Option<Arc<String>>);

impl std::fmt::Display for StdinData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(s) => {
                let size_string = ByteSize::b(s.len().try_into().unwrap_or_default()).to_string();
                write!(f, "{size_string}")
            }
            None => write!(f, "None"),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct OwnedCommandAndArgs {
    pub command_path: PathBuf,
    pub args: Vec<String>,
    pub stdin: StdinData,
}

impl std::fmt::Display for OwnedCommandAndArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cmd={:?},args={:?},stdin={}",
            self.command_path, self.args, self.stdin
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
            stdin: StdinData(None),
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

    pub fn with_stdin(mut self, stdin: StdinData) -> Self {
        self.stdin = stdin;
        self
    }
}
