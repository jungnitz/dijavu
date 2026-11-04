use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
};

/// An optional field, which is absent unless it is added during the declare step.
///
/// While building, the state of the injectable holds an `Option` of the data of `F`, which starts
/// out as `None`. To add the field, set it to the declaration of `F` in a `declare` hook, typically
/// via `state.field = Some(F::declare(ctx.field())?)`. Declaring `F` there (rather than always
/// declaring it and discarding it later) means that a field which is not added does not declare
/// anything, e.g. no dependency of a [`Depend`](crate::fields::Depend) and no required input of an
/// [`Input`](crate::fields::Input), so `Input::declare(ctx.field()).ok()` makes an input optional.
///
/// Its [`Start`](InjectableField::Start) is `Some` if the field has been added.
impl<F: InjectableField> InjectableField for Option<F> {
    type Declaration = Option<F::Declaration>;
    type Preparation = Option<F::Preparation>;
    type Start = Option<F::Start>;

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(None)
    }

    async fn prepare(
        declaration: Self::Declaration,
        ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        match declaration {
            Some(declaration) => Ok(Some(F::prepare(declaration, ctx).await?)),
            None => Ok(None),
        }
    }

    async fn assemble(
        preparation: Self::Preparation,
        ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        match preparation {
            Some(preparation) => {
                let (field, start) = F::assemble(preparation, ctx).await?;
                Ok((Some(field), Some(start)))
            }
            None => Ok((None, None)),
        }
    }
}
