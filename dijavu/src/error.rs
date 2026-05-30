use std::fmt::{Debug, Display, Formatter};

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A dynamic, type-erased error.
///
/// This error is used as the default error type throughout this crate.
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
    #[must_use]
    pub fn with_context<D>(self, context: D) -> Self
    where
        D: Display + Send + Sync + 'static,
    {
        Self(self.0.context(context))
    }

    /// Convert to a std error trait object
    #[must_use]
    pub fn into_std_error(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.0.into_boxed_dyn_error()
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
