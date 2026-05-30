use crate::InjectableKey;
use crate::InjectablesData;
use crate::injectable::Injectable;
use std::any::type_name;

/// Immutable handle to a set of injectables.
///
/// `Injector` is the central access point for dependency injection at runtime.
/// It stores all injectables that were constructed during the build phase.
///
/// ## Usage
///
/// The primary way to access dependencies is via [`get`](Self::get):
///
/// ```
/// # use dijavu::{Error, Injector};
/// # type MyService = ();
/// # fn example(injector: Injector) -> Result<(), Error> {
/// let service: &MyService = injector.get();
/// # Ok(())
/// # }
/// ```
///
/// Note that this method will panic if the requested injectable is unknown to the `Injector`.
/// If you want to avoid this, use [`get_opt`](Self::get_opt).
///
/// ## Construction
///
/// See [`InitInjector`](crate::InitInjector) and [`Injectable`](crate::Injectable)
/// module for details on how to construct an `Injector`.
#[derive(Copy, Clone)]
pub struct Injector(&'static InjectablesData);

impl Injector {
    pub(crate) fn new(data: &'static InjectablesData) -> Self {
        Self(data)
    }

    /// Creates an [`Injector`] containing no injectables.
    #[must_use]
    pub fn empty() -> Self {
        Self(Box::leak(Box::new(InjectablesData::default())))
    }

    /// If present, retrieves an [`Injectable`] from the injector.
    ///
    /// # Example
    ///
    /// ```
    /// # use dijavu::{Error, Injector};
    /// # type MyService = ();
    /// # fn example(injector: Injector) -> Result<(), Error> {
    /// let service: Option<&MyService> = injector.get_opt();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_opt<I>(self) -> Option<&'static I>
    where
        I: Injectable,
    {
        self.0.get::<InjectableKey<I>>().copied()
    }

    /// Retrieves an [`Injectable`] from the injector.
    ///
    /// # Panics
    ///
    /// This method panics if no injectable of the given type exists in this injector.
    ///
    /// # Example
    ///
    /// ```
    /// # use dijavu::{Error, Injector};
    /// # type MyService = ();
    /// # fn example(injector: Injector) -> Result<(), Error> {
    /// let service: &MyService = injector.get();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get<I>(self) -> &'static I
    where
        I: Injectable,
    {
        let Some(inj) = self.0.get::<InjectableKey<I>>() else {
            panic!(
                "Injector does not contain injectable `{}`",
                type_name::<I>()
            )
        };
        inj
    }
}
