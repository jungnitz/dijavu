use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    Registered, RegisteredValues, Visitable,
};

impl<R: Registered> InjectableField for RegisteredValues<R> {
    type Declaration = ();
    type Preparation = Self;
    type Start = Self;

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(())
    }

    async fn prepare(
        _declaration: Self::Declaration,
        ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(ctx.registered::<R>())
    }

    async fn assemble(
        preparation: Self::Preparation,
        _ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        Ok((preparation.clone(), preparation))
    }
}

// SAFETY: visits nothing
unsafe impl<R> Visitable for RegisteredValues<R> {
    fn visit_refs(&mut self, _visitor: &mut crate::RefVisitor<'_>) {}
}
