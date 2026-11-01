//! `Injectable` and `InjectableField` must be implementable outside of `dijavu`.

use dijavu::{
    AssembleContext, Builder, DeclareContext, Error, Injectable, InjectableField, PrepareContext,
    Preparing, Visitable,
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
    this: <This as InjectableField<Multiple>>::Preparation,
    other: <Other as InjectableField<Multiple>>::Preparation,
}

struct MultiplePreparing<'ctx> {
    ctx: PrepareContext<'ctx, Multiple>,
    preparation: MultiplePreparation,
}

impl<'ctx> Preparing<'ctx, Multiple> for MultiplePreparing<'ctx> {
    fn into_parts(self) -> (PrepareContext<'ctx, Multiple>, MultiplePreparation) {
        (self.ctx, self.preparation)
    }
}

impl Injectable for Multiple {
    type Declaration = (
        <This as InjectableField<Self>>::Declaration,
        <Other as InjectableField<Self>>::Declaration,
    );
    type Preparation = MultiplePreparation;
    type Preparing<'ctx> = MultiplePreparing<'ctx>;

    fn declare(mut ctx: DeclareContext<'_, Self>) -> Result<Self::Declaration, Error> {
        Ok((
            <This as InjectableField<Self>>::declare(ctx.as_owned())?,
            <Other as InjectableField<Self>>::declare(ctx.as_owned())?,
        ))
    }

    async fn prepare(
        (this, other): Self::Declaration,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<Self::Preparation, Error> {
        Ok(MultiplePreparation {
            this: <This as InjectableField<Self>>::prepare(this, ctx.as_owned()).await?,
            other: <Other as InjectableField<Self>>::prepare(other, ctx.as_owned()).await?,
        })
    }

    fn make_preparing<'ctx>(
        preparation: Self::Preparation,
        ctx: PrepareContext<'ctx, Self>,
    ) -> Self::Preparing<'ctx> {
        MultiplePreparing { ctx, preparation }
    }

    async fn assemble(
        preparation: Self::Preparation,
        mut ctx: AssembleContext<'_, Self>,
    ) -> Result<Self, Error> {
        Ok(Self {
            this: <This as InjectableField<Self>>::assemble(preparation.this, ctx.as_owned())
                .await?,
            other: <Other as InjectableField<Self>>::assemble(preparation.other, ctx.as_owned())
                .await?,
        })
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

impl<I: Injectable> InjectableField<I> for Steps {
    type Declaration = u32;
    type Preparation = u32;

    fn declare(_ctx: DeclareContext<'_, I>) -> Result<Self::Declaration, Error> {
        Ok(1)
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: PrepareContext<'_, I>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration + 1)
    }

    async fn assemble(
        preparation: Self::Preparation,
        _ctx: AssembleContext<'_, I>,
    ) -> Result<Self, Error> {
        Ok(Self(preparation + 1))
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
