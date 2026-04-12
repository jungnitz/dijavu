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
        AppContainer, AppContainerBuilder, Data, DataKey, InitAppContainer, InitInjectable, Result,
        data::DataItem,
    };
    use std::{any::type_name, marker::PhantomData};

    pub use ::ctor;

    struct RuntimeKey<Injectable, Runtime>(PhantomData<(Injectable, Runtime)>);

    impl<Injectable: 'static, Runtime> DataKey for RuntimeKey<Injectable, Runtime>
    where
        Runtime: DataItem,
    {
        type Item = Runtime;
    }

    pub fn impl_init_injectable_get_init<'a, Injectable, Init, Runtime>(
        container: &'a mut InitAppContainer,
        construct: impl FnOnce(&mut InitAppContainer) -> Result<Init>,
        into_runtime: impl FnOnce(Init, &mut Data, &mut AppContainerBuilder) -> Result<Runtime>
        + 'static,
    ) -> Result<&'a mut Init>
    where
        Injectable: for<'i> InitInjectable<Init<'i> = &'i mut Init>,
        Init: DataItem,
        Runtime: DataItem,
    {
        struct InitKey<T>(PhantomData<T>);
        impl<T: DataItem> DataKey for InitKey<T> {
            type Item = T;
        }

        if container.data_mut().contains_key::<InitKey<Init>>() {
            return Ok(container.data_mut().get_mut::<InitKey<Init>>().unwrap());
        }

        let value = construct(container)?;
        container.on_build(move |data, builder| {
            let value = data.remove::<InitKey<Init>>().unwrap();
            let runtime = into_runtime(value, data, builder)?;
            builder.insert_app_data::<RuntimeKey<Injectable, Runtime>>(runtime)?;
            Ok(())
        });
        Ok(container
            .data_mut()
            .entry::<InitKey<Init>>()
            .insert_entry(value)
            .into_mut())
    }

    pub fn impl_init_injectable_get_runtime<Injectable, Runtime>(
        container: AppContainer,
    ) -> Result<&'static Runtime>
    where
        Injectable: 'static,
        Runtime: DataItem,
    {
        container
            .data()
            .get::<RuntimeKey<Injectable, Runtime>>()
            .ok_or_else(|| {
                dijavu::Error::msg(format!(
                    "could not get runtime data for {}: uninitialized",
                    type_name::<Injectable>()
                ))
            })
    }
}
