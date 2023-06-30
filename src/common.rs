use std::{collections::VecDeque, path::PathBuf};

#[derive(Debug, Eq, PartialEq)]
pub struct OwnedCommandAndArgs {
    pub command_path: PathBuf,
    pub args: Vec<String>,
}

impl OwnedCommandAndArgs {
    pub fn append_arg(&self, arg: String) -> Self {
        Self {
            command_path: self.command_path.clone(),
            args: [self.args.clone(), vec![arg]].concat(),
        }
    }
}

impl std::fmt::Display for OwnedCommandAndArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}", self.command_path, self.args)
    }
}

impl TryFrom<VecDeque<String>> for OwnedCommandAndArgs {
    type Error = &'static str;

    fn try_from(mut deque: VecDeque<String>) -> Result<Self, Self::Error> {
        let command = match deque.pop_front() {
            Some(command) => command,
            None => return Err("deque is empty"),
        };

        Ok(Self {
            command_path: PathBuf::from(command),
            args: deque.into(),
        })
    }
}

impl TryFrom<Vec<String>> for OwnedCommandAndArgs {
    type Error = &'static str;

    fn try_from(vec: Vec<String>) -> Result<Self, Self::Error> {
        Self::try_from(VecDeque::from(vec))
    }
}
