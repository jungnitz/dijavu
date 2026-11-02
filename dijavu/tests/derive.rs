//! The derive macros must work outside of `dijavu`.

use dijavu::{
    AssembleContext, Builder, DeclareContext, Error, FieldAssembleContext, FieldDeclareContext,
    Injectable, InjectableField, Injector, PrepareContext, Target, Visitable,
    fields::{Depend, Inject, Value},
    state::{Declare, Prepare, Preparing},
};
use futures::executor::block_on;

fn build<I: Injectable>() -> Injector {
    let mut builder = Builder::new();
    builder.add::<I>();
    block_on(builder.build()).unwrap()
}

#[derive(Injectable)]
pub struct Config {
    name: Value<String>,
    #[visitable(skip)]
    count: Value<u32>,
}

impl ConfigState<Preparing<'_>> {
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }
}

#[derive(Injectable)]
pub struct First {
    pub second: Inject<Second>,
    config: Depend<Config>,
}

#[derive(Injectable)]
pub struct Second {
    pub first: Inject<First>,
    value: Value<u32>,
}

#[test]
fn named_fields_with_cycle() {
    let injector = build::<First>();
    let first = injector.get::<First>();
    assert!(std::ptr::eq(&*first.second.first, first));
    assert_eq!(*first.second.value, 0);
    assert_eq!(first.config.name.as_str(), "");
    assert_eq!(*first.config.count, 0);
}

#[derive(Injectable)]
#[injectable(prepare = Self::configure)]
struct WithPrepareHook {
    config: Depend<Config>,
    value: Value<u32>,
}

impl WithPrepareHook {
    async fn configure(
        state: &mut WithPrepareHookState<Declare>,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.get(&mut state.config).set_name("from hook");
        state.value = 3;
        Ok(())
    }
}

#[test]
fn prepare_hook_configures_dependency() {
    let injector = build::<WithPrepareHook>();
    let injectable = injector.get::<WithPrepareHook>();
    assert_eq!(injectable.config.name.as_str(), "from hook");
    assert_eq!(*injectable.value, 3);
}

#[derive(Injectable)]
struct Outer {
    config: Depend<Config>,
}

#[derive(Injectable)]
#[injectable(prepare = Self::configure)]
struct OuterConfigurer {
    outer: Depend<Outer>,
}

impl OuterConfigurer {
    async fn configure(
        state: &mut OuterConfigurerState<Declare>,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.get(&mut state.outer).configure_config("through outer");
        Ok(())
    }
}

impl OuterState<Preparing<'_>> {
    fn configure_config(&mut self, name: &str) {
        self.ctx.get(&mut self.config).set_name(name);
    }
}

#[test]
fn preparing_state_reaches_its_dependencies() {
    let injector = build::<OuterConfigurer>();
    let outer = &injector.get::<OuterConfigurer>().outer;
    assert_eq!(outer.config.name.as_str(), "through outer");
}

#[derive(Injectable)]
struct Tuple(Inject<Config>, pub Value<u32>);

#[derive(Injectable)]
struct Unit;

#[derive(Injectable)]
struct EmptyNamed {}

#[derive(Injectable)]
struct EmptyTuple();

impl TupleState<Preparing<'_>> {
    fn _context_is_last(&mut self) {
        let _value: &mut u32 = &mut self.1;
        let _ctx: &mut PrepareContext<'_, Tuple> = &mut self.2;
    }
}

#[test]
fn tuple_and_unit_structs() {
    let injector = build::<Tuple>();
    assert_eq!(*injector.get::<Tuple>().1, 0);
    assert_eq!(injector.get::<Tuple>().0.name.as_str(), "");
    build::<Unit>().get::<Unit>();
    build::<EmptyNamed>().get::<EmptyNamed>();
    build::<EmptyTuple>().get::<EmptyTuple>();
}

#[derive(Default)]
struct Len<const N: usize>;

impl<const N: usize> Len<N> {
    fn get(&self) -> usize {
        N
    }
}

#[derive(Injectable)]
struct Generic<T: Target, const N: usize = 3, U = u32>
where
    U: Default + Send + Sync + 'static,
{
    target: Inject<T>,
    value: Value<U>,
    len: Value<Len<N>>,
}

impl<T: Target, U: Default + Send + Sync + 'static> GenericState<T, 3, U, Preparing<'_>> {
    fn _set(&mut self, value: U) {
        self.value = value;
    }
}

#[test]
fn generics() {
    let injector = build::<Generic<Config>>();
    let generic = injector.get::<Generic<Config>>();
    assert_eq!(generic.target.name.as_str(), "");
    assert_eq!(*generic.value, 0);
    assert_eq!(generic.len.get(), 3);
}

pub trait Service: Send + Sync {}
dijavu::bindable!(Service);

pub trait Other: Send + Sync {}
dijavu::bindable!(Other);

#[derive(Injectable)]
#[injectable(bind(dyn Service, dyn Other))]
struct ServiceImpl;

impl Service for ServiceImpl {}
impl Other for ServiceImpl {}

#[derive(Injectable)]
struct UsesService {
    service: Depend<dyn Service>,
    other: Inject<dyn Other>,
}

#[test]
fn bind_attribute() {
    let mut builder = Builder::new();
    builder.add::<ServiceImpl>().add::<UsesService>();
    let injector = block_on(builder.build()).unwrap();
    let uses = injector.get::<UsesService>();
    let service_impl = injector.get::<ServiceImpl>();
    assert!(std::ptr::addr_eq(&*uses.service, service_impl));
    assert!(std::ptr::addr_eq(&*uses.other, service_impl));
}

#[derive(Injectable)]
#[injectable(declare = Self::on_declare)]
struct WithDeclareHook {
    value: Value<u32>,
}

impl WithDeclareHook {
    fn on_declare(
        state: &mut WithDeclareHookState<Declare>,
        _ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.value = 4;
        Ok(())
    }
}

#[test]
fn declare_hook() {
    let injector = build::<WithDeclareHook>();
    assert_eq!(*injector.get::<WithDeclareHook>().value, 4);
}

#[derive(Injectable)]
#[injectable(assemble = Self::on_assemble)]
struct WithAssembleHook {
    value: Value<u32>,
}

impl WithAssembleHook {
    async fn on_assemble(
        state: &mut WithAssembleHookState<Prepare>,
        _ctx: AssembleContext<'_, Self>,
    ) -> Result<(), std::convert::Infallible> {
        state.value = 5;
        Ok(())
    }
}

#[test]
fn assemble_hook() {
    let injector = build::<WithAssembleHook>();
    assert_eq!(*injector.get::<WithAssembleHook>().value, 5);
}

#[derive(Debug, thiserror::Error)]
#[error("hook failed")]
struct HookError;

impl From<HookError> for Error {
    fn from(error: HookError) -> Self {
        Error::custom(Box::new(error))
    }
}

#[derive(Injectable)]
#[injectable(prepare = Self::fail)]
struct FailingHook;

impl FailingHook {
    async fn fail(
        _state: &mut FailingHookState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), HookError> {
        Err(HookError)
    }
}

#[test]
fn hook_errors_fail_the_build() {
    let mut builder = Builder::new();
    builder.add::<FailingHook>();
    let error = block_on(builder.build()).err().unwrap();
    assert_eq!(error.to_string(), "hook failed");
}

/// A reusable group of fields
#[derive(InjectableField)]
#[injectable(declare = Self::on_declare, assemble = Self::on_assemble)]
pub struct Group {
    config: Inject<Config>,
    declared: Value<u32>,
}

impl Group {
    fn on_declare(
        state: &mut GroupState<Declare>,
        _ctx: FieldDeclareContext<'_>,
    ) -> Result<(), Error> {
        state.declared = 1;
        Ok(())
    }

    async fn on_assemble(
        state: &mut GroupState<Prepare>,
        _ctx: FieldAssembleContext<'_>,
    ) -> Result<(), Error> {
        state.declared += 1;
        Ok(())
    }
}

#[derive(InjectableField)]
pub struct Wrapper<T>(pub T);

#[derive(InjectableField)]
pub struct UnitField;

/// Fields without any fields have no generated state, so this name is free.
pub struct UnitFieldState;

#[derive(InjectableField)]
pub struct EmptyField {}

#[derive(InjectableField)]
pub struct EmptyTupleField();

static UNIT_HOOK_RAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(InjectableField)]
#[injectable(declare = Self::on_declare)]
pub struct UnitWithHook;

impl UnitWithHook {
    fn on_declare(_state: &mut (), _ctx: FieldDeclareContext<'_>) -> Result<(), Error> {
        UNIT_HOOK_RAN.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

/// The state of a field has no context, so unlike for injectables, the name `ctx` is free.
#[derive(InjectableField)]
#[injectable(declare = Self::on_declare)]
pub struct NamedCtx {
    ctx: Value<u32>,
}

impl NamedCtx {
    fn on_declare(
        state: &mut NamedCtxState<Declare>,
        _ctx: FieldDeclareContext<'_>,
    ) -> Result<(), Error> {
        state.ctx = 1;
        Ok(())
    }
}

#[derive(Injectable)]
struct UsesFields {
    group: Group,
    wrapper: Wrapper<Wrapper<Inject<Config>>>,
    unit: UnitField,
    empty: EmptyField,
    empty_tuple: EmptyTupleField,
    unit_with_hook: UnitWithHook,
    named_ctx: NamedCtx,
}

#[test]
fn injectable_field() {
    let injector = build::<UsesFields>();
    let fields = injector.get::<UsesFields>();
    assert_eq!(*fields.group.declared, 2);
    assert!(std::ptr::eq(
        &*fields.group.config,
        injector.get::<Config>()
    ));
    assert!(std::ptr::eq(&*fields.wrapper.0.0, injector.get::<Config>()));
    assert!(UNIT_HOOK_RAN.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(*fields.named_ctx.ctx, 1);
}

#[derive(Injectable)]
struct GenericField<T: InjectableField> {
    field: T,
}

#[test]
fn field_as_a_generic_parameter() {
    let injector = build::<GenericField<Value<u32>>>();
    assert_eq!(*injector.get::<GenericField<Value<u32>>>().field, 0);
}

// The following only need to compile.

fn _assert_visitable<T: Visitable>() {}

fn _derived_types_are_visitable() {
    _assert_visitable::<First>();
    _assert_visitable::<Group>();
    _assert_visitable::<Generic<Config>>();
    _assert_visitable::<Wrapper<Group>>();
}

fn _assert_field<F: InjectableField>() {}

fn _fields_work_for_any_injectable() {
    _assert_field::<Group>();
    _assert_field::<Wrapper<Value<String>>>();
}

#[derive(Injectable)]
struct Private;

/// A private field type in a public injectable must not lead to warnings.
#[derive(Injectable)]
pub struct PublicDerived {
    private: Inject<Private>,
}
