use crate::Error;

/// The return value of a hook given via `#[injectable(...)]`: either `()` for hooks that cannot
/// fail, or `Result<(), E>` with `E: Into<Error>`.
pub trait HookResult {
    /// Converts the return value into a `Result`.
    fn into_result(self) -> Result<(), Error>;
}

impl HookResult for () {
    fn into_result(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<E: Into<Error>> HookResult for Result<(), E> {
    fn into_result(self) -> Result<(), Error> {
        self.map_err(Into::into)
    }
}
