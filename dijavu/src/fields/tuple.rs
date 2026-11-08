use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    RefVisitor, Visitable,
};

impl InjectableField for () {
    type Declaration = ();
    type Preparation = ();
    type Start = ();

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(())
    }

    async fn prepare(
        _declaration: Self::Declaration,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(())
    }

    async fn assemble(
        _preparation: Self::Preparation,
        _ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        Ok(((), ()))
    }
}

macro_rules! impl_tuple {
    ($($F:ident $v:ident),+) => {
        #[diagnostic::do_not_recommend]
        impl<$($F: InjectableField),+> InjectableField for ($($F,)+) {
            type Declaration = ($($F::Declaration,)+);
            type Preparation = ($($F::Preparation,)+);
            type Start = ($($F::Start,)+);

            fn declare(mut ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
                Ok(($($F::declare(ctx.reborrow())?,)+))
            }

            async fn prepare(
                declaration: Self::Declaration,
                mut ctx: FieldPrepareContext<'_>,
            ) -> Result<Self::Preparation, Error> {
                let ($($v,)+) = declaration;
                Ok(($($F::prepare($v, ctx.reborrow()).await?,)+))
            }

            async fn assemble(
                preparation: Self::Preparation,
                mut ctx: FieldAssembleContext<'_>,
            ) -> Result<(Self, Self::Start), Error> {
                let ($($v,)+) = preparation;
                $(let $v = $F::assemble($v, ctx.reborrow()).await?;)+
                Ok((($($v.0,)+), ($($v.1,)+)))
            }
        }

        // SAFETY: the elements can only be moved out of the tuple through `&mut self`, and each
        // of them is `Visitable` itself
        unsafe impl<$($F: Visitable),+> Visitable for ($($F,)+) {
            fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>) {
                let ($($v,)+) = self;
                $($v.visit_refs(visitor);)+
            }
        }
    };
}

impl_tuple!(F0 f0);
impl_tuple!(F0 f0, F1 f1);
impl_tuple!(F0 f0, F1 f1, F2 f2);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3, F4 f4);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7, F8 f8);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7, F8 f8, F9 f9);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7, F8 f8, F9 f9, F10 f10);
impl_tuple!(F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7, F8 f8, F9 f9, F10 f10, F11 f11);
impl_tuple!(
    F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7, F8 f8, F9 f9, F10 f10, F11 f11, F12 f12
);
impl_tuple!(
    F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7, F8 f8, F9 f9, F10 f10, F11 f11, F12 f12,
    F13 f13
);
impl_tuple!(
    F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7, F8 f8, F9 f9, F10 f10, F11 f11, F12 f12,
    F13 f13, F14 f14
);
impl_tuple!(
    F0 f0, F1 f1, F2 f2, F3 f3, F4 f4, F5 f5, F6 f6, F7 f7, F8 f8, F9 f9, F10 f10, F11 f11, F12 f12,
    F13 f13, F14 f14, F15 f15
);
