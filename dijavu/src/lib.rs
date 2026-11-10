#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]

mod build;
pub mod error;
pub mod fields;
mod hook;
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
pub use hook::HookResult;
pub use injectable::{Injectable, Preparing};
pub use injector::{Injector, InjectorHandle};
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
/// The following attributes can be used as `#[injectable(...)]` on the struct.
///
/// - `declare = hook`: `fn(&mut FooState<Declare>, DeclareContext<'_, Self>)`, runs after the
///   fields have been declared
/// - `bind(dyn Trait, ...)`: binds the given targets to this injectable
/// - `provide(dyn Trait, ...)`: provides the given types, so that every `InjectAll` of them
///   references this injectable, see [`FieldDeclareContext::provide`]
/// - `register(expr, ...)`: registers the value of each expression after the `declare` hook, see
///   [`FieldDeclareContext::register`]. The type of the expression decides what is registered, so trait
///   objects need a cast, like `Box::new(MyPlugin) as Box<dyn Plugin>`.
/// - `prepare = hook`: `async fn(&mut FooState<Declare>, PrepareContext<'_, Self>)`, runs before the
///   fields are prepared
/// - `prepared = hook`: `async fn(&mut FooState<Preparing<'_>>)`, runs after the fields have been
///   prepared, with the context in the `ctx` field
/// - `assemble = hook`: `async fn(&mut FooState<Prepare>, AssembleContext<'_, Self>)`, runs before
///   the fields are assembled
/// - `start = hook`: `async fn(&Foo, FooState<Start>, StartContext<'_, Self>)`, implements
///   [`Injectable::start`]
/// - `crate = path`: the path to this crate, `dijavu` by default, e.g. when it is re-exported by
///   another crate
///
/// Hooks return a [`HookResult`]: `()` if they cannot fail, or `Result<(), E>` with
/// `E: Into<Error>`. Apart from `declare`, hooks are async by default. Use `prepare_sync`,
/// `prepared_sync`, `assemble_sync` or `start_sync` instead to give a sync hook with the same
/// parameters.
///
/// A hook is any expression that can be called with its parameters, e.g. a path or a closure. A
/// bare name like `prepared = fill` refers to a method of the state, i.e. `FooState::fill`, except
/// for `start`, where it refers to `Foo::fill`.
///
/// Fields can be excluded from visiting via `#[visitable(skip)]`.
///
/// `#[allow(...)]` attributes on the struct are copied onto `FooState`, e.g. to allow
/// `non_camel_case_types` for both.
///
/// # Generics
///
/// Only the bounds written on the struct are used, like `T: Target` for an `Inject<T>` field or
/// `T: InjectableField` for a field of type `T`.
pub use dijavu_macros::Injectable;

/// Derives [`InjectableField`] and [`Visitable`] for a struct whose fields all implement
/// [`InjectableField`].
///
/// This works like the derive macro for [`Injectable`](macro@Injectable), except that `bind`,
/// `provide` and `start` are not supported, as they require an injectable, and that hooks receive
/// the context of a field, like [`FieldPrepareContext`]. As there is no `FooState<Preparing<'_>>`
/// for a field, the `prepared` hook is an
/// `async fn(&mut FooState<Prepare>, FieldPrepareContext<'_>)`.
///
/// The generated state has no `ctx` field. A struct without any fields has nothing to hold, so no
/// state struct is generated for it, and its hooks receive `&mut ()` as the state. Bare hook names
/// cannot be used then.
pub use dijavu_macros::InjectableField;

/// Derives [`Visitable`] by forwarding the visitor to every field.
///
/// Fields can be excluded via `#[visitable(skip)]`. The path to this crate can be set via
/// `#[visitable(crate = path)]` on the type, it is `dijavu` by default.
pub use dijavu_macros::Visitable;
