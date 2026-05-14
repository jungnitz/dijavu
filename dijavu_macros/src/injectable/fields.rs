use std::rc::Rc;

use crate::injectable::DeriveInjectableConfig;
use crate::injectable::field::InjectableField;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Fields;

pub struct InjectableFields {
    config: Rc<DeriveInjectableConfig>,
    fields: Vec<InjectableField>,
    named: bool,
}

impl InjectableFields {
    pub fn from_fields(config: Rc<DeriveInjectableConfig>, fields: Fields) -> syn::Result<Self> {
        Ok(Self {
            named: matches!(fields, Fields::Named(_)),
            fields: fields
                .into_iter()
                .enumerate()
                .map(|(idx, field)| InjectableField::from_field(config.clone(), idx, field))
                .collect::<syn::Result<_>>()?,
            config,
        })
    }

    pub fn field_decls(&self, defs: impl Fn(&InjectableField) -> TokenStream) -> TokenStream {
        let defs = self.fields.iter().map(defs);
        let where_clause = self.config.generics.split_for_impl().2;
        if self.named {
            quote!(#where_clause {
                #(#defs,)*
            })
        } else {
            quote!((#(#defs,)*) #where_clause;)
        }
    }

    pub fn construct_value(
        &self,
        field_construct: impl Fn(&InjectableField) -> TokenStream,
    ) -> TokenStream {
        let fields = self.fields.iter().map(field_construct);
        if self.named {
            quote!({
                #(#fields,)*
            })
        } else {
            quote!((
                #(#fields,)*
            ))
        }
    }

    pub fn init_field_decls(&self) -> TokenStream {
        self.field_decls(InjectableField::init_field_decl)
    }

    pub fn init_construct(&self) -> TokenStream {
        self.construct_value(InjectableField::init_field_construct)
    }

    pub fn runtime_field_decls(&self) -> TokenStream {
        self.field_decls(InjectableField::runtime_field_decl)
    }

    pub fn runtime_construct(&self) -> TokenStream {
        self.construct_value(InjectableField::runtime_field_construct)
    }

    pub fn construct_from_data_and_runtime(&self) -> TokenStream {
        let fields = self
            .fields
            .iter()
            .map(InjectableField::init_field_get_injectable);
        if self.named {
            quote!({
                #(#fields,)*
            })
        } else {
            quote!((
                #(#fields,)*
            ))
        }
    }
}
