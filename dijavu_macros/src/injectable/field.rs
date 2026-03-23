use crate::injectable::DeriveInjectable;
use darling::FromField;
use darling::util::Flag;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::Field;

#[derive(darling::FromField)]
#[darling(attributes(inject))]
struct FieldAttrs {
    init: Flag,
}

pub struct InjectableField {
    pub index: usize,
    pub field: Field,
    pub source: InjectableFieldSource,
    pub derive: DeriveInjectable,
}

pub enum InjectableFieldSource {
    Injectable,
    InitValue,
}

impl InjectableField {
    pub fn from_field(derive: &DeriveInjectable, index: usize, field: Field) -> syn::Result<Self> {
        let attrs = FieldAttrs::from_field(&field)?;
        if !derive.init && attrs.init.is_present() {
            return Err(syn::Error::new(
                attrs.init.span(),
                "you can only use `init` on a field when deriving `InitInjectable`",
            ));
        }
        Ok(Self {
            index,
            field,
            source: if attrs.init.is_present() {
                InjectableFieldSource::InitValue
            } else {
                InjectableFieldSource::Injectable
            },
            derive: derive.clone(),
        })
    }

    fn accessor(&self) -> TokenStream {
        let index = self.index;
        self.field
            .ident
            .as_ref()
            .map(Ident::to_token_stream)
            .unwrap_or_else(|| proc_macro2::Literal::usize_unsuffixed(index).into_token_stream())
    }

    fn field_name(&self) -> String {
        self.field
            .ident
            .as_ref()
            .map(Ident::to_string)
            .unwrap_or_else(|| format!(".{}", self.index))
    }

    pub fn is_init(&self) -> bool {
        matches!(self.source, InjectableFieldSource::InitValue)
    }

    fn with_init_ident_and_ty<R>(
        &self,
        f: impl FnOnce(std::option::Iter<Ident>, TokenStream) -> R,
    ) -> Option<R> {
        if !self.is_init() {
            return None;
        }
        let ty = &self.field.ty;
        Some(f(
            self.field.ident.iter(),
            quote!(<#ty as dijavu::Initializable>),
        ))
    }

    pub fn init_field_def(&self) -> Option<TokenStream> {
        self.with_init_ident_and_ty(|ident, ty| quote!(#(#ident:)* #ty::Init))
    }

    pub fn init_field_construct(&self) -> Option<TokenStream> {
        self.with_init_ident_and_ty(|ident, ty| {
            let error_msg = format!(
                "could not create initialization state for field `{}` of `{}`",
                self.field_name(),
                self.derive.ident
            );
            quote!(
                #(#ident:)* #ty::new_init::<Self>(container)
                    .map_err(|err| err.with_context(#error_msg))?
            )
        })
    }

    pub fn init_field_on_build(&self) -> Option<TokenStream> {
        self.with_init_ident_and_ty(|_, ty| {
            let accessor = self.accessor();
            let error_msg = format!(
                "could not build runtime state for field `{}` of `{}`",
                self.field_name(),
                self.derive.ident
            );
            quote!(
                #ty::on_build::<Self>(value.#accessor, data, builder)
                    .map_err(|err| err.with_context(#error_msg))?;
            )
        })
    }

    pub fn init_field_get_injectable(&self) -> TokenStream {
        let ty = &self.field.ty;
        let error_msg = format!(
            "could not inject field `{}` of `{}`",
            self.field_name(),
            self.derive.ident
        );
        let ident = self.field.ident.iter();
        match self.source {
            InjectableFieldSource::Injectable => {
                quote!(
                    #(#ident:)* <#ty as dijavu::Injectable>::get(container)
                        .map_err(|err| dijavu::Error::from(err).with_context(#error_msg))?
                )
            }
            InjectableFieldSource::InitValue => {
                quote!(
                    #(#ident:)* <#ty as dijavu::Initializable>::get::<Self>(container)
                        .map_err(|err| dijavu::Error::from(err).with_context(#error_msg))?
                )
            }
        }
    }
}
