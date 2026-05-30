use crate::common::args::DeriveArgs;
use crate::common::field::InitializableField;
use proc_macro2::TokenStream;
use quote::quote;
use std::rc::Rc;
use syn::Fields;

pub struct InitializableFields {
    args: Rc<DeriveArgs>,
    fields: Vec<InitializableField>,
    named: bool,
}

impl InitializableFields {
    pub fn from_fields(config: Rc<DeriveArgs>, fields: Fields) -> syn::Result<Self> {
        Ok(Self {
            named: matches!(fields, Fields::Named(_)),
            fields: fields
                .into_iter()
                .enumerate()
                .map(|(idx, field)| InitializableField::new(config.clone(), idx, field))
                .collect::<Result<_, _>>()?,
            args: config,
        })
    }

    fn field_decls(&self, defs: impl Fn(&InitializableField) -> TokenStream) -> TokenStream {
        let defs = self.fields.iter().map(defs);
        let where_clause = self.args.generics.split_for_impl().2;
        if self.named {
            quote!(#where_clause {
                #(#defs,)*
            })
        } else {
            quote!((#(#defs,)*) #where_clause;)
        }
    }

    fn field_construct(
        &self,
        field_construct: impl Fn(&InitializableField) -> TokenStream,
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

    pub fn init_fields_decl(&self) -> TokenStream {
        self.field_decls(InitializableField::init_field_decl)
    }

    pub fn init(&self) -> TokenStream {
        self.field_construct(InitializableField::init)
    }

    pub fn build(&self) -> TokenStream {
        self.field_construct(InitializableField::build)
    }
}
