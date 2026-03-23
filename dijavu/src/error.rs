use std::fmt::{Debug, Display, Formatter};

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A dynamic, type-erased error.
///
/// This error is used as the default error for derived implementations with fallible methods as
/// well as for functions where multiple or dynamic error types could arise.
pub struct Error(anyhow::Error);

impl Error {
    /// Creates a new erased error from any error type.
    pub fn new<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self(anyhow::Error::new(error))
    }

    /// Creates an error from a message.
    ///
    /// Useful for quick error construction without defining a type.
    ///
    /// ```
    /// # use dijavu::Error;
    /// let err = Error::msg("value not present");
    /// ```
    pub fn msg<D>(msg: D) -> Self
    where
        D: Display + Debug + Send + Sync + 'static,
    {
        Self(anyhow::Error::msg(msg))
    }

    /// Adds context to an error, improving diagnostics.
    ///
    /// ```
    /// # use dijavu::Error;
    /// let err = Error::msg("value not present")
    ///     .with_context("injecting field `thing` of `Struct` failed");
    /// ```
    pub fn with_context<D>(self, context: D) -> Self
    where
        D: Display + Send + Sync + 'static,
    {
        Self(self.0.context(context))
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(value: E) -> Self {
        Self(value.into())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
