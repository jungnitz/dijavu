//! `Injectable` and `InjectableField` must be implementable outside of `dijavu`.

use dijavu::{
    AssembleContext, Builder, DeclareContext, Error, FieldAssembleContext, FieldDeclareContext,
    FieldPrepareContext, Injectable, InjectableField, PrepareContext, Preparing, Visitable,
    fields::{Depend, Inject},
};
use futures::executor::block_on;

#[derive(Visitable)]
struct Multiple {
    this: Inject<Multiple>,
    other: Depend<()>,
}

type This = Inject<Multiple>;
type Other = Depend<()>;

struct MultiplePreparation {
    this: <This as InjectableField>::Preparation,
    other: <Other as InjectableField>::Preparation,
}

struct MultiplePreparing<'ctx> {
    ctx: PrepareContext<'ctx, Multiple>,
    preparation: MultiplePreparation,
}

impl<'ctx> Preparing<'ctx, Multiple> for MultiplePreparing<'ctx> {
    fn make(preparation: MultiplePreparation, ctx: PrepareContext<'ctx, Multiple>) -> Self {
        MultiplePreparing { ctx, preparation }
    }

    fn into_parts(self) -> (PrepareContext<'ctx, Multiple>, MultiplePreparation) {
        (self.ctx, self.preparation)
    }
}

impl Injectable for Multiple {
    type Declaration = (
        <This as InjectableField>::Declaration,
        <Other as InjectableField>::Declaration,
    );
    type Preparation = MultiplePreparation;
    type Preparing<'ctx> = MultiplePreparing<'ctx>;
    type Start = (
        <This as InjectableField>::Start,
        <Other as InjectableField>::Start,
    );

    fn declare(mut ctx: DeclareContext<'_, Self>) -> Result<Self::Declaration, Error> {
        Ok((
            <This as InjectableField>::declare(ctx.field())?,
            <Other as InjectableField>::declare(ctx.field())?,
        ))
    }

    async fn prepare(
        (this, other): Self::Declaration,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<Self::Preparation, Error> {
        Ok(MultiplePreparation {
            this: <This as InjectableField>::prepare(this, ctx.field()).await?,
            other: <Other as InjectableField>::prepare(other, ctx.field()).await?,
        })
    }

    async fn assemble(
        preparation: Self::Preparation,
        mut ctx: AssembleContext<'_, Self>,
    ) -> Result<(Self, Self::Start), Error> {
        let (this, this_start) =
            <This as InjectableField>::assemble(preparation.this, ctx.field()).await?;
        let (other, other_start) =
            <Other as InjectableField>::assemble(preparation.other, ctx.field()).await?;
        Ok((Self { this, other }, (this_start, other_start)))
    }
}

#[test]
fn multiple_fields_outside_of_crate() {
    let mut builder = Builder::new();
    builder.add::<Multiple>();
    let injector = block_on(builder.build()).unwrap();
    let multiple = injector.get::<Multiple>();
    assert!(std::ptr::eq(&*multiple.this, multiple));
    assert!(std::ptr::eq(&*multiple.other, injector.get::<()>()));
}

/// Counts the build steps it went through.
#[derive(Visitable)]
struct Steps(#[visitable(skip)] u32);

impl InjectableField for Steps {
    type Declaration = u32;
    type Preparation = u32;
    type Start = ();

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(1)
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration + 1)
    }

    async fn assemble(
        preparation: Self::Preparation,
        _ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        Ok((Self(preparation + 1), ()))
    }
}

#[derive(Injectable)]
struct UsesSteps {
    steps: Steps,
}

#[test]
fn field_outside_of_crate() {
    let mut builder = Builder::new();
    builder.add::<UsesSteps>();
    let injector = block_on(builder.build()).unwrap();
    assert_eq!(injector.get::<UsesSteps>().steps.0, 3);
}
