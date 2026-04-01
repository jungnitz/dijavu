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
    init: Option<InitConfig>,
}

pub struct InitConfig {
    struct_name: Ident,
    automatic: bool,

    on_construct: Option<Expr>,
    on_build: Option<Expr>,
    on_start: Option<Expr>,
    on_start_async: Option<Expr>,
}

pub fn derive_injectable(input: DeriveInput, init: bool) -> syn::Result<TokenStream> {
    // TODO: cleanup this here mess
    #[derive(FromDeriveInput)]
    #[darling(attributes(inject))]
    struct Meta {
        #[darling(default)]
        init: Option<InitMeta>,
    }
    #[derive(Default, FromMeta)]
    struct InitMeta {
        auto: Flag,
        on_construct: Option<Expr>,
        on_build: Option<Expr>,
        on_start: Option<Expr>,
        on_start_async: Option<Expr>,
        #[darling(rename = "type")]
        ty: Option<Ident>,
    }

    let mut meta = Meta::from_derive_input(&input)?;
    if init {
        meta.init.get_or_insert_default();
    }

    let init = meta.init.map(|init| InitConfig {
        struct_name: init
            .ty
            .unwrap_or_else(|| format_ident!("{}Init", input.ident)),
        automatic: init.auto.is_present(),
        on_construct: init.on_construct,
        on_build: init.on_build,
        on_start: init.on_start,
        on_start_async: init.on_start_async,
    });
    let mode = DeriveInjectable(Rc::new(DeriveInjectableInner {
        span: input.span(),
        ident: input.ident.clone(),
        generics: GenericsHelper::from_generics(input.generics),
        init,
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

    let init_struct = mode.init.as_ref().map(|init| {
        let fields = fields.init_fields(phantom_data, &where_clause);
        let struct_name = &init.struct_name;
        quote! {
            #vis struct #struct_name<#impl_gen> #fields
        }
    });

    let impl_init_injectable = mode.init.as_ref().map(|init| {
        let construct = fields.init_construct(phantom_data);
        let on_build_hook = init.on_build.as_ref().map(|on_build| {
            quote!(
                ((#on_build)(&mut value, &mut *data, &mut *builder) as dijavu::Result<()>)?;
            )
        });
        let struct_name = &init.struct_name;
        let on_build = fields.init_on_build();
        let on_construct = init.on_construct.as_ref().map(|expr| quote!({
            let res: Result<(), dijavu::Error> = (#expr)(container);
            res?;
        }));
        let on_start = init
            .on_start
            .as_ref()
            .map(|on_start| quote!(builder.add_start_fn(#on_start);));
        let on_start_async = init.on_start_async.as_ref().map(|on_start_async| {
            quote!(
                builder.add_async_start_fn(|__dijavu_container: dijavu::AppContainer, __dijavu_data: &mut dijavu::Data| {
                    Box::pin(async move { (#on_start_async)(__dijavu_container, __dijavu_data).await })
                });
            )
        });
        let automatic = init.automatic.then(|| {
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
        quote! {
            impl<#impl_gen> dijavu::InitInjectable for #ident<#ty_gen> #where_clause {
                type InitError = dijavu::Error;
                type Init<'a> = &'a mut #struct_name<#ty_gen>;

                fn get_init(
                    container: &mut dijavu::InitAppContainer
                ) -> Result<Self::Init<'_>, Self::Error> {
                    dijavu::__private::init_injectable_get_init::<Self, #struct_name<#ty_gen>>(
                        container,
                        |container| {
                            #on_construct
                            Ok(#struct_name::<#ty_gen> #construct)
                        },
                        |mut value, data, builder| {
                            #on_build_hook
                            #on_build
                            #on_start
                            #on_start_async
                            Ok(())
                        }
                    )
                }
            }
            #automatic
        }
    });

    Ok(quote! {
        #init_struct
        #impl_init_injectable

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
