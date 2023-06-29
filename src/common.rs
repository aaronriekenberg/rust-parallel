use itertools::Itertools;

#[derive(Debug, Eq, PartialEq)]
pub struct OwnedCommandAndArgs(Vec<String>);

impl From<Vec<String>> for OwnedCommandAndArgs {
    fn from(v: Vec<String>) -> Self {
        Self(v)
    }
}

impl From<Vec<&str>> for OwnedCommandAndArgs {
    fn from(v: Vec<&str>) -> Self {
        Self(v.into_iter().map_into().collect())
    }
}

impl std::ops::Deref for OwnedCommandAndArgs {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for OwnedCommandAndArgs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::fmt::Display for OwnedCommandAndArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
