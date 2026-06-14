#[cfg(doc)]
use crate::Injectable;
use crate::{Error, InitInjector, InjectorBuilder, data::DataValue};
use std::convert::Infallible;

/// Building block for [`Injectable`] types
///
/// `Initializable` defines the behavior of a field when using the `Injectable` derive macro on a
/// struct.
/// In particular, it defines
///
/// - the type of the field in the initialization struct,
/// - how the initialization data is transformed into an instance of this type.
///
/// ## Implementations
///
/// dijavu provides some simple, but useful implementations of the trait in the
/// [`initializables`](crate::initializables) module.
pub trait Initializable: Sized {
    /// The type of the data that is modifiable during initialization
    type Init: DataValue;

    /// Consumes the initialization value and constructs an instance of this type.
    fn build(
        init: Self::Init,
        builder: &mut InjectorBuilder,
    ) -> impl Future<Output = crate::Result<Self>> + Send;
}

/// Defines how the default init value of an [`Initializable`] can be constructed.
///
/// This trait primarily is used by the derive macros for `Injectable` and `Initializable` to create
/// an instance of the init value for each field in the struct that the macro is applied to.
/// If the trait is not implemented, or if you want to provide your own builders, you can use the
/// field-level `#[inject(init = ...)]` attribute of both macros.
pub trait NewInitValue: Initializable {
    type Error: Into<Error>;
    /// Creates a new initialization value.
    fn new_init(
        injector: &mut InitInjector,
    ) -> impl Future<Output = Result<Self::Init, Self::Error>> + Send;
}

/// Derives [`Initializable`] for a struct with fields that implement [`Initializable`].
///
/// ## Attributes
///
/// ### Struct-level
///
/// | Attribute               | Description                                      | Default              |
/// |-------------------------|--------------------------------------------------|----------------------|
/// | `init(hide)`            | Apply `#[doc(hidden)` to the init struct         | disabled             |
/// | `init(hook = <expr>)`   | Runs async function `<expr>` with arguments `&mut <InitStruct>, &mut InitInjector` when [`new_init`](NewInitValue::new_init) is called and after the init struct was successfully created | - |
/// | `build(hook = <expr>)`  | Runs async function `<expr>` with arguments `&mut <InitStruct>, &mut InjectorBuilder` when [`build`](Initializable::build) is called | - |
///
/// All hook functions must return a `Result<(), impl Into<dijavu::Error>>`
///
/// ### Field-level
///
/// | Attribute           | Description                                      | Default              |
/// |---------------------|--------------------------------------------------|----------------------|
/// | `init = <expr>`     | Uses async function `<expr>` with signature `(&mut InitInjector) -> Result<<T as Initializable>::Init, impl Into<dijavu::Error>>` to construct the init value of the field instead of the field type's implementation of [`NewInitValue`] | - |
///
pub use dijavu_macros::Initializable;

impl Initializable for () {
    type Init = ();

    async fn build(_init: Self::Init, _builder: &mut InjectorBuilder) -> crate::Result<Self> {
        Ok(())
    }
}

impl NewInitValue for () {
    type Error = Infallible;

    async fn new_init(_injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(())
    }
}
