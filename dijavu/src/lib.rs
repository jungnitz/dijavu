#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]

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
pub use init::{Dependency, DropValue, InitAppContainer, Initializable, StartValue, Value};

/// Global hooks into the application lifecycle
pub mod hooks;

mod injectable;
pub use self::injectable::{Injectable, ScopeInjectable};

/// Derives [`Injectable`] for a struct.
///
/// There are two ways, in which fields may be constructed:
/// First, fields marked with `#[inject]` indicate [`Injectable`] types on which this type depends.
/// The implementation will ensure that these types are initialized before this type.
/// All other fields allow configuration during the initialization phase by utilizing their
/// [`Initializable`] implementation.
///
/// ## Example
///
/// ```rust
/// # use dijavu::*;
/// #[derive(Injectable)]
/// #[inject(init(auto, type = ServiceInitValue))]
/// pub struct Service {
///     #[inject]
///     dependency: (), // () implements Injectable
///     init_value: Value<String>,
/// }
/// ```
///
/// Generates the additional initialization struct
///
/// ```rust
/// # use dijavu::{Dependency, Initializable, Value};
/// pub struct ServiceInitValue {
///     dependency: Dependency<()>,
///     init_value: <Value<String> as Initializable>::Init, // = String
/// }
/// ```
///
/// and can then be used like this:
///
/// ```rust
/// # use dijavu::*;
/// # #[derive(Injectable)]
/// # #[inject(init(auto, type = ServiceInitValue))]
/// # pub struct Service {
/// #     #[inject]
/// #     dependency: (), // () implements Injectable
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
/// | `init(hide)`        | Hide init struct in a `const _: ()`-block        | disabled             |
/// | `init(on_construct = <expr>`) | Runs function `<expr>` with argument `&mut InitAppContainer` on first construction of the initialization value for an `InitAppContainer` | - |
/// | `init(on_build = <expr>`) | Runs function `<expr>` with arguments `&mut <InitStruct>, &mut Data, &mut AppContainerBuilder` on build | - |
/// | `init(on_build_async = <expr>`) | Runs async function `<expr>` with arguments `&mut <InitStruct>, &mut Data, &mut AppContainerBuilder` on build | - |
/// | `init(on_start = <expr>`) | Runs function `<expr>` with arguments `AppContainer, &mut Data` on start (i.e. right after building the `AppContainer`) with the start data | - |
/// | `init(on_start_async = <expr>`) | Runs async function `<expr>` with arguments `AppContainer, &mut Data` on start (i.e. right after building the `AppContainer`) with the start data | - |
pub use dijavu_macros::Injectable;

use crate::data::{LeakedValues, define_data_wrapper};

define_data_wrapper!(
    /// Data that is assembled during the initialization phase
    pub InitData;
);
define_data_wrapper!(
    /// Data that is assembled during the build phase and is available in the start hooks
    pub BuildData;
);
define_data_wrapper!(
    /// Data that is assembled during the build phase and is available from the [`AppContainer`]
    pub RuntimeData -> LeakedValues;
);
define_data_wrapper!(
    /// Data that is local to a [`ScopeContainer`]
    pub ScopeData;
);

#[doc(hidden)]
pub mod __private {
    use crate::{
        AppContainerBuilder, DataKey, InitAppContainer, InitData, Injectable, Result, RuntimeData,
        data::DataValue,
    };
    use std::{any::type_name, marker::PhantomData, pin::Pin};

    pub use ::ctor;

    struct RuntimeKey<Injectable, Runtime>(PhantomData<(Injectable, Runtime)>);

    impl<Injectable: 'static, Runtime> DataKey for RuntimeKey<Injectable, Runtime>
    where
        Runtime: DataValue,
    {
        type Item = Runtime;
    }

    pub fn impl_injectable_get_init<'a, Inject, Init, Runtime>(
        container: &'a mut InitAppContainer,
        construct: impl FnOnce(&mut InitAppContainer) -> Result<Init>,
        into_runtime: impl for<'f> FnOnce(
            Init,
            &'f mut InitData,
            &'f mut AppContainerBuilder,
        )
            -> Pin<Box<dyn Future<Output = Result<Runtime>> + Send + 'f>>
        + Send
        + 'static,
    ) -> Result<&'a mut Init>
    where
        Inject: for<'i> Injectable<Init<'i> = &'i mut Init>,
        Init: DataValue,
        Runtime: DataValue,
    {
        struct InitKey<T>(PhantomData<T>);
        impl<T: DataValue> DataKey for InitKey<T> {
            type Item = T;
        }

        if container.data_mut().contains_key::<InitKey<Init>>() {
            return Ok(container.data_mut().get_mut::<InitKey<Init>>().unwrap());
        }

        let value = construct(container)?;
        container.on_build_async(move |data, builder| {
            Box::pin(async move {
                let value = data.remove::<InitKey<Init>>().unwrap();
                let runtime = into_runtime(value, data, builder).await?;
                builder.insert_app_data::<RuntimeKey<Inject, Runtime>>(runtime)?;
                Ok(())
            })
        });
        Ok(container
            .data_mut()
            .entry::<InitKey<Init>>()
            .insert_entry(value)
            .into_mut())
    }

    pub fn impl_injectable_get_runtime<Injectable, Runtime>(
        data: &RuntimeData,
    ) -> Result<&'static Runtime>
    where
        Injectable: 'static,
        Runtime: DataValue,
    {
        data.get::<RuntimeKey<Injectable, Runtime>>()
            .ok_or_else(|| {
                dijavu::Error::msg(format!(
                    "could not get runtime data for {}: uninitialized",
                    type_name::<Injectable>()
                ))
            })
    }
}
