//! Injectables receive a context typed with themselves, and their fields an untyped one.

use dijavu::{
    AssembleContext, Builder, DeclareContext, Error, FieldAssembleContext, FieldDeclareContext,
    FieldPrepareContext, Injectable, InjectableField, PrepareContext,
    fields::{Depend, InjectToken, Input, Value},
    state::{Declare, Prepare, Preparing},
};
use futures::executor::block_on;

struct Port(u16);

/// Every hook receives a context that provides a token for the injectable itself, and forwards to
/// the field context.
#[derive(Injectable)]
#[injectable(declare = Self::on_declare, prepare = Self::on_prepare, assemble = Self::on_assemble)]
struct Hooks {
    ports: Value<Vec<u16>>,
}

impl Hooks {
    fn on_declare(
        state: &mut HooksState<Declare>,
        ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        let _: InjectToken<Self> = ctx.inject_token();
        state.ports.push(ctx.input::<Port>()?.0);
        Ok(())
    }

    async fn on_prepare(
        state: &mut HooksState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        let _: InjectToken<Self> = ctx.inject_token();
        state.ports.push(ctx.input::<Port>()?.0 + 1);
        Ok(())
    }

    async fn on_assemble(
        state: &mut HooksState<Prepare>,
        ctx: AssembleContext<'_, Self>,
    ) -> Result<(), Error> {
        let _: InjectToken<Self> = ctx.inject_token();
        state.ports.push(ctx.input::<Port>()?.0 + 2);
        Ok(())
    }
}

#[test]
fn hooks_receive_the_context_of_the_injectable() {
    let mut builder = Builder::new();
    builder.set_input(Port(80)).add::<Hooks>();
    let injector = block_on(builder.build()).unwrap();
    assert_eq!(*injector.get::<Hooks>().ports, [80, 81, 82]);
}

/// The hooks of a reusable field cannot know the injectable, so they receive a field context.
#[derive(InjectableField)]
#[injectable(declare = Self::on_declare, prepare = Self::on_prepare, assemble = Self::on_assemble)]
struct Group {
    port: Input<Port>,
    ports: Value<Vec<u16>>,
}

impl Group {
    fn on_declare(
        state: &mut GroupState<Declare>,
        ctx: FieldDeclareContext<'_>,
    ) -> Result<(), Error> {
        state.ports.push(ctx.input::<Port>()?.0);
        Ok(())
    }

    async fn on_prepare(
        state: &mut GroupState<Declare>,
        ctx: FieldPrepareContext<'_>,
    ) -> Result<(), Error> {
        state.ports.push(ctx.input::<Port>()?.0 + 1);
        Ok(())
    }

    async fn on_assemble(
        state: &mut GroupState<Prepare>,
        ctx: FieldAssembleContext<'_>,
    ) -> Result<(), Error> {
        state.ports.push(ctx.input::<Port>()?.0 + 2);
        Ok(())
    }
}

#[derive(Injectable)]
struct UsesGroup {
    group: Group,
}

#[test]
fn hooks_of_fields_receive_the_field_context() {
    let mut builder = Builder::new();
    builder.set_input(Port(80)).add::<UsesGroup>();
    let injector = block_on(builder.build()).unwrap();
    let group = &injector.get::<UsesGroup>().group;
    assert_eq!(group.port.0, 80);
    assert_eq!(*group.ports, [80, 81, 82]);
}

/// The state accessed by other injectables holds the context of its own injectable as well.
#[derive(Injectable)]
struct Configurable {
    port: Value<u16>,
}

impl ConfigurableState<Preparing<'_>> {
    fn use_input(&mut self) -> Result<InjectToken<Configurable>, Error> {
        self.port = self.ctx.input::<Port>()?.0;
        Ok(self.ctx.inject_token())
    }
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct Configures {
    configurable: Depend<Configurable>,
}

impl Configures {
    async fn on_prepare(
        state: &mut ConfiguresState<Declare>,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.get(&mut state.configurable).use_input()?;
        Ok(())
    }
}

#[test]
fn preparing_state_holds_the_context_of_the_injectable() {
    let mut builder = Builder::new();
    builder.set_input(Port(80)).add::<Configures>();
    let injector = block_on(builder.build()).unwrap();
    assert_eq!(*injector.get::<Configurable>().port, 80);
}
