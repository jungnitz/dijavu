//! Values registered while declaring can be read while preparing and assembling.

use std::marker::PhantomData;

use dijavu::{
    AssembleContext, Builder, DeclareContext, Error, FieldDeclareContext, Injectable,
    InjectableField, Injector, PrepareContext, Registered, RegisteredValues,
    fields::{Depend, Value},
    state::{Declare, Prepare, Preparing},
};
use futures::executor::block_on;

fn build(add: impl FnOnce(&mut Builder)) -> Injector {
    let mut builder = Builder::new();
    add(&mut builder);
    block_on(builder.build()).unwrap()
}

trait Plugin: Send + Sync {
    fn id(&self) -> u32;
}

struct Fixed(u32);

impl Plugin for Fixed {
    fn id(&self) -> u32 {
        self.0
    }
}

struct Doubled(u32);

impl Plugin for Doubled {
    fn id(&self) -> u32 {
        self.0 * 2
    }
}

impl Registered for Box<dyn Plugin> {
    type Consumer = ();
}

struct Name(&'static str);

impl Registered for Name {
    type Consumer = ();
}

struct Number(u32);

impl Registered for Number {
    type Consumer = ();
}

fn ids(values: RegisteredValues<Box<dyn Plugin>>) -> Vec<u32> {
    values.iter().map(|value| value.id()).collect()
}

#[derive(Injectable)]
#[injectable(declare = Self::register)]
struct Registrar;

impl Registrar {
    fn register(
        _state: &mut RegistrarState<Declare>,
        mut ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.register::<Box<dyn Plugin>>(Box::new(Fixed(1)));
        ctx.register::<Box<dyn Plugin>>(Box::new(Doubled(1)));
        ctx.register(Name("other type"));
        Ok(())
    }
}

/// Does not reference `Registrar`.
#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare, assemble = Self::on_assemble)]
struct Reader {
    prepared: Value<Vec<u32>>,
    assembled: Value<Vec<u32>>,
}

impl Reader {
    async fn on_prepare(
        state: &mut ReaderState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.prepared = ids(ctx.registered());
        Ok(())
    }

    async fn on_assemble(
        state: &mut ReaderState<Prepare>,
        ctx: AssembleContext<'_, Self>,
    ) -> Result<(), Error> {
        state.assembled = ids(ctx.registered());
        Ok(())
    }
}

#[test]
fn registered_values_can_be_read_while_preparing_and_assembling() {
    let injector = build(|builder| {
        builder.add::<Registrar>().add::<Reader>();
    });
    let reader = injector.get::<Reader>();
    assert_eq!(*reader.prepared, [1, 2]);
    assert_eq!(*reader.assembled, [1, 2]);
}

#[test]
fn reader_declared_after_registrar_sees_the_same_values() {
    let injector = build(|builder| {
        builder.add::<Reader>().add::<Registrar>();
    });
    let reader = injector.get::<Reader>();
    assert_eq!(*reader.prepared, [1, 2]);
    assert_eq!(*reader.assembled, [1, 2]);
}

#[test]
fn nothing_registered_reads_as_empty() {
    let injector = build(|builder| {
        builder.add::<Reader>();
    });
    assert!(injector.get::<Reader>().prepared.is_empty());
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct TypeReader {
    strs: Value<Vec<&'static str>>,
    numbers: Value<usize>,
}

impl TypeReader {
    async fn on_prepare(
        state: &mut TypeReaderState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.strs = ctx.registered::<Name>().iter().map(|name| name.0).collect();
        state.numbers = ctx.registered::<Number>().len();
        Ok(())
    }
}

#[test]
fn values_are_grouped_by_type() {
    let injector = build(|builder| {
        builder.add::<Registrar>().add::<TypeReader>();
    });
    let reader = injector.get::<TypeReader>();
    assert_eq!(*reader.strs, ["other type"]);
    assert_eq!(*reader.numbers, 0);
}

/// A field registering a value for every injectable containing it.
#[derive(InjectableField)]
#[injectable(declare = Self::on_declare)]
struct RegistersSeven;

impl RegistersSeven {
    fn on_declare(_state: &mut (), mut ctx: FieldDeclareContext<'_>) -> Result<(), Error> {
        ctx.register(Number(7));
        Ok(())
    }
}

#[derive(Injectable)]
struct FirstWithField(RegistersSeven);

#[derive(Injectable)]
struct SecondWithField(RegistersSeven);

/// Keeps the list after the build, and configures a dependency while iterating over it.
#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct Keeper {
    kept: Value<Option<RegisteredValues<Number>>>,
    sum: Depend<Sum>,
}

impl Keeper {
    async fn on_prepare(
        state: &mut KeeperState<Declare>,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        let values = ctx.registered::<Number>();
        // the list must be usable across await points
        futures::future::ready(()).await;
        for value in &values {
            ctx.get(&mut state.sum).add(value.0);
        }
        ctx.get(&mut state.sum).add_registered_again();
        state.kept = Some(values);
        Ok(())
    }
}

#[derive(Injectable)]
struct Sum(Value<u32>);

impl SumState<Preparing<'_>> {
    fn add(&mut self, value: u32) {
        self.0 += value;
    }

    fn add_registered_again(&mut self) {
        self.0 += self
            .1
            .registered::<Number>()
            .iter()
            .map(|number| number.0)
            .sum::<u32>();
    }
}

#[test]
fn registered_values_from_fields_are_kept_and_configure_dependencies() {
    let injector = build(|builder| {
        builder
            .add::<FirstWithField>()
            .add::<SecondWithField>()
            .add::<Keeper>();
    });
    let keeper = injector.get::<Keeper>();
    assert_eq!(keeper.kept.as_ref().unwrap().len(), 2);
    assert_eq!(*keeper.sum.0, 28);
}

#[derive(Injectable)]
#[injectable(
    declare = Self::on_declare,
    register(Box::new(Fixed(3)) as Box<dyn Plugin>, Number(Self::NUMBER)),
    register(Name("attribute"))
)]
struct RegistersViaAttribute;

impl RegistersViaAttribute {
    const NUMBER: u32 = 5;

    fn on_declare(
        _state: &mut RegistersViaAttributeState<Declare>,
        mut ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.register(Name("hook"));
        Ok(())
    }
}

#[test]
fn register_attribute() {
    let injector = build(|builder| {
        builder
            .add::<RegistersViaAttribute>()
            .add::<Reader>()
            .add::<TypeReader>();
    });
    assert_eq!(*injector.get::<Reader>().prepared, [3]);
    let type_reader = injector.get::<TypeReader>();
    assert_eq!(*type_reader.strs, ["hook", "attribute"]);
    assert_eq!(*type_reader.numbers, 1);
}

#[derive(InjectableField)]
#[injectable(register(Number(9)))]
struct RegistersNineViaAttribute;

#[derive(Injectable)]
struct WithAttributeField(RegistersNineViaAttribute);

#[test]
fn register_attribute_on_field() {
    let injector = build(|builder| {
        builder.add::<WithAttributeField>().add::<TypeReader>();
    });
    assert_eq!(*injector.get::<TypeReader>().numbers, 1);
}

struct Consumed(u32);

impl Registered for Consumed {
    type Consumer = ConsumedSum;
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct ConsumedSum {
    sum: Value<u32>,
}

impl ConsumedSum {
    async fn on_prepare(
        state: &mut ConsumedSumState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.sum = ctx
            .registered::<Consumed>()
            .iter()
            .map(|value| value.0)
            .sum();
        Ok(())
    }
}

#[derive(Injectable)]
#[injectable(register(Consumed(1), Consumed(2)))]
struct RegistersConsumed;

#[test]
fn registering_adds_the_consumer() {
    let injector = build(|builder| {
        builder.add::<RegistersConsumed>();
    });
    assert_eq!(*injector.get::<ConsumedSum>().sum, 3);
}

#[test]
fn consumer_is_not_added_without_a_registered_value() {
    let injector = build(|builder| {
        builder.add::<Reader>();
    });
    assert!(injector.get_optional::<ConsumedSum>().is_none());
}

trait Amount: Copy + Into<u64> + Send + Sync + 'static {}

impl Amount for u8 {}
impl Amount for u32 {}

/// Every `T` has its own list of values, whose consumer forwards them to a shared injectable.
struct Typed<T>(T);

impl<T: Amount> Registered for Typed<T> {
    type Consumer = Forwarder<T>;
}

#[derive(Injectable)]
#[injectable(prepare = Self::forward)]
struct Forwarder<T: Amount> {
    total: Depend<Total>,
    _t: Value<PhantomData<fn() -> T>>,
}

impl<T: Amount> Forwarder<T> {
    async fn forward(
        state: &mut ForwarderState<T, Declare>,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        for value in &ctx.registered::<Typed<T>>() {
            ctx.get(&mut state.total).add(value.0.into());
        }
        Ok(())
    }
}

#[derive(Injectable)]
struct Total(Value<u64>);

impl TotalState<Preparing<'_>> {
    fn add(&mut self, amount: u64) {
        self.0 += amount;
    }
}

#[derive(Injectable)]
#[injectable(register(Typed(1u8), Typed(2u32), Typed(3u32)))]
struct RegistersTyped;

#[test]
fn generic_consumers_can_forward_to_a_shared_injectable() {
    let injector = build(|builder| {
        builder.add::<RegistersTyped>();
    });
    assert_eq!(*injector.get::<Total>().0, 6);
}
