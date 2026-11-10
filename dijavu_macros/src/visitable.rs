use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Field, Fields, Ident, Path, Type, WherePredicate, parse_quote};

/// Derives `Visitable` by forwarding the visitor to every field not marked with
/// `#[visitable(skip)]`.
///
/// This is sound as long as the generated code only visits fields that are owned directly by the
/// value: those can only be moved through `&mut self`. What happens within a field is covered by
/// the field type's own `Visitable` implementation.
pub fn derive_visitable(input: DeriveInput) -> syn::Result<TokenStream> {
    let krate = crate_path(&input)?.unwrap_or_else(|| parse_quote!(dijavu));
    derive_visitable_with_crate(input, &krate)
}

/// Derives `Visitable`, referring to the `dijavu` crate via `krate`.
pub fn derive_visitable_with_crate(
    mut input: DeriveInput,
    krate: &Path,
) -> syn::Result<TokenStream> {
    check_not_packed(&input)?;

    let mut field_types = Vec::new();
    let body = match &input.data {
        Data::Struct(data) => {
            let (pattern, visits) =
                destructure(krate, quote!(Self), &data.fields, &mut field_types)?;
            quote!(
                let #pattern = self;
                #(#visits)*
            )
        }
        Data::Enum(data) => {
            let arms = data
                .variants
                .iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let (pattern, visits) = destructure(
                        krate,
                        quote!(Self::#ident),
                        &variant.fields,
                        &mut field_types,
                    )?;
                    Ok(quote!(#pattern => { #(#visits)* }))
                })
                .collect::<syn::Result<Vec<_>>>()?;
            quote!(
                match self {
                    #(#arms)*
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
            )
        }
        Data::Union(data) => {
            return Err(syn::Error::new(
                data.union_token.span(),
                "`Visitable` cannot be derived for unions",
            ));
        }
    };

    // require `Visitable` for every visited field type, so that generic fields work as well
    let where_clause = input.generics.make_where_clause();
    for ty in field_types {
        let predicate: WherePredicate = parse_quote!(#ty: #krate::Visitable);
        where_clause.predicates.push(predicate);
    }

    let ident = &input.ident;
    let (impl_gen, ty_gen, where_clause) = input.generics.split_for_impl();
    Ok(quote!(
        // SAFETY: only fields owned directly by this value are visited, which can only be moved
        // through `&mut self`. Each call visits the same fields and does not dereference anything.
        #[automatically_derived]
        unsafe impl #impl_gen #krate::Visitable for #ident #ty_gen #where_clause {
            #[allow(unused_variables)]
            fn visit_refs(&mut self, visitor: &mut #krate::RefVisitor<'_>) {
                #body
            }
        }
    ))
}

/// Returns a pattern binding all visited fields and the code visiting them.
fn destructure(
    krate: &Path,
    path: TokenStream,
    fields: &Fields,
    field_types: &mut Vec<Type>,
) -> syn::Result<(TokenStream, Vec<TokenStream>)> {
    let mut bindings = Vec::new();
    let mut visits = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if is_skipped(field)? {
            continue;
        }
        let binding = format_ident!("__field_{index}", span = Span::mixed_site());
        let member = match &field.ident {
            Some(ident) => quote!(#ident),
            None => {
                let index = syn::Index::from(index);
                quote!(#index)
            }
        };
        let ty = &field.ty;
        visits.push(quote!(<#ty as #krate::Visitable>::visit_refs(#binding, visitor);));
        bindings.push(quote!(#member: #binding));
        field_types.push(field.ty.clone());
    }
    Ok((quote!(#path { #(#bindings,)* .. }), visits))
}

/// Returns the path given via `#[visitable(crate = path)]` on the container.
fn crate_path(input: &DeriveInput) -> syn::Result<Option<Path>> {
    let mut krate = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("visitable") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                krate = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("unknown `visitable` attribute, expected `crate`"))
            }
        })?;
    }
    Ok(krate)
}

fn is_skipped(field: &Field) -> syn::Result<bool> {
    let mut skip = false;
    for attr in &field.attrs {
        if !attr.path().is_ident("visitable") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
                Ok(())
            } else {
                Err(meta.error("unknown `visitable` attribute, expected `skip`"))
            }
        })?;
    }
    Ok(skip)
}

/// Fields of packed structs cannot be borrowed, so reject them with a clear error.
fn check_not_packed(input: &DeriveInput) -> syn::Result<()> {
    for attr in &input.attrs {
        if !attr.path().is_ident("repr") {
            continue;
        }
        let mut packed = None::<Ident>;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("packed") {
                packed = meta.path.get_ident().cloned();
            }
            // skip arguments like `align(8)` or `packed(2)`
            if meta.input.peek(syn::token::Paren) {
                let content;
                syn::parenthesized!(content in meta.input);
                content.parse::<TokenStream>()?;
            }
            Ok(())
        })?;
        if let Some(packed) = packed {
            return Err(syn::Error::new(
                packed.span(),
                "`Visitable` cannot be derived for packed structs",
            ));
        }
    }
    Ok(())
}
