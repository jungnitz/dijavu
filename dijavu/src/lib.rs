#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]

mod build;
pub mod error;
pub mod fields;
mod injectable;
mod injector;
mod refs;
pub mod state;
mod target;

pub use build::{
    AssembleContext, Builder, DeclareContext, FieldAssembleContext, FieldDeclareContext,
    FieldPrepareContext, PrepareContext, PreparingHandle, Registered, RegisteredValues,
    StartContext,
};
#[doc(inline)]
pub use error::Error;
#[doc(inline)]
pub use fields::InjectableField;
pub use injectable::{Injectable, Preparing};
pub use injector::Injector;
pub use refs::{Bindable, RefVisitor, UnsizeFrom, Visitable};
pub use target::Target;

/// Derives [`Injectable`] and [`Visitable`] for a struct whose fields all implement
/// [`InjectableField`].
///
/// For a struct `Foo`, this also generates `FooState<S>`, which has the same fields as `Foo`, but
/// holds the data of the build step `S` (see [`state`]) in each of them, as well as a `ctx` field
/// (the last field for tuple structs). Add methods to `FooState<state::Preparing<'_>>` to allow
/// other injectables to configure `Foo`, see [`FieldPrepareContext::get`].
///
/// `FooState<Start>` holds the [`InjectableField::Start`] of each field and is the
/// [`Injectable::Start`] of `Foo`.
///
/// The following attributes can be used as `#[injectable(...)]` on the struct. All hooks return a
/// `Result<(), E>` with `E: Into<Error>`.
///
/// - `declare = hook`: `fn(&mut FooState<Declare>, DeclareContext<'_, Self>)`, runs after the
///   fields have been declared
/// - `bind(dyn Trait, ...)`: binds the given targets to this injectable
/// - `register(expr, ...)`: registers the value of each expression after the `declare` hook, see
///   [`FieldDeclareContext::register`]. The type of the expression decides what is registered, so trait
///   objects need a cast, like `Box::new(MyPlugin) as Box<dyn Plugin>`.
/// - `prepare = hook`: `async fn(&mut FooState<Declare>, PrepareContext<'_, Self>)`, runs before the
///   fields are prepared
/// - `prepared = hook`: `async fn(&mut FooState<Prepare>, PrepareContext<'_, Self>)`, runs after
///   the fields have been prepared
/// - `assemble = hook`: `async fn(&mut FooState<Prepare>, AssembleContext<'_, Self>)`, runs before
///   the fields are assembled
/// - `start = hook`: `async fn(&Foo, FooState<Start>, StartContext<'_, Self>)`, implements
///   [`Injectable::start`]
///
/// Fields can be excluded from visiting via `#[visitable(skip)]`.
///
/// # Generics
///
/// Only the bounds written on the struct are used, like `T: Target` for an `Inject<T>` field or
/// `T: InjectableField` for a field of type `T`.
pub use dijavu_macros::Injectable;

/// Derives [`InjectableField`] and [`Visitable`] for a struct whose fields all implement
/// [`InjectableField`].
///
/// This works like the derive macro for [`Injectable`](macro@Injectable), except that `bind` and
/// `start` are not supported, as they require an injectable, and that hooks receive the context of a
/// field, like [`FieldPrepareContext`].
///
/// The generated state has no `ctx` field. A struct without any fields has nothing to hold, so no
/// state struct is generated for it, and its hooks receive `&mut ()` as the state.
pub use dijavu_macros::InjectableField;

/// Derives [`Visitable`] by forwarding the visitor to every field.
///
/// Fields can be excluded via `#[visitable(skip)]`.
pub use dijavu_macros::Visitable;
