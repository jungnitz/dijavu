//! Values set on the builder can be read in every build step.

use std::sync::atomic::{AtomicBool, Ordering};

use dijavu::{
    AssembleContext, Builder, DeclareContext, Error, Injectable, InjectableField, Injector,
    PrepareContext,
    error::ErrorKind,
    fields::{Input, Value},
    state::{Declare, Prepare},
};
use futures::executor::block_on;

fn build(add: impl FnOnce(&mut Builder)) -> Result<Injector, Error> {
    let mut builder = Builder::new();
    add(&mut builder);
    block_on(builder.build())
}

#[derive(Debug, PartialEq)]
struct Port(u16);

#[derive(Injectable)]
#[injectable(declare = Self::on_declare, prepare = Self::on_prepare)]
struct UsesField {
    port: Input<Port>,
    from_declare: Value<u16>,
    from_prepare: Value<u16>,
}

impl UsesField {
    fn on_declare(
        state: &mut UsesFieldState<Declare>,
        _ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.from_declare = state.port.0;
        Ok(())
    }

    async fn on_prepare(
        state: &mut UsesFieldState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.from_prepare = state.port.0 + 1;
        Ok(())
    }
}

#[test]
fn field_is_available_in_the_state_and_afterwards() {
    let injector = build(|builder| {
        builder.set_input(Port(80)).add::<UsesField>();
    })
    .unwrap();
    let uses = injector.get::<UsesField>();
    assert_eq!(*uses.port, Port(80));
    assert_eq!(*uses.from_declare, 80);
    assert_eq!(*uses.from_prepare, 81);
}

#[derive(Injectable)]
struct AlsoUsesField {
    port: Input<Port>,
}

#[test]
fn all_fields_share_the_value() {
    let injector = build(|builder| {
        builder
            .set_input(Port(80))
            .add::<UsesField>()
            .add::<AlsoUsesField>();
    })
    .unwrap();
    assert!(std::ptr::eq(
        &*injector.get::<UsesField>().port,
        &*injector.get::<AlsoUsesField>().port,
    ));
}

#[test]
fn setting_an_input_again_replaces_it() {
    let injector = build(|builder| {
        builder
            .set_input(Port(80))
            .set_input(Port(443))
            .add::<AlsoUsesField>();
    })
    .unwrap();
    assert_eq!(*injector.get::<AlsoUsesField>().port, Port(443));
}

static PREPARED: AtomicBool = AtomicBool::new(false);

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct RecordsPrepare;

impl RecordsPrepare {
    async fn on_prepare(
        _state: &mut RecordsPrepareState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        PREPARED.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn missing_input_fails_before_anything_is_prepared() {
    let error = build(|builder| {
        builder.add::<RecordsPrepare>().add::<AlsoUsesField>();
    })
    .err()
    .unwrap();
    assert!(matches!(error.kind(), ErrorKind::MissingInput(_)));
    assert!(!PREPARED.load(Ordering::SeqCst));
}

#[derive(Injectable)]
#[injectable(declare = Self::on_declare, prepare = Self::on_prepare, assemble = Self::on_assemble)]
struct UsesContexts {
    from_declare: Value<u16>,
    from_prepare: Value<u16>,
    from_assemble: Value<u16>,
}

impl UsesContexts {
    fn on_declare(
        state: &mut UsesContextsState<Declare>,
        ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.from_declare = ctx.input::<Port>()?.0;
        Ok(())
    }

    async fn on_prepare(
        state: &mut UsesContextsState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.from_prepare = ctx.input::<Port>()?.0 + 1;
        Ok(())
    }

    async fn on_assemble(
        state: &mut UsesContextsState<Prepare>,
        ctx: AssembleContext<'_, Self>,
    ) -> Result<(), Error> {
        state.from_assemble = ctx.input::<Port>()?.0 + 2;
        Ok(())
    }
}

#[test]
fn contexts_return_the_input() {
    let injector = build(|builder| {
        builder.set_input(Port(80)).add::<UsesContexts>();
    })
    .unwrap();
    let uses = injector.get::<UsesContexts>();
    assert_eq!(*uses.from_declare, 80);
    assert_eq!(*uses.from_prepare, 81);
    assert_eq!(*uses.from_assemble, 82);
}

#[test]
fn contexts_fail_on_missing_input() {
    let error = build(|builder| {
        builder.add::<UsesContexts>();
    })
    .err()
    .unwrap();
    assert!(matches!(error.kind(), ErrorKind::MissingInput(_)));
}

#[derive(Injectable)]
#[injectable(declare = Self::on_declare, prepare = Self::on_prepare, assemble = Self::on_assemble)]
struct OptionalInput {
    missing: Value<[bool; 3]>,
}

impl OptionalInput {
    fn on_declare(
        state: &mut OptionalInputState<Declare>,
        ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.missing[0] = ctx.input::<Port>().is_err();
        Ok(())
    }

    async fn on_prepare(
        state: &mut OptionalInputState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.missing[1] = ctx.input::<Port>().is_err();
        Ok(())
    }

    async fn on_assemble(
        state: &mut OptionalInputState<Prepare>,
        ctx: AssembleContext<'_, Self>,
    ) -> Result<(), Error> {
        state.missing[2] = ctx.input::<Port>().is_err();
        Ok(())
    }
}

#[test]
fn hooks_can_handle_a_missing_input() {
    let injector = build(|builder| {
        builder.add::<OptionalInput>();
    })
    .unwrap();
    assert_eq!(*injector.get::<OptionalInput>().missing, [true; 3]);
}

/// A reusable group of fields, which works the same as a field of an injectable.
#[derive(InjectableField)]
struct Group {
    port: Input<Port>,
}

#[derive(Injectable)]
struct UsesGroup {
    group: Group,
}

#[test]
fn input_in_a_reusable_field() {
    let injector = build(|builder| {
        builder.set_input(Port(80)).add::<UsesGroup>();
    })
    .unwrap();
    assert_eq!(*injector.get::<UsesGroup>().group.port, Port(80));
}
