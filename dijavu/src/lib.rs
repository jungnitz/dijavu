#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]

extern crate self as dijavu;

mod build;
pub use self::build::InjectorBuilder;

mod injector;
pub use self::injector::Injector;

/// Container for storing heterogeneous values via key types
pub mod data;
#[doc(inline)]
pub use self::data::{Data, DataKey};

mod error;
pub use self::error::{Error, Result};

/// Global hooks into the application lifecycle
pub mod hooks;

mod initialization;
pub use self::initialization::{
    InitInjector,
    initializable::{Initializable, NewInitValue},
    initializables,
};

mod injectable;
pub use self::injectable::Injectable;

mod on_start;
pub use self::on_start::OnStart;

use std::marker::PhantomData;

use crate::data::define_data_wrapper;

define_data_wrapper!(
    /// Data that is assembled during the initialization phase
    pub InitData;
);
define_data_wrapper!(
    /// Data that is assembled during the build phase and is available from the [`Injector`].
    ///
    /// Contains entries with key of type `InjectableKey` for all known injectables.
    InjectablesData;
);

/// Prevents some trait functions to be called from non-dijavu-code.
pub struct Restricted(());

struct InjectableKey<I: Injectable>(PhantomData<I>);

impl<I: Injectable> DataKey for InjectableKey<I> {
    type Value = &'static I;
}

#[doc(hidden)]
pub mod __private {
    pub use ctor;
}
