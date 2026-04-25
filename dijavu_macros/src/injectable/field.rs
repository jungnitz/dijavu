use std::rc::Rc;

use crate::injectable::DeriveInjectableConfig;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::{Field, Meta};

pub struct InjectableField {
    pub config: Rc<DeriveInjectableConfig>,
    pub index: usize,
    pub field: Field,
    pub source: InjectableFieldSource,
}

pub enum InjectableFieldSource {
    Injectable,
    InitValue,
}

impl InjectableField {
    pub fn from_field(
        config: Rc<DeriveInjectableConfig>,
        index: usize,
        field: Field,
    ) -> syn::Result<Self> {
        let mut inject = false;
        for attr in &field.attrs {
            if attr.meta.path().to_token_stream().to_string() != "inject" {
                continue;
            }
            match &attr.meta {
                Meta::Path(_) => {
                    inject = true;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "did not expect a value for `#[inject]`",
                    ));
                }
            }
        }
        Ok(Self {
            index,
            field,
            source: if inject {
                InjectableFieldSource::Injectable
            } else {
                InjectableFieldSource::InitValue
            },
            config,
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

    fn with_init_ident_and_ty<R>(
        &self,
        f: impl FnOnce(std::option::Iter<Ident>, TokenStream) -> R,
    ) -> R {
        let ty = &self.field.ty;
        let ty = match self.source {
            InjectableFieldSource::Injectable => {
                quote!(<dijavu::Dependency<#ty> as dijavu::Initializable>)
            }
            InjectableFieldSource::InitValue => quote!(<#ty as dijavu::Initializable>),
        };
        f(self.field.ident.iter(), ty)
    }

    pub fn init_field_decl(&self) -> TokenStream {
        self.with_init_ident_and_ty(|ident, ty| quote!(#(#ident:)* #ty::Init))
    }

    pub fn init_field_construct(&self) -> TokenStream {
        self.with_init_ident_and_ty(|ident, ty| {
            let error_msg = format!(
                "could not create initialization state for field `{}` of `{}`",
                self.field_name(),
                self.config.ident
            );
            quote!(
                #(#ident:)* #ty::new_init_value(container)
                    .map_err(|err| dijavu::Error::from(err).with_context(#error_msg))?
            )
        })
    }

    pub fn runtime_field_decl(&self) -> TokenStream {
        self.with_init_ident_and_ty(|ident, ty| quote!(#(#ident:)* #ty::Runtime))
    }

    pub fn runtime_field_construct(&self) -> TokenStream {
        self.with_init_ident_and_ty(|ident, ty| {
            let accessor = self.accessor();
            let error_msg = format!(
                "could not create runtime state for field `{}` of `{}`",
                self.field_name(),
                self.config.ident
            );
            quote!(
                #(#ident:)* #ty::build_runtime_value(init.#accessor, data, builder)
                    .map_err(|err| dijavu::Error::from(err).with_context(#error_msg))?
            )
        })
    }

    pub fn init_field_get_injectable(&self) -> TokenStream {
        let ty = &self.field.ty;
        let error_msg = format!(
            "could not inject field `{}` of `{}`",
            self.field_name(),
            self.config.ident
        );
        let ident = self.field.ident.iter();
        match self.source {
            InjectableFieldSource::Injectable => {
                quote!(
                    #(#ident:)* <#ty as dijavu::Injectable>::get(data)
                        .map_err(|err| dijavu::Error::from(err).with_context(#error_msg))?
                )
            }
            InjectableFieldSource::InitValue => {
                let accessor = self.accessor();
                quote!(
                    #(#ident:)* <#ty as dijavu::Initializable>::from_runtime_value(&runtime.#accessor, data)
                        .map_err(|err| dijavu::Error::from(err).with_context(#error_msg))?
                )
            }
        }
    }
}
