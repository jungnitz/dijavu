use darling::{FromDeriveInput, FromMeta};
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Expr, Fields, GenericParam, Generics, Ident, Index, Member, Meta, Token,
    Type, Visibility, parse_quote,
};

use crate::visitable;

/// Derives `Injectable` and `Visitable`, and generates the state struct.
pub fn derive_injectable(input: DeriveInput) -> syn::Result<TokenStream> {
    let derive = Derive::new(&input)?;
    let state = &derive.state;
    let ident = &input.ident;
    let binds = derive.args.binds();

    let (_, ty_gen, _) = input.generics.split_for_impl();
    let owner = quote!(#ident #ty_gen);
    let state_struct = derive.state_struct(&input.generics, &owner);

    let declaration = derive.state_ty(quote!(dijavu::state::Declare));
    let preparation = derive.state_ty(quote!(dijavu::state::Prepare));
    let preparing = derive.state_ty(quote!(dijavu::state::Preparing<'__ctx>));

    let steps = derive.steps(&quote!(Self), &quote!(#(__ctx.bind::<#binds, Self>()?;)*));

    let (impl_gen, _, where_clause) = input.generics.split_for_impl();

    let mut preparing_generics = input.generics.clone();
    preparing_generics.params.insert(0, parse_quote!('__ctx));
    let (preparing_impl_gen, _, _) = preparing_generics.split_for_impl();

    let ctx = &derive.ctx;
    let members = derive.fields.iter().map(|field| &field.member);
    let members_again = members.clone();

    let visitable = visitable::derive_visitable(input.clone())?;
    Ok(quote!(
        #state_struct

        #visitable

        #[automatically_derived]
        impl #preparing_impl_gen dijavu::Preparing<'__ctx, #owner> for #preparing #where_clause {
            fn into_parts(
                self,
            ) -> (dijavu::PrepareContext<'__ctx, #owner>, #preparation) {
                (self.#ctx, #state { #ctx: (), #(#members: self.#members,)* })
            }
        }

        #[automatically_derived]
        impl #impl_gen dijavu::Injectable for #owner #where_clause {
            type Declaration = #declaration;
            type Preparation = #preparation;
            type Preparing<'__ctx> = #preparing;

            #steps

            fn make_preparing<'__ctx>(
                preparation: Self::Preparation,
                ctx: dijavu::PrepareContext<'__ctx, Self>,
            ) -> Self::Preparing<'__ctx> {
                #state { #ctx: ctx, #(#members_again: preparation.#members_again,)* }
            }
        }
    ))
}

/// Derives `InjectableField` and `Visitable`, and generates the state struct.
pub fn derive_injectable_field(input: DeriveInput) -> syn::Result<TokenStream> {
    let mut derive = Derive::new(&input)?;
    if let Some(bind) = derive.args.binds().next() {
        return Err(syn::Error::new(
            bind.span(),
            "`bind` is not supported for `InjectableField`",
        ));
    }
    derive.state_args.push(quote!(__I));

    // unlike for injectables, bounds on the field types can't lead to cycles here
    let mut generics = input.generics.clone();
    generics.params.push(parse_quote!(__I: dijavu::Injectable));
    let where_clause = generics.make_where_clause();
    for field in &derive.fields {
        let ty = field.ty;
        where_clause
            .predicates
            .push(parse_quote!(#ty: dijavu::InjectableField<__I>));
    }

    let state_struct = derive.state_struct(&generics, &quote!(__I));
    let declaration = derive.state_ty(quote!(dijavu::state::Declare));
    let preparation = derive.state_ty(quote!(dijavu::state::Prepare));
    let steps = derive.steps(&quote!(__I), &quote!());

    let ident = &input.ident;
    let (impl_gen, _, where_clause) = generics.split_for_impl();
    let (_, ty_gen, _) = input.generics.split_for_impl();

    let visitable = visitable::derive_visitable(input.clone())?;
    Ok(quote!(
        #state_struct

        #visitable

        #[automatically_derived]
        impl #impl_gen dijavu::InjectableField<__I> for #ident #ty_gen #where_clause {
            type Declaration = #declaration;
            type Preparation = #preparation;

            #steps
        }
    ))
}

#[expect(clippy::redundant_field_names, reason = "emitted by FromDeriveInput")]
mod args {
    use super::{Expr, FromDeriveInput, List, Type};

    /// The attributes given via `#[injectable(...)]`.
    #[derive(FromDeriveInput)]
    #[darling(attributes(injectable))]
    pub struct Args {
        pub declare: Option<Expr>,
        pub prepare: Option<Expr>,
        pub assemble: Option<Expr>,
        #[darling(multiple)]
        pub bind: Vec<List<Type>>,
        #[darling(multiple)]
        pub register: Vec<List<Expr>>,
    }
}
use args::Args;

impl Args {
    fn binds(&self) -> impl Iterator<Item = &Type> {
        self.bind.iter().flat_map(|binds| &binds.0)
    }

    fn registers(&self) -> impl Iterator<Item = &Expr> {
        self.register.iter().flat_map(|registers| &registers.0)
    }
}

/// The items within an attribute like `bind(...)`.
struct List<T>(Vec<T>);

impl<T: Parse> FromMeta for List<T> {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        let items = item
            .require_list()?
            .parse_args_with(Punctuated::<T, Token![,]>::parse_terminated)?;
        Ok(Self(items.into_iter().collect()))
    }
}

struct Field<'a> {
    member: Member,
    vis: &'a Visibility,
    ty: &'a Type,
}

/// The parts shared between the derive macros.
struct Derive<'a> {
    args: Args,
    fields: Vec<Field<'a>>,
    is_tuple: bool,
    /// The name of the generated struct
    state: Ident,
    /// The generic arguments of the generated struct, except for the build step
    state_args: Vec<TokenStream>,
    /// The field of the generated struct containing `InjectableState::ContextField`
    ctx: Member,
    vis: &'a Visibility,
    doc: String,
}

impl<'a> Derive<'a> {
    fn new(input: &'a DeriveInput) -> syn::Result<Self> {
        let data = match &input.data {
            Data::Struct(data) => data,
            Data::Enum(data) => {
                return Err(syn::Error::new(
                    data.enum_token.span(),
                    "injectables and their fields can only be derived for structs",
                ));
            }
            Data::Union(data) => {
                return Err(syn::Error::new(
                    data.union_token.span(),
                    "injectables and their fields can only be derived for structs",
                ));
            }
        };
        if let Some(lifetime) = input.generics.lifetimes().next() {
            return Err(syn::Error::new(
                lifetime.span(),
                "injectables and their fields must be `'static`, so they can't have lifetime \
                 parameters",
            ));
        }

        let mut fields = Vec::new();
        for (index, field) in data.fields.iter().enumerate() {
            // `Self` would refer to the generated struct there
            if let Some(span) = find_self(field.ty.to_token_stream()) {
                return Err(syn::Error::new(
                    span,
                    "`Self` is not supported in field types here, use the name of the struct",
                ));
            }
            if field.ident.as_ref().is_some_and(|ident| ident == "ctx") {
                return Err(syn::Error::new(
                    field.span(),
                    "the name `ctx` is reserved for the context in the generated state struct",
                ));
            }
            fields.push(Field {
                member: member(field.ident.as_ref(), index),
                vis: &field.vis,
                ty: &field.ty,
            });
        }

        let is_tuple = matches!(data.fields, Fields::Unnamed(_));
        let ident = &input.ident;
        Ok(Self {
            args: Args::from_derive_input(input)?,
            ctx: if is_tuple {
                member(None, fields.len())
            } else {
                Member::Named(format_ident!("ctx"))
            },
            fields,
            is_tuple,
            state: format_ident!("{ident}State"),
            state_args: input.generics.params.iter().map(generic_arg).collect(),
            vis: &input.vis,
            doc: format!("The state of [`{ident}`] during the build."),
        })
    }

    /// Returns the generated struct's type in the given build step.
    fn state_ty(&self, step: TokenStream) -> TokenStream {
        let state = &self.state;
        let args = &self.state_args;
        quote!(#state<#(#args,)* #step>)
    }

    /// Returns the definition of the generated struct.
    fn state_struct(&self, generics: &Generics, owner: &TokenStream) -> TokenStream {
        let mut generics = generics.clone();
        generics
            .params
            .push(parse_quote!(__S: dijavu::state::InjectableState<#owner>));
        let (impl_gen, _, where_clause) = generics.split_for_impl();

        let state = &self.state;
        let vis = self.vis;
        let doc = &self.doc;
        let context = quote!(<__S as dijavu::state::InjectableState<#owner>>::ContextField);
        let fields = self.fields.iter().map(|field| {
            let vis = field.vis;
            let ty = field.ty;
            let ty = quote!(<__S as dijavu::state::InjectableState<#owner>>::Field<#ty>);
            match &field.member {
                Member::Named(ident) => quote!(#vis #ident: #ty),
                Member::Unnamed(_) => quote!(#vis #ty),
            }
        });
        let body = if self.is_tuple {
            quote!((#(#fields,)* pub #context) #where_clause;)
        } else {
            quote!(#where_clause { #(#fields,)* pub ctx: #context })
        };
        quote!(
            #[doc = #doc]
            #[allow(dead_code)]
            #vis struct #state #impl_gen #body
        )
    }

    /// Returns the `declare`, `prepare` and `assemble` functions, which have the same signatures
    /// for injectables and their fields.
    fn steps(&self, owner: &TokenStream, after_declare: &TokenStream) -> TokenStream {
        let state = &self.state;
        let ctx = &self.ctx;
        let field = |step: &str, input: fn(&Member) -> TokenStream, suffix: TokenStream| {
            let step = format_ident!("{step}");
            let fields = self.fields.iter().map(move |field| {
                let member = &field.member;
                let ty = field.ty;
                let input = input(member);
                let suffix = &suffix;
                quote!(
                    #member: <#ty as dijavu::InjectableField<#owner>>::#step(
                        #input __ctx.as_owned()
                    ) #suffix?
                )
            });
            quote!(#(#fields,)*)
        };

        let declare_fields = field("declare", |_| quote!(), quote!());
        let prepare_fields = field("prepare", |m| quote!(__state.#m,), quote!(.await));
        let assemble_fields = field("assemble", |m| quote!(__state.#m,), quote!(.await));

        let declare_hook = run_hook(self.args.declare.as_ref(), quote!());
        let prepare_hook = run_hook(self.args.prepare.as_ref(), quote!(.await));
        let assemble_hook = run_hook(self.args.assemble.as_ref(), quote!(.await));
        let registers = self.args.registers();

        quote!(
            #[allow(unused_mut, unused_variables)]
            fn declare(
                mut __ctx: dijavu::DeclareContext<'_, #owner>,
            ) -> ::core::result::Result<Self::Declaration, dijavu::Error> {
                let mut __state: Self::Declaration = #state { #declare_fields #ctx: () };
                #declare_hook
                #(__ctx.register(#registers);)*
                #after_declare
                ::core::result::Result::Ok(__state)
            }

            #[allow(unused_mut, unused_variables)]
            async fn prepare(
                mut __state: Self::Declaration,
                mut __ctx: dijavu::PrepareContext<'_, #owner>,
            ) -> ::core::result::Result<Self::Preparation, dijavu::Error> {
                #prepare_hook
                ::core::result::Result::Ok(#state { #prepare_fields #ctx: () })
            }

            #[allow(unused_mut, unused_variables)]
            async fn assemble(
                mut __state: Self::Preparation,
                mut __ctx: dijavu::AssembleContext<'_, #owner>,
            ) -> ::core::result::Result<Self, dijavu::Error> {
                #assemble_hook
                ::core::result::Result::Ok(Self { #assemble_fields })
            }
        )
    }
}

fn run_hook(hook: Option<&Expr>, suffix: TokenStream) -> Option<TokenStream> {
    hook.map(|hook| {
        quote!(
            let _: () = ::core::result::Result::map_err(
                (#hook)(&mut __state, __ctx.as_owned()) #suffix,
                ::core::convert::Into::<dijavu::Error>::into,
            )?;
        )
    })
}

fn member(ident: Option<&Ident>, index: usize) -> Member {
    match ident {
        Some(ident) => Member::Named(ident.clone()),
        None => Member::Unnamed(Index::from(index)),
    }
}

fn generic_arg(param: &GenericParam) -> TokenStream {
    match param {
        GenericParam::Lifetime(param) => param.lifetime.to_token_stream(),
        GenericParam::Type(param) => param.ident.to_token_stream(),
        GenericParam::Const(param) => param.ident.to_token_stream(),
    }
}

fn find_self(tokens: TokenStream) -> Option<Span> {
    tokens.into_iter().find_map(|token| match token {
        TokenTree::Ident(ident) if ident == "Self" => Some(ident.span()),
        TokenTree::Group(group) => find_self(group.stream()),
        _ => None,
    })
}
