use crate::data::DataValue;
use crate::initialization::InitInjector;
use crate::{Error, InjectableInit, InjectorBuilder, Restricted};
#[cfg(doc)]
use crate::{Initializable, Injector, NewInitValue};
use std::convert::Infallible;

/// The central type for dependency injection.
pub trait Injectable: Send + Sync + Sized + 'static {
    /// The error type returned when construction of the initialization data fails.
    type Error: Into<Error>;

    /// Value stored in the initialization data storage for this injectable.
    type Data: DataValue;

    /// Initialization state of this type
    ///
    /// The purpose of separating this type from [`Self::Data`] is that a value of this type
    /// typically is a wrapper around an [`InjectableInit`], which allows accessing the injectable's
    /// init data *and* the entire [`InitInjector`] state (but of course not at the same time, that
    /// would be wholly unsound).
    type Init<'a>;

    /// Create the initialization data for this injectable.
    ///
    /// This method is called exactly once during the initialization or build phase for each
    /// injectable when it is referenced during the initialization phase.
    ///
    /// # Restriction
    /// The purpose of the restriction is to prevent users of this library calling this function
    /// manually, which will not automatically enqueue this type for the build process.
    /// Use [`InitInjector::get`] instead.
    fn new_init_data(
        injector: &mut InitInjector,
        token: Restricted<Self>,
    ) -> impl Future<Output = Result<Self::Data, Self::Error>> + Send;

    /// Builds the initialization state
    ///
    /// This method is called once for every call to [`InitInjector::get`].
    fn new_init(init: InjectableInit<'_, Self>) -> Self::Init<'_>;

    /// Build this injectable.
    ///
    /// # Restriction
    /// Calls to this method are restricted to dijavu itself as the build process should be fully
    /// under control of [`InjectorBuilder`].
    /// Otherwise, a user of this library might accidentally attempt to build a second instance of
    /// an [`Injectable`].
    /// Consider using [`Inject`](crate::initializables::Inject) instead.
    fn build(
        data: Self::Data,
        builder: &mut InjectorBuilder,
        token: Restricted<Self>,
    ) -> impl Future<Output = crate::Result<Self>> + Send;
}

impl Injectable for () {
    type Error = Infallible;
    type Data = ();
    type Init<'a> = ();

    async fn new_init_data(
        _injector: &mut InitInjector,
        _token: Restricted<Self>,
    ) -> Result<Self::Init<'_>, Self::Error> {
        Ok(())
    }

    fn new_init(_init: InjectableInit<'_, Self>) -> Self::Init<'_> {}

    async fn build(
        _data: Self::Data,
        _builder: &mut InjectorBuilder,
        _token: Restricted<Self>,
    ) -> crate::Result<Self> {
        Ok(())
    }
}

/// Derives [`Injectable`] for a struct with fields that implement [`Initializable`].
///
/// ## Example
///
/// ```
/// # use dijavu::*;
/// # use dijavu::initializables::{Inject, Value};
/// # type AnotherService = ();
/// #[derive(Injectable)]
/// pub struct Service {
///     dependency: Inject<AnotherService>,
///     init_value: Value<String>,
/// }
/// ```
///
/// Generates the additional initialization data struct
///
/// ```
/// # use dijavu::*;
/// # use dijavu::initializables::{Inject, Value};
/// # type MyService = ();
/// pub struct ServiceInitData {
///     dependency: <Inject<MyService> as Initializable>::Init, // = ()
///     init_value: <Value<String> as Initializable>::Init, // = String
/// }
/// ```
///
/// and can then be used like this:
///
/// ```
/// # use dijavu::*;
/// # use dijavu::initializables::Value;
/// # #[derive(Injectable)]
/// # pub struct Service {
/// #     init_value: Value<String>,
/// # }
/// # tokio_test::block_on(async {
/// // initialization
/// let mut init_injector: InitInjector = InitInjector::default();
/// let mut init = init_injector.get::<Service>().await.unwrap();
/// let mut data: &mut ServiceInitData = init.0.data_mut();
/// data.init_value = "hello".to_owned();
/// // build
/// let injector: Injector = init_injector.build().await.unwrap();
/// // runtime
/// let service: &Service = injector.get();
/// assert_eq!(*service.init_value, "hello");
/// # });
/// ```
///
/// ## Attributes
///
/// ### Struct-level
///
/// | Attribute               | Description                                      | Default              |
/// |-------------------------|--------------------------------------------------|----------------------|
/// | `init(auto)`            | Always initialize during build, even when not explicitly accessed | disabled |
/// | `init(hide)`            | Apply `#[doc(hidden)]` to init struct            | disabled             |
/// | `init(data(hide = <bool>))` | Apply `#[doc(hidden)]` to init data struct   | enabled              |
/// | `init(hook = <expr>)`   | Runs async function `<expr>` with argument `<InitStruct>` whenever [`new_init_data`](Injectable::new_init_data) is called and after the init struct was successfully created | - |
/// | `build(hook = <expr>)`  | Runs async function `<expr>` with arguments `&mut <InitDataStruct>, &mut InjectorBuilder` when [`build`](Injectable::build) is called | - |
///
/// All hook functions must return a `Result<(), impl Into<dijavu::Error>>`
///
/// ### Field-level
///
/// | Attribute           | Description                                      | Default              |
/// |---------------------|--------------------------------------------------|----------------------|
/// | `init = <expr>`     | Uses async function `<expr>` with signature `(&mut InitInjector) -> Result<<T as Initializable>::Init, impl Into<dijavu::Error>>` to construct the init value of the field instead of the field type's implementation of [`NewInitValue`] | - |
///
pub use dijavu_macros::Injectable;
