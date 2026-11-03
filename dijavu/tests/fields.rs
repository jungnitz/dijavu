//! `Fields` must work outside of `dijavu`.

use dijavu::{
    Builder, DeclareContext, Error, FieldAssembleContext, FieldPrepareContext, Injectable,
    InjectableField, Injector, Visitable,
    fields::{Fields, Inject, Value},
    state::{Declare, Prepare},
};
use futures::executor::block_on;

trait Named: Visitable + Send + Sync {
    fn name(&self) -> String;
}
dijavu::unsizable!(Named);

fn build<I: Injectable>() -> Result<Injector, Error> {
    let mut builder = Builder::new();
    builder.add::<I>();
    block_on(builder.build())
}

fn names(fields: &Fields<dyn Named>) -> Vec<String> {
    fields.iter().map(|field| field.name()).collect()
}

#[derive(InjectableField)]
#[injectable(prepare = Self::configure)]
struct Prepared {
    name: Value<String>,
}

impl Prepared {
    async fn configure(
        state: &mut PreparedState<Declare>,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<(), std::convert::Infallible> {
        state.name = "prepared".to_owned();
        Ok(())
    }
}

impl Named for Prepared {
    fn name(&self) -> String {
        self.name.to_string()
    }
}

#[derive(InjectableField)]
#[injectable(assemble = Self::configure)]
struct Assembled(Value<u32>);

impl Assembled {
    async fn configure(
        state: &mut AssembledState<Prepare>,
        _ctx: FieldAssembleContext<'_>,
    ) -> Result<(), std::convert::Infallible> {
        state.0 = 2;
        Ok(())
    }
}

impl Named for Assembled {
    fn name(&self) -> String {
        format!("assembled {}", *self.0)
    }
}

#[derive(Injectable)]
struct Target;

#[derive(InjectableField)]
struct References(Inject<Target>);

impl Named for References {
    fn name(&self) -> String {
        // panics if the reference has not been enabled
        let _: &Target = &self.0;
        "references".to_owned()
    }
}

#[derive(Injectable)]
#[injectable(declare = Self::add_fields)]
struct Holder {
    fields: Fields<dyn Named>,
}

impl Holder {
    fn add_fields(
        state: &mut HolderState<Declare>,
        mut ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        state
            .fields
            .add::<Prepared>(Prepared::declare(ctx.field())?);
        state
            .fields
            .add::<References>(References::declare(ctx.field())?);
        state
            .fields
            .add::<Assembled>(Assembled::declare(ctx.field())?);
        Ok(())
    }
}

#[test]
fn added_fields_are_built_in_order() {
    let injector = build::<Holder>().unwrap();
    let fields = &injector.get::<Holder>().fields;
    assert_eq!(names(fields), ["prepared", "references", "assembled 2"]);
}

#[test]
fn declarations_of_added_fields_take_effect() {
    // `Target` is only referenced by an added field
    let injector = build::<Holder>().unwrap();
    assert!(injector.get_optional::<Target>().is_some());
}

#[test]
fn accessors() {
    let injector = build::<Holder>().unwrap();
    let fields = &injector.get::<Holder>().fields;
    assert_eq!(fields.len(), 3);
    assert!(!fields.is_empty());
    assert_eq!(
        fields.get(1).map(|field| field.name()).as_deref(),
        Some("references")
    );
    assert!(fields.get(3).is_none());
    let reversed: Vec<_> = fields.iter().rev().map(|field| field.name()).collect();
    assert_eq!(reversed, ["assembled 2", "references", "prepared"]);
    let by_ref: Vec<_> = fields.into_iter().map(|field| field.name()).collect();
    assert_eq!(by_ref, names(fields));
    assert_eq!(fields.iter().len(), 3);
}

#[derive(Injectable)]
struct Empty {
    fields: Fields<dyn Named>,
}

#[test]
fn no_fields_added() {
    let injector = build::<Empty>().unwrap();
    assert!(injector.get::<Empty>().fields.is_empty());
}

#[derive(Debug, thiserror::Error)]
#[error("prepare failed")]
struct PrepareError;

#[derive(InjectableField)]
#[injectable(prepare = Self::fail)]
struct Failing;

impl Failing {
    async fn fail(_state: &mut (), _ctx: FieldPrepareContext<'_>) -> Result<(), Error> {
        Err(Error::custom(Box::new(PrepareError)))
    }
}

impl Named for Failing {
    fn name(&self) -> String {
        "failing".to_owned()
    }
}

#[derive(Injectable)]
#[injectable(declare = Self::add_fields)]
struct WithFailing {
    fields: Fields<dyn Named>,
}

impl WithFailing {
    fn add_fields(
        state: &mut WithFailingState<Declare>,
        mut ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.fields.add::<Failing>(Failing::declare(ctx.field())?);
        Ok(())
    }
}

#[test]
fn errors_of_added_fields_fail_the_build() {
    let Err(error) = build::<WithFailing>() else {
        panic!("build should fail")
    };
    assert!(error.into_custom().unwrap().is::<PrepareError>());
}
