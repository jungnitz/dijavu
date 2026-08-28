use crate::common::args::DeriveArgs;
use crate::common::field::InitializableField;
use proc_macro2::TokenStream;
use quote::quote;
use std::rc::Rc;
use syn::Fields;

pub struct InitializableFields {
    args: Rc<DeriveArgs>,
    fields: Vec<InitializableField>,
    ty: Type,
}

enum Type {
    Named,
    Tuple,
    Unit,
}

impl InitializableFields {
    pub fn from_fields(config: Rc<DeriveArgs>, fields: Fields) -> syn::Result<Self> {
        Ok(Self {
            ty: match fields {
                Fields::Named(_) => Type::Named,
                Fields::Unnamed(_) => Type::Tuple,
                Fields::Unit => Type::Unit,
            },
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
        match self.ty {
            Type::Named => quote!(#where_clause {
                #(#defs,)*
            }),
            Type::Tuple => quote!((#(#defs,)*) #where_clause;),
            Type::Unit => quote!(;),
        }
    }

    fn field_construct(
        &self,
        field_construct: impl Fn(&InitializableField) -> TokenStream,
    ) -> TokenStream {
        let fields = self.fields.iter().map(field_construct);
        match self.ty {
            Type::Named => quote!({
                #(#fields,)*
            }),
            Type::Tuple => quote!((
                #(#fields,)*
            )),
            Type::Unit => TokenStream::new(),
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
