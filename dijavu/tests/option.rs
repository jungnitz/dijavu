use dijavu::{
    Builder, DeclareContext, Error, Injectable, InjectableField, Injector,
    fields::{Depend, Input, Value},
    state::Declare,
};
use futures::executor::block_on;

fn build<I: Injectable>(builder: &mut Builder) -> Injector {
    builder.add::<I>();
    block_on(std::mem::take(builder).build()).unwrap()
}

#[derive(Injectable)]
struct Dependency {
    value: Value<u32>,
}

#[derive(Injectable)]
struct NotAdded {
    dependency: Option<Depend<Dependency>>,
}

#[test]
fn absent_by_default() {
    let injector = build::<NotAdded>(&mut Builder::new());
    assert!(injector.get::<NotAdded>().dependency.is_none());
    // the field has not been declared, so it did not add its dependency
    assert!(injector.get_optional::<Dependency>().is_none());
}

#[derive(Injectable)]
#[injectable(declare = Self::add)]
struct Added {
    dependency: Option<Depend<Dependency>>,
}

impl Added {
    fn add(
        state: &mut AddedState<Declare>,
        mut ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.dependency = Some(Depend::declare(ctx.field())?);
        Ok(())
    }
}

#[test]
fn added_in_declare_hook() {
    let injector = build::<Added>(&mut Builder::new());
    let dependency = injector.get::<Added>().dependency.as_ref().unwrap();
    assert!(std::ptr::eq(&**dependency, injector.get::<Dependency>()));
    assert_eq!(*dependency.value, 0);
}

struct Name(&'static str);

#[derive(Injectable)]
#[injectable(declare = Self::add)]
struct OptionalInput {
    name: Option<Input<Name>>,
}

impl OptionalInput {
    fn add(
        state: &mut OptionalInputState<Declare>,
        mut ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.name = Input::declare(ctx.field()).ok();
        Ok(())
    }
}

#[test]
fn optional_input() {
    let injector = build::<OptionalInput>(&mut Builder::new());
    assert!(injector.get::<OptionalInput>().name.is_none());

    let mut builder = Builder::new();
    builder.set_input(Name("set"));
    let injector = build::<OptionalInput>(&mut builder);
    assert_eq!(
        injector.get::<OptionalInput>().name.as_ref().unwrap().0,
        "set"
    );
}
