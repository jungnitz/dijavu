use dijavu::{
    AssembleContext, DeclareContext, Error, Injectable, PrepareContext, Preparing, RefVisitor,
    Visitable,
};

struct Leaky;

fn stash(_ctx: DeclareContext<'static, Leaky>) {}

// SAFETY: does not visit anything
unsafe impl Visitable for Leaky {
    fn visit_refs(&mut self, _visitor: &mut RefVisitor<'_>) {}
}

struct LeakyPreparing<'ctx>(PrepareContext<'ctx, Leaky>);

impl<'ctx> Preparing<'ctx, Leaky> for LeakyPreparing<'ctx> {
    fn into_parts(self) -> (PrepareContext<'ctx, Leaky>, ()) {
        (self.0, ())
    }
}

impl Injectable for Leaky {
    type Declaration = ();
    type Preparation = ();
    type Preparing<'ctx> = LeakyPreparing<'ctx>;
    type Start = ();

    fn declare(ctx: DeclareContext<'_, Self>) -> Result<(), Error> {
        stash(ctx);
        Ok(())
    }

    async fn prepare(_declaration: (), _ctx: PrepareContext<'_, Self>) -> Result<(), Error> {
        Ok(())
    }

    fn make_preparing<'ctx>(
        _preparation: (),
        ctx: PrepareContext<'ctx, Self>,
    ) -> LeakyPreparing<'ctx> {
        LeakyPreparing(ctx)
    }

    async fn assemble(
        _preparation: (),
        _ctx: AssembleContext<'_, Self>,
    ) -> Result<(Self, ()), Error> {
        Ok((Leaky, ()))
    }
}

fn main() {}
