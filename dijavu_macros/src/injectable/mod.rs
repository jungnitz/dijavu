mod field;
mod fields;

use crate::injectable::fields::InjectableFields;
use crate::utils::GenericsHelper;
use darling::util::Flag;
use darling::{FromDeriveInput, FromMeta};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use std::ops::Deref;
use std::rc::Rc;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Expr};

#[derive(Clone)]
pub struct DeriveInjectable(Rc<DeriveInjectableInner>);

pub struct DeriveInjectableInner {
    span: Span,
    ident: Ident,
    generics: GenericsHelper,
    init_struct_name: Option<Ident>,
    on_build: Option<Expr>,
    init: bool,
    automatic: bool,
}

pub fn derive_injectable(input: DeriveInput, init: bool) -> syn::Result<TokenStream> {
    // TODO: cleanup this here mess
    #[derive(FromDeriveInput)]
    #[darling(attributes(inject))]
    struct Meta {
        #[darling(default)]
        init: InitMeta,
    }
    #[derive(Default, FromMeta)]
    struct InitMeta {
        auto: Flag,
        on_build: Option<Expr>,
        #[darling(rename = "type")]
        ty: Option<Ident>,
    }

    let meta = Meta::from_derive_input(&input)?;
    let mode = DeriveInjectable(Rc::new(DeriveInjectableInner {
        span: input.span(),
        ident: input.ident.clone(),
        generics: GenericsHelper::from_generics(input.generics),
        init_struct_name: if init {
            Some(
                meta.init
                    .ty
                    .unwrap_or_else(|| format_ident!("{}Init", input.ident)),
            )
        } else {
            if meta.init.ty.is_some() {
                return Err(syn::Error::new(
                    meta.init.ty.span(),
                    "init only supported when deriving `InitInjectable`",
                ));
            }
            None
        },
        on_build: if init {
            meta.init.on_build
        } else {
            if meta.init.on_build.is_some() {
                return Err(syn::Error::new(
                    meta.init.on_build.span(),
                    "init only supported when deriving `InitInjectable`",
                ));
            }
            None
        },
        init,
        automatic: meta.init.auto.is_present(),
    }));
    let phantom_data = mode.generics.make_phantom_data();
    let phantom_data = phantom_data.as_ref();

    let vis = &input.vis;
    let ident = &input.ident;
    let Data::Struct(struct_data) = input.data else {
        return Err(syn::Error::new(mode.span, "must be a struct"));
    };

    let fields = InjectableFields::from_fields(&mode, struct_data.fields)?;

    let construct_from_container = fields.construct_from_container();

    let (impl_gen, ty_gen, where_clause) = mode.generics.split_for_impl();

    let init_struct = mode.init_struct_name.as_ref().map(|name| {
        let fields = fields.init_fields(phantom_data, &where_clause);
        quote! {
            #vis struct #name<#impl_gen> #fields
        }
    });

    let impl_init_injectable = mode.init_struct_name.as_ref().map(|name| {
        let construct = fields.init_construct(phantom_data);
        let on_build_hook = mode.on_build.as_ref().map(|on_build| {
            quote!(
                (#on_build(&mut value, &mut *data, &mut *builder) as dijavu::Result<()>)?;
            )
        });
        let on_build = fields.init_on_build();
        quote! {
            impl<#impl_gen> dijavu::InitInjectable for #ident<#ty_gen> #where_clause {
                type InitError = dijavu::Error;
                type Init<'a> = &'a mut #name<#ty_gen>;

                fn get_init(
                    container: &mut dijavu::InitAppContainer
                ) -> Result<Self::Init<'_>, Self::Error> {
                    dijavu::__private::init_injectable_get_init::<Self, #name<#ty_gen>>(
                        container,
                        |container| {
                            Ok(#name::<#ty_gen> #construct)
                        },
                        |mut value, data, builder| {
                            #on_build_hook
                            #on_build
                            Ok(())
                        }
                    )
                }
            }
        }
    });

    let automatic = mode.automatic.then(|| {
        quote! {
            #[dijavu::__private::ctor::ctor(anonymous, crate_path = dijavu::__private::ctor)]
            fn auto_init() {
                dijavu::hooks::add_global_before_build_hook(|container| {
                    <#ident as dijavu::InitInjectable>::get_init(container)?;
                    Ok(())
                })
            }
        }
    });

    Ok(quote! {
        #init_struct
        #impl_init_injectable
        #automatic

        impl<#impl_gen> dijavu::Injectable for #ident<#ty_gen> #where_clause {
            type Error = dijavu::Error;

            fn get(container: dijavu::AppContainer) -> Result<Self, Self::Error> {
                Ok(Self #construct_from_container)
            }
        }
    })
}

impl Deref for DeriveInjectable {
    type Target = DeriveInjectableInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
