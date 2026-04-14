use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields, Generics, Type, Visibility, spanned::Spanned};

pub fn derive_initializable(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let Data::Struct(struct_data) = input.data else {
        return Err(syn::Error::new(input.span(), "must be a struct"));
    };

    let ident = &input.ident;

    let init_struct = format_ident!("{}Init", ident);
    let init_struct_decl = declare_derived_struct(
        &input.vis,
        &init_struct,
        &input.generics,
        &struct_data.fields,
        |ty| quote!(<#ty as dijavu::Initializable>::Init),
    );

    let runtime_struct = format_ident!("{}Runtime", ident);
    let runtime_struct_decl = declare_derived_struct(
        &input.vis,
        &runtime_struct,
        &input.generics,
        &struct_data.fields,
        |ty| quote!(<#ty as dijavu::Initializable>::Runtime),
    );

    let new_init_value = instantiate_derived_struct(
        &init_struct,
        &input.generics,
        &struct_data.fields,
        |_, field| {
            let ty = &field.ty;
            quote!(<#ty as dijavu::Initializable>::new_init_value(container)
                .map_err(Into::<dijavu::Error>::into)?)
        },
    );
    let build_runtime_value = instantiate_from_similar_struct(
        &runtime_struct,
        &input.generics,
        &struct_data.fields,
        |accessor, field| {
            let ty = &field.ty;
            quote!(<#ty as dijavu::Initializable>::build_runtime_value(init.#accessor, data, builder)?)
        },
    );
    let from_runtime_value = instantiate_from_similar_struct(
        ident,
        &input.generics,
        &struct_data.fields,
        |accessor, field| {
            let ty = &field.ty;
            quote!(<#ty as dijavu::Initializable>::from_runtime_value(&runtime.#accessor, container)
                .map_err(Into::<dijavu::Error>::into)?)
        },
    );

    let (impl_gen, ty_gen, where_clause) = input.generics.split_for_impl();
    Ok(quote!(
        #init_struct_decl

        const _: () = {
            #runtime_struct_decl

            impl #impl_gen dijavu::Initializable for #ident #ty_gen #where_clause {
                type Error = dijavu::Error;
                type Init = #init_struct #ty_gen;
                type Runtime = #runtime_struct #ty_gen;

                fn new_init_value(
                    container: &mut dijavu::InitAppContainer
                ) -> dijavu::Result<Self::Init, Self::Error> {
                    Ok(#new_init_value)
                }

                fn build_runtime_value(
                    init: Self::Init,
                    data: &mut dijavu::Data,
                    builder: &mut dijavu::AppContainerBuilder
                ) -> dijavu::Result<Self::Runtime> {
                    Ok(#build_runtime_value)
                }

                fn from_runtime_value(
                    runtime: &'static Self::Runtime,
                    container: dijavu::AppContainer,
                ) -> Result<Self, Self::Error> {
                    Ok(#from_runtime_value)
                }
            }
        };
    ))
}

fn declare_derived_struct(
    vis: &Visibility,
    name: &Ident,
    generics: &Generics,
    fields: &Fields,
    field_ty: impl Fn(&Type) -> TokenStream,
) -> TokenStream {
    let (generics, _, where_clause) = generics.split_for_impl();
    match fields {
        Fields::Unit => quote!(#vis struct #name;),
        Fields::Named(fields) => {
            let fields = fields.named.iter();
            let field_vis = fields.clone().map(|field| &field.vis);
            let field_ident = fields.clone().map(|field| &field.ident);
            let field_ty = fields.clone().map(|field| field_ty(&field.ty));
            quote!(#vis struct #name #generics #where_clause {
                #(#field_vis #field_ident: #field_ty,)*
            })
        }
        Fields::Unnamed(fields) => {
            let fields = fields.unnamed.iter();
            let field_vis = fields.clone().map(|field| &field.vis);
            let field_ty = fields.clone().map(|field| field_ty(&field.ty));
            quote!(#vis struct #name #generics(
                #(#field_vis #field_ty,)*
            ) #where_clause;)
        }
    }
}

fn instantiate_from_similar_struct(
    name: &Ident,
    generics: &Generics,
    fields: &Fields,
    field_value: impl Fn(TokenStream, &Field) -> TokenStream,
) -> TokenStream {
    instantiate_derived_struct(name, generics, fields, |idx, field| {
        let accessor = field
            .ident
            .as_ref()
            .map(ToTokens::to_token_stream)
            .unwrap_or_else(|| proc_macro2::Literal::usize_unsuffixed(idx).into_token_stream());
        field_value(accessor, field)
    })
}

fn instantiate_derived_struct(
    name: &Ident,
    generics: &Generics,
    fields: &Fields,
    field_value: impl Fn(usize, &Field) -> TokenStream,
) -> TokenStream {
    let generics = generics.split_for_impl().1.as_turbofish();
    match fields {
        Fields::Unit => quote!(#name #generics),
        Fields::Named(fields) => {
            let fields = fields.named.iter();
            let values = fields
                .clone()
                .enumerate()
                .map(|(i, field)| field_value(i, field));
            let names = fields.map(|field| &field.ident);
            quote!(#name #generics {
                #(#names: #values,)*
            })
        }
        Fields::Unnamed(fields) => {
            let fields = fields.unnamed.iter();
            let values = fields
                .clone()
                .enumerate()
                .map(|(i, field)| field_value(i, field));
            quote!(#name #generics(
                #(#values,)*
            ))
        }
    }
}
