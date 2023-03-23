pub type BorrowedCommandAndArgs<'a> = Vec<&'a str>;

impl<'a> From<&'a OwnedCommandAndArgs> for BorrowedCommandAndArgs<'a> {
    fn from(o: &'a OwnedCommandAndArgs) -> BorrowedCommandAndArgs<'a> {
        o.0.iter().map(|s| s.as_ref()).collect()
    }
}

#[derive(Debug)]
pub struct OwnedCommandAndArgs(pub Vec<String>);

impl From<BorrowedCommandAndArgs<'_>> for OwnedCommandAndArgs {
    fn from(a: BorrowedCommandAndArgs<'_>) -> OwnedCommandAndArgs {
        OwnedCommandAndArgs(a.into_iter().map(|s| s.to_owned()).collect())
    }
}
