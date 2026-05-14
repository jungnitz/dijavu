mod field;
mod fields;

use crate::injectable::fields::InjectableFields;
use crate::utils::GenericsHelper;
use darling::util::Flag;
use darling::{FromDeriveInput, FromMeta};
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use std::rc::Rc;
use syn::{Data, DeriveInput, Expr, Visibility};

struct DeriveInjectableConfig {
    vis: Visibility,
    ident: Ident,
    generics: GenericsHelper,
    init: InitConfig,
}

pub struct InitConfig {
    struct_name: Ident,
    hide_struct: bool,
    runtime_struct_name: Ident,
    automatic: bool,

    on_construct: Option<Expr>,
    on_build: Option<Expr>,
    on_build_async: Option<Expr>,
    on_start: Option<Expr>,
    on_start_async: Option<Expr>,
}

pub fn derive_injectable(input: DeriveInput) -> syn::Result<TokenStream> {
    #[derive(FromDeriveInput)]
    #[darling(attributes(inject))]
    struct Meta {
        #[darling(default)]
        init: InitMeta,
    }
    #[derive(Default, FromMeta)]
    struct InitMeta {
        auto: Option<bool>,
        on_construct: Option<Expr>,
        on_build: Option<Expr>,
        on_build_async: Option<Expr>,
        on_start: Option<Expr>,
        on_start_async: Option<Expr>,
        #[darling(rename = "type")]
        ty: Option<Ident>,
        hide: Flag,
    }

    let meta = Meta::from_derive_input(&input)?;
    let init = meta.init;
    let config = Rc::new(DeriveInjectableConfig {
        vis: input.vis,
        ident: input.ident.clone(),
        generics: GenericsHelper::from_generics(input.generics),
        init: InitConfig {
            struct_name: init
                .ty
                .unwrap_or_else(|| format_ident!("{}Init", input.ident)),
            hide_struct: init.hide.is_present(),
            runtime_struct_name: format_ident!("{}Runtime", input.ident),
            automatic: init.auto.unwrap_or(default_auto()),
            on_construct: init.on_construct,
            on_build: init.on_build,
            on_build_async: init.on_build_async,
            on_start: init.on_start,
            on_start_async: init.on_start_async,
        },
    });
    let init = &config.init;
    let Data::Struct(struct_data) = input.data else {
        return Err(syn::Error::new(config.ident.span(), "must be a struct"));
    };

    let fields = InjectableFields::from_fields(config.clone(), struct_data.fields)?;
    let fields = &fields;

    let (init_struct, init_struct_hidden) = {
        let def = DerivedStruct {
            config: config.clone(),
            fields,
            name: &init.struct_name,
            decls: InjectableFields::init_field_decls,
        };
        if init.hide_struct {
            (None, Some(def))
        } else {
            (Some(def), None)
        }
    };

    let runtime_struct = DerivedStruct {
        config: config.clone(),
        fields,
        name: &init.runtime_struct_name,
        decls: InjectableFields::runtime_field_decls,
    };
    let impl_injectable = ImplInjectable {
        fields,
        config: config.clone(),
    };

    Ok(quote! {
        #init_struct
        const _: () = {
            #init_struct_hidden
            #runtime_struct
            #impl_injectable
        };
    })
}

struct DerivedStruct<'a> {
    fields: &'a InjectableFields,
    config: Rc<DeriveInjectableConfig>,
    name: &'a Ident,
    decls: fn(&InjectableFields) -> TokenStream,
}

impl ToTokens for DerivedStruct<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = self.name;
        let (impl_gen, _, _) = self.config.generics.split_for_impl();
        let fields = (self.decls)(self.fields);
        let vis = &self.config.vis;
        tokens.extend(quote!(
            #vis struct #name<#impl_gen> #fields
        ))
    }
}

struct ImplInjectable<'a> {
    fields: &'a InjectableFields,
    config: Rc<DeriveInjectableConfig>,
}

impl ToTokens for ImplInjectable<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let init = &self.config.init;
        let init_construct = self.fields.init_construct();
        let init_struct_name = &init.struct_name;
        let runtime_struct_name = &init.runtime_struct_name;

        // hooks
        let on_construct = init.on_construct.as_ref().map(|expr| {
            quote!({
                let res: Result<(), dijavu::Error> = (#expr)(container);
                res?;
            })
        });
        let on_build = init.on_build.iter();
        let on_build_async = init.on_build_async.iter();
        let on_build = quote!({
            #(
            let res: Result<(), dijavu::Error> = (#on_build)(&mut init, &mut *data, &mut *builder);
            res?;
            )*
            #(
            let res: Result<(), dijavu::Error> = (#on_build_async)(&mut init, &mut *data, &mut *builder).await;
            res?;
            )*
        });
        let on_start = init.on_start.iter();
        let on_start_async = init.on_start_async.iter();
        let on_start = quote!({
            #( builder.add_start_fn(#on_start); )*
            #( builder.add_async_start_fn(|__dijavu_container: dijavu::AppContainer, __dijavu_data: &mut dijavu::BuildData| {
                Box::pin(async move { (#on_start_async)(__dijavu_container, __dijavu_data).await })
            }); )*
        });

        // automatic initialization
        let ident = &self.config.ident;
        let automatic = init.automatic.then(|| {
            quote! {
                #[dijavu::__private::ctor::ctor(anonymous, crate_path = dijavu::__private::ctor)]
                fn auto_init() {
                    dijavu::hooks::add_global_before_build_hook(|container| {
                        <#ident as dijavu::Injectable>::get_init(container)?;
                        Ok(())
                    })
                }
            }
        });

        let (impl_gen, ty_gen, where_clause) = self.config.generics.split_for_impl();
        let runtime_construct = self.fields.runtime_construct();

        let get_runtime_value = quote!(
            let runtime = dijavu::__private::impl_injectable_get_runtime::<Self, #runtime_struct_name<#ty_gen>>(data)?;
        );
        let construct_from_data = self.fields.construct_from_data_and_runtime();

        tokens.extend(quote! {
            impl<#impl_gen> dijavu::Injectable for #ident<#ty_gen> #where_clause {
                type Error = dijavu::Error;
                type Init<'a> = &'a mut #init_struct_name<#ty_gen>;

                fn get_init(
                    container: &mut dijavu::InitAppContainer
                ) -> Result<Self::Init<'_>, Self::Error> {
                    dijavu::__private::impl_injectable_get_init::<Self, #init_struct_name<#ty_gen>, #runtime_struct_name::<#ty_gen>>(
                        container,
                        |container| {
                            #on_construct
                            Ok(#init_struct_name::<#ty_gen> #init_construct)
                        },
                        |mut init, data, builder| {
                            Box::pin(async move {
                                #on_build
                                #on_start
                                Ok(#runtime_struct_name::<#ty_gen> #runtime_construct)
                            })
                        }
                    )
                }

                fn get(data: &dijavu::RuntimeData) -> Result<Self, Self::Error> {
                    #get_runtime_value
                    Ok(Self #construct_from_data)
                }
            }
            #automatic
        });
    }
}

#[cfg(feature = "auto_init_default")]
fn default_auto() -> bool {
    true
}

#[cfg(not(feature = "auto_init_default"))]
fn default_auto() -> bool {
    false
}
