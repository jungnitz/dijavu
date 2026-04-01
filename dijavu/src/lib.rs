#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]

extern crate self as dijavu;

mod container;
pub use self::container::{AppContainer, AppContainerBuilder, ScopeContainer};

/// Container for storing heterogeneous values via key types
pub mod data;
#[doc(inline)]
pub use self::data::{Data, DataKey};

mod error;
pub use self::error::{Error, Result};

mod init;
pub use init::{InitAppContainer, InitInjectable, Initializable, StartValue, Value};

/// Global hooks into the application lifecycle
pub mod hooks;

mod injectable;
pub use self::injectable::{Injectable, ScopeInjectable};

/// Derives [`InitInjectable`] and [`Injectable`] for a struct
///
/// Fields marked with `#[inject(init)]` are initialized via a generated *init struct*,
/// which is accessible during setup and consumed during build.
///
/// ## Example
///
/// ```rust
/// # use dijavu::*;
/// #[derive(InitInjectable)]
/// #[inject(init(auto, type = ServiceInitValue))]
/// pub struct Service {
///     dependency: (), // () implements Injectable
///     #[inject(init)]
///     init_value: Value<String>,
/// }
/// ```
///
/// Generates the additional initialization struct
///
/// ```rust
/// # use dijavu::{Initializable, Value};
/// pub struct ServiceInitValue {
///     init_value: <Value<String> as Initializable>::Init, // = String
/// }
/// ```
///
/// and can then be used like this:
///
/// ```rust
/// # use dijavu::*;
/// # #[derive(InitInjectable)]
/// # #[inject(init(auto, type = ServiceInitValue))]
/// # pub struct Service {
/// #     dependency: (), // () implements Injectable
/// #     #[inject(init)]
/// #     init_value: Value<String>,
/// # }
/// # tokio_test::block_on(async {
/// // initialization
/// let mut container: InitAppContainer = InitAppContainer::default();
/// let init: &mut ServiceInitValue = container.get::<Service>().unwrap();
/// init.init_value = "hello".to_owned();
///
/// // build
/// let container = container.build().await.unwrap();
///
/// // runtime
/// let service: Service = container.get::<Service>().unwrap();
/// assert_eq!(*service.init_value, "hello");
/// # });
/// ```
///
/// ## Attributes
///
/// ### Struct-level
///
/// | Attribute           | Description                                      | Default              |
/// |---------------------|--------------------------------------------------|----------------------|
/// | `init(auto)`        | Always initialize during build, even when not explicitly accessed | disabled |
/// | `init(type = Name`) | Name of generated init struct                    | `<StructName>Init`   |
/// | `init(on_construct = <expr>`) | Runs function `<expr>` with argument `&mut InitAppContainer` on first construction of the initialization value for an `InitAppContainer` | - |
/// | `init(on_build = <expr>`) | Runs function `<expr>` with arguments `&mut <InitStruct>, &mut Data, &mut AppContainerBuilder` where the second argument contains the initialization state | - |
/// | `init(on_start = <expr>`) | Runs function `<expr>` with arguments `AppContainer, &mut Data` on start (i.e. right after building the `AppContainer`) with the start data | - |
/// | `init(on_start_async = <expr>`) | Runs async function `<expr>` with arguments `AppContainer, &mut Data` on start (i.e. right after building the `AppContainer`) with the start data | - |
///
/// ### Field-level
///
/// | Attribute        | Description                                                               |
/// |------------------|---------------------------------------------------------------------------|
/// | `init`           | Marks a field to be added to the initialization struct using its [`Initializable`] implementation. |
pub use dijavu_macros::InitInjectable;

/// Derives [`Injectable`] for a struct.
///
/// Each field must be injectable and is retrieved using [`Injectable::get`].
/// This allows composing injectables without manually implementing [`Injectable`].
///
/// ## Example
///
/// ```rust
/// use dijavu::*;
/// #[derive(Injectable)]
/// pub struct Service {
///     dependency: (), // () implements Injectable
/// }
/// ```
///
/// Expands roughly to:
///
/// ```rust
/// # use dijavu::*;
/// # pub struct Service {
/// #     dependency: (),
/// # }
/// impl Injectable for Service {
///     type Error = Error;
///
///     fn get(container: AppContainer) -> Result<Self, Self::Error> {
///         Ok(Self {
///             dependency: <() as Injectable>::get(container)?,
///         })
///     }
/// }
/// ```
pub use dijavu_macros::Injectable;

#[doc(hidden)]
pub mod __private {
    use crate::{
        AppContainerBuilder, Data, DataKey, InitAppContainer, InitInjectable, Result,
        data::DataItem,
    };
    use std::marker::PhantomData;

    pub use ::ctor;

    pub fn init_injectable_get_init<'a, I, T: DataItem>(
        container: &'a mut InitAppContainer,
        construct: impl FnOnce(&mut InitAppContainer) -> Result<T>,
        on_build: impl FnOnce(T, &mut Data, &mut AppContainerBuilder) -> Result<()> + 'static,
    ) -> Result<&'a mut T>
    where
        I: InitInjectable<Init<'a> = &'a mut T>,
    {
        struct Key<I, T>(PhantomData<(I, T)>);
        impl<I: 'static, T: DataItem> DataKey for Key<I, T> {
            type Item = T;
        }
        if container.data_mut().contains_key::<Key<I, T>>() {
            return Ok(container.data_mut().get_mut::<Key<I, T>>().unwrap());
        }
        let value = construct(container)?;
        container.on_build(move |init, builder| {
            let value = init.remove::<Key<I, T>>().unwrap();
            on_build(value, init, builder)
        });
        Ok(container
            .data_mut()
            .entry::<Key<I, T>>()
            .insert_entry(value)
            .into_mut())
    }
}
