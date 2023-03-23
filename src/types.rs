pub type BorrowedCommandAndArgs<'a> = Vec<&'a str>;

impl<'a> From<&'a OwnedCommandAndArgs> for BorrowedCommandAndArgs<'a> {
    fn from(owned: &'a OwnedCommandAndArgs) -> BorrowedCommandAndArgs<'a> {
        owned.0.iter().map(|s| s.as_ref()).collect()
    }
}

#[derive(Debug)]
pub struct OwnedCommandAndArgs(pub Vec<String>);

impl From<BorrowedCommandAndArgs<'_>> for OwnedCommandAndArgs {
    fn from(borrowed: BorrowedCommandAndArgs<'_>) -> OwnedCommandAndArgs {
        OwnedCommandAndArgs(borrowed.into_iter().map(|s| s.to_owned()).collect())
    }
}
