use crate::common::args::DeriveArgs;
use darling::FromField;
use proc_macro2::{Ident, Literal, TokenStream};
use quote::{ToTokens, quote};
use std::rc::Rc;
use syn::{Expr, Field};

pub struct InitializableField {
    pub args: Rc<DeriveArgs>,
    pub index: usize,
    pub field: Field,
    pub init: Option<Expr>,
}

impl InitializableField {
    #[expect(clippy::needless_continue, reason = "emitted by FromField")]
    pub fn new(args: Rc<DeriveArgs>, index: usize, field: Field) -> syn::Result<Self> {
        #[derive(FromField)]
        #[darling(attributes(inject))]
        struct FieldAttrs {
            init: Option<Expr>,
        }
        let attrs = FieldAttrs::from_field(&field)?;
        Ok(Self {
            args,
            index,
            field,
            init: attrs.init,
        })
    }

    fn accessor(&self) -> TokenStream {
        let index = self.index;
        self.field.ident.as_ref().map_or_else(
            || Literal::usize_unsuffixed(index).into_token_stream(),
            Ident::to_token_stream,
        )
    }

    fn field_name(&self) -> String {
        self.field
            .ident
            .as_ref()
            .map_or_else(|| format!(".{}", self.index), Ident::to_string)
    }

    fn as_initializable<R>(&self, f: impl FnOnce(std::option::Iter<Ident>, TokenStream) -> R) -> R {
        let ty = &self.field.ty;
        let ty = quote!(<#ty as dijavu::Initializable>);
        f(self.field.ident.iter(), ty)
    }

    pub fn init_field_decl(&self) -> TokenStream {
        self.as_initializable(|ident, ty| quote!(#(#ident:)* #ty::Init))
    }

    pub fn init(&self) -> TokenStream {
        let ty = &self.field.ty;
        let ident = self.field.ident.iter();
        let error_msg = format!(
            "could not create initialization state for field `{}` of `{}`",
            self.field_name(),
            self.args.ident
        );
        let init = if let Some(init) = &self.init {
            quote!(#init)
        } else {
            quote!(<#ty as dijavu::NewInitValue>::new_init)
        };
        quote!(
            #(#ident:)* dijavu::Result::map_err(
                #init(injector).await,
                |err| dijavu::Error::from(err).with_context(#error_msg)
            )?
        )
    }

    pub fn build(&self) -> TokenStream {
        self.as_initializable(|ident, ty| {
            let accessor = self.accessor();
            let error_msg = format!(
                "could not build field `{}` of `{}`",
                self.field_name(),
                self.args.ident
            );
            quote!(
                #(#ident:)* dijavu::Result::map_err(
                    #ty::build(init.#accessor, builder).await,
                    |err| dijavu::Error::from(err).with_context(#error_msg)
                )?
            )
        })
    }
}
