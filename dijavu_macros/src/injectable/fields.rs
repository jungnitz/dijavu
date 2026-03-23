use crate::injectable::DeriveInjectable;
use crate::injectable::field::InjectableField;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Fields;

pub struct InjectableFields {
    fields: Vec<InjectableField>,
    named: bool,
}

impl InjectableFields {
    pub fn from_fields(mode: &DeriveInjectable, fields: Fields) -> syn::Result<Self> {
        Ok(InjectableFields {
            named: matches!(fields, Fields::Named(_)),
            fields: fields
                .into_iter()
                .enumerate()
                .map(|(idx, field)| InjectableField::from_field(mode, idx, field))
                .collect::<syn::Result<_>>()?,
        })
    }

    pub fn init_fields(
        &self,
        phantom_data: Option<&TokenStream>,
        where_clause: impl ToTokens,
    ) -> TokenStream {
        let defs = self
            .fields
            .iter()
            .filter_map(InjectableField::init_field_def);
        let phantom_data = phantom_data.into_iter();
        if self.named {
            quote!(#where_clause {
                #(#defs,)*
                #(_dijavu_pd: #phantom_data,)*
            })
        } else {
            quote!((#(#defs,)* #(#phantom_data,)*) #where_clause;)
        }
    }

    pub fn init_construct(&self, phantom_data: Option<&TokenStream>) -> TokenStream {
        let phantom_data = phantom_data.map(|_| quote!(PhantomData)).into_iter();
        let fields = self
            .fields
            .iter()
            .filter_map(InjectableField::init_field_construct);
        if self.named {
            quote!({
                #(#fields,)*
                #(_dijavu_pd: #phantom_data,)*
            })
        } else {
            quote!((
                #(#fields,)*
                #(#phantom_data,)*
            ))
        }
    }

    pub fn init_on_build(&self) -> TokenStream {
        self.fields
            .iter()
            .filter_map(InjectableField::init_field_on_build)
            .collect()
    }

    pub fn construct_from_container(&self) -> TokenStream {
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
