//! The errors that can occur during a build.

use std::{borrow::Cow, convert::Infallible, error::Error as StdError};

use thiserror::Error;

/// Errors that may occur during build
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    /// Creates an error from any other error, e.g. to return it from a hook.
    pub fn custom(error: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self::new(ErrorKind::Custom(error))
    }

    /// Returns what kind of error this is.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn into_custom(self) -> Result<Box<dyn StdError + Send + Sync + 'static>, Self> {
        match self.kind {
            ErrorKind::Custom(custom) => Ok(custom),
            _ => Err(self),
        }
    }
}

impl From<Infallible> for Error {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

/// The kinds of [`Error`](struct@Error).
#[derive(Debug, Error)]
#[error(transparent)]
#[non_exhaustive]
pub enum ErrorKind {
    /// See [`CycleError`].
    Cycle(CycleError),
    /// See [`BindingConflictError`].
    BindingConflict(BindingConflictError),
    /// See [`UnboundError`].
    Unbound(UnboundError),
    /// An error created via [`Error::custom`].
    Custom(Box<dyn StdError + Send + Sync + 'static>),
}

/// A dependency cycle was detected
#[derive(Debug, thiserror::Error)]
#[error("a dependency cycle containing `{injectable}` was detected")]
pub struct CycleError {
    injectable: Cow<'static, str>,
}

impl CycleError {
    pub(crate) fn new(injectable: Cow<'static, str>) -> Self {
        Self { injectable }
    }
}

impl From<CycleError> for Error {
    fn from(value: CycleError) -> Self {
        Self::new(ErrorKind::Cycle(value))
    }
}

/// A target was bound to two different injectables
#[derive(Debug, thiserror::Error)]
#[error("`{target}` cannot be bound to `{injectable}`, it is already bound to `{existing}`")]
pub struct BindingConflictError {
    target: &'static str,
    existing: Cow<'static, str>,
    injectable: Cow<'static, str>,
}

impl BindingConflictError {
    pub(crate) fn new(
        target: &'static str,
        existing: Cow<'static, str>,
        injectable: Cow<'static, str>,
    ) -> Self {
        Self {
            target,
            existing,
            injectable,
        }
    }
}

impl From<BindingConflictError> for Error {
    fn from(value: BindingConflictError) -> Self {
        Self::new(ErrorKind::BindingConflict(value))
    }
}

/// An injectable references a target that has not been bound to any injectable
#[derive(Debug, thiserror::Error)]
#[error("`{injectable}` references `{target}`, which is not bound to any injectable")]
pub struct UnboundError {
    target: &'static str,
    injectable: Cow<'static, str>,
}

impl UnboundError {
    pub(crate) fn new(target: &'static str, injectable: Cow<'static, str>) -> Self {
        Self { target, injectable }
    }
}

impl From<UnboundError> for Error {
    fn from(value: UnboundError) -> Self {
        Self::new(ErrorKind::Unbound(value))
    }
}
