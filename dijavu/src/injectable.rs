use crate::initialization::InitInjector;
use crate::{Error, InjectorBuilder, Restricted};
#[cfg(doc)]
use crate::{Initializable, Injector, NewInitValue};
use std::convert::Infallible;

/// The central type for dependency injection.
pub trait Injectable: Send + Sync + Sized + 'static {
    /// The error type returned when construction of the initialization value fails.
    type Error: Into<Error>;

    /// Initialization state of this type
    ///
    /// This type may borrow mutably from a [`InitInjector`] with lifetime `'a`.
    type Init<'a>;

    /// Create an initialization value for this injectable.
    ///
    /// # Restriction
    /// The purpose of the restriction here is to prevent users of this library calling this
    /// function manually, which will not automatically enqueue this type for the build process.
    /// Use [`InitInjector::get`] instead.
    fn init(
        injector: &mut InitInjector,
        token: Restricted,
    ) -> impl Future<Output = Result<Self::Init<'_>, Self::Error>> + Send;

    /// Build this injectable.
    ///
    /// # Restriction
    /// Calls to this method are restricted to dijavu itself as the build process should be fully
    /// under control of [`InjectorBuilder`].
    /// Otherwise, a user of this library might accidentally attempt to build a second instance of
    /// an [`Injectable`].
    /// Consider using [`Inject`](crate::initializables::Inject) instead.
    fn build(
        builder: &mut InjectorBuilder,
        token: Restricted,
    ) -> impl Future<Output = crate::Result<Self>> + Send;
}

impl Injectable for () {
    type Error = Infallible;
    type Init<'a> = ();

    async fn init(
        _injector: &mut InitInjector,
        _token: Restricted,
    ) -> Result<Self::Init<'_>, Self::Error> {
        Ok(())
    }

    async fn build(_builder: &mut InjectorBuilder, _token: Restricted) -> crate::Result<Self> {
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
/// #[inject(init(ident = ServiceInitValue))]
/// pub struct Service {
///     dependency: Inject<AnotherService>,
///     init_value: Value<String>,
/// }
/// ```
///
/// Generates the additional initialization struct
///
/// ```
/// # use dijavu::*;
/// # use dijavu::initializables::{Inject, Value};
/// # type MyService = ();
/// pub struct ServiceInitValue {
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
/// # #[inject(init(ident = ServiceInitValue))]
/// # pub struct Service {
/// #     init_value: Value<String>,
/// # }
/// # tokio_test::block_on(async {
/// // initialization
/// let mut init_injector: InitInjector = InitInjector::default();
/// let init: &mut ServiceInitValue = init_injector.get::<Service>().await.unwrap();
/// init.init_value = "hello".to_owned();
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
/// | `init(ident = <Name>)`  | Name of generated init struct                    | `<StructName>Init`   |
/// | `init(hide)`            | Hide init struct in a `const _: ()`-block        | disabled             |
/// | `init(hook = <expr>)`   | Runs async function `<expr>` with arguments `&mut <InitStruct>, &mut InitInjector` the first time that [`init`](Injectable::init) is called for an `InitInjector` and after the init struct was successfully created | - |
/// | `build(hook = <expr>)`  | Runs async function `<expr>` with arguments `&mut <InitStruct>, &mut InjectorBuilder` when [`build`](Injectable::build) is called | - |
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
