use crate::data::{Data, DataEntry};
use crate::injectable::{Injectable, ScopeInjectable};
use crate::{DataKey, Error, Result};

/// Immutable handle to application-wide runtime data.
///
/// `AppContainer` is the central access point for dependency injection during the runtime phase.
/// It is a thin wrapper around a [`&'static Data`](Data) instance constructed during initialization.
///
/// ## Usage
///
/// The primary way to access dependencies is via [`AppContainer::get`]:
///
/// ```rust,ignore
/// # fn example(container: AppContainer) -> Result<(), Error> {
/// let service = container.get::<MyService>()?;
/// # Ok(())
/// # }
/// ```
///
/// You can also access the underlying [`Data`] directly if needed, usually in the implementations
/// of [`Injectable`]:
///
/// ```rust
/// # use dijavu::{Data, AppContainer};
/// # let container = AppContainer::empty();
/// let data: &'static Data = container.data();
/// ```
///
/// ## Construction
///
/// See [`InitAppContainer`](crate::InitAppContainer) and [`InitInjectable`](crate::InitInjectable)
/// module for details on how to construct an `AppContainer`.
#[derive(Copy, Clone)]
pub struct AppContainer(&'static Data);

impl AppContainer {
    /// Creates an [`AppContainer`] with no data.
    pub fn empty() -> Self {
        Self(Box::leak(Box::new(Data::default())))
    }

    /// Returns the underlying [`Data`] storage.
    ///
    /// This is primarily intended for low-level access.
    pub fn data(self) -> &'static Data {
        self.0
    }

    /// Retrieves an [`Injectable`] from the container.
    ///
    /// This is the primary entry point for dependency injection.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # fn example(app: AppContainer) -> Result<(), Error> {
    /// let service = app.get::<MyService>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<I>(self) -> Result<I, I::Error>
    where
        I: Injectable,
    {
        I::get(self)
    }
}

/// Builder for constructing the [`AppContainer`] during initialization.
///
/// `AppContainerBuilder` is used during the _build phase_ to assemble the final runtime container
/// from initialization data.
/// It is typically accessed from within build hooks registered on the initialization container.
///
/// The builder separates data into two categories: _App data_ becomes part of the final
/// [`AppContainer`], while _init results_ are returned separately after build.
/// This allows initialization code to:
///
/// - construct runtime dependencies,
/// - extract and transform initialization state and
/// - return one-time artifacts (e.g. routers, startup handles)
#[derive(Default)]
pub struct AppContainerBuilder {
    app: Data,
    init_results: Data,
}

impl AppContainerBuilder {
    /// Inserts a value into the final [`AppContainer`].
    ///
    /// Returns an error if a value for this key has already been inserted.
    /// You should **really** aim to guarantee that no entry is inserted twice (e.g. using private
    /// key types).
    /// If you run into an error here, this usually means bad design on your end.
    pub fn insert_app_data<K>(&mut self, value: K::Item) -> Result<()>
    where
        K: DataKey,
    {
        let DataEntry::Vacant(entry) = self.app.entry::<K>() else {
            return Err(Error::msg("app data entry already present"));
        };
        entry.insert(value);
        Ok(())
    }

    /// Inserts a value into the initialization results storage.
    ///
    /// These values are returned separately after build and are intended for  one-time use during
    /// application startup.
    ///
    /// Returns an error if a value for this key has already been inserted.
    /// You should **really** aim to guarantee that no entry is inserted twice (e.g. using private
    /// key types).
    /// If you run into an error here, this usually means bad design on your end.
    pub fn insert_init_result<K>(&mut self, value: K::Item) -> Result<()>
    where
        K: DataKey,
    {
        let DataEntry::Vacant(entry) = self.init_results.entry::<K>() else {
            return Err(Error::msg("init result entry already present"));
        };
        entry.insert(value);
        Ok(())
    }

    /// Finalizes the build process.
    ///
    /// Consumes the builder and returns:
    ///
    /// - the constructed [`AppContainer`] (runtime data)
    /// - a [`Data`] instance containing init-only results
    ///
    /// The runtime data is leaked to `'static`, making the resulting [`AppContainer`] globally
    /// usable for the lifetime of the application.
    pub fn build(self) -> (AppContainer, Data) {
        (
            AppContainer(Box::leak(Box::new(self.app))),
            self.init_results,
        )
    }
}

/// Container supplementing an [`AppContainer`] with additional, short-lived data.
///
/// `ScopeContainer` extends [`AppContainer`] with a mutable, scope-local
/// [`Data`] store. It is intended for per-request / per-task state while still
/// providing access to global application data.
pub struct ScopeContainer {
    app: AppContainer,
    data: Data,
}

impl ScopeContainer {
    /// Creates a new scope from the given [`AppContainer`].
    pub fn new(container: AppContainer) -> Self {
        Self {
            app: container,
            data: Data::default(),
        }
    }

    /// Returns the underlying [`AppContainer`].
    pub fn app(&self) -> AppContainer {
        self.app
    }

    /// Returns mutable access to the scoped [`Data`].
    pub fn scope_data_mut(&mut self) -> &mut Data {
        &mut self.data
    }

    /// Returns shared access to the scoped [`Data`].
    pub fn scope_data(&self) -> &Data {
        &self.data
    }

    /// Retrieves a [`ScopeInjectable`] from this scope.
    ///
    /// This is the primary way to access scoped dependencies.
    pub fn get<'a, I>(&'a mut self) -> Result<I, I::Error>
    where
        I: ScopeInjectable<'a>,
    {
        I::get(self)
    }
}
