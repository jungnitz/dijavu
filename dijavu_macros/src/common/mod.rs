use crate::common::args::DeriveArgs;
use crate::common::fields::InitializableFields;
use darling::FromDeriveInput;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::rc::Rc;
use syn::{Data, DeriveInput};

mod args;
mod field;
mod fields;

pub struct StructOfInitializables {
    pub args: Rc<DeriveArgs>,
    fields: InitializableFields,
    pub init_struct_name: Ident,
}

impl StructOfInitializables {
    pub fn new(input: DeriveInput) -> syn::Result<Self> {
        let args = Rc::new(DeriveArgs::from_derive_input(&input)?);
        let Data::Struct(struct_data) = input.data else {
            return Err(syn::Error::new(input.ident.span(), "must be a struct"));
        };
        let fields = InitializableFields::from_fields(args.clone(), struct_data.fields)?;
        let init_struct_name = args
            .init
            .ident
            .clone()
            .unwrap_or_else(|| format_ident!("{}Init", &args.ident));
        Ok(Self {
            args,
            fields,
            init_struct_name,
        })
    }

    fn struct_def(&self, ident: &Ident, fields: &TokenStream) -> TokenStream {
        let vis = &self.args.vis;
        let impl_gen = self.args.generics.split_for_impl().0;
        quote!(#vis struct #ident <#impl_gen> #fields)
    }

    fn struct_value(&self, ident: &Ident, fields: &TokenStream) -> TokenStream {
        let ty_gen = self.args.generics.split_for_impl().1;
        quote!(#ident::<#ty_gen> #fields)
    }

    pub fn init_struct_def(&self) -> TokenStream {
        if self.args.init.hide.is_present() {
            return quote!();
        }
        self.struct_def(&self.init_struct_name, &self.fields.init_fields_decl())
    }

    pub fn hidden_init_struct_def(&self) -> TokenStream {
        if !self.args.init.hide.is_present() {
            return quote!();
        }
        self.struct_def(&self.init_struct_name, &self.fields.init_fields_decl())
    }

    pub fn init_value_with_hook(&self) -> TokenStream {
        let value = self.struct_value(&self.init_struct_name, &self.fields.init());
        let init_hook = self.args.run_init_hook();
        quote!({
            let mut init = #value;
            #init_hook
            init
        })
    }
    pub fn build(&self) -> TokenStream {
        let build_hook = self.args.run_build_hook();
        let value = self.struct_value(&self.args.ident, &self.fields.build());
        quote!({
            #build_hook
            #value
        })
    }
}
