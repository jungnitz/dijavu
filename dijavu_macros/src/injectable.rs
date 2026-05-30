use crate::common::StructOfInitializables;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn derive_injectable(input: DeriveInput) -> syn::Result<TokenStream> {
    let init = StructOfInitializables::new(input)?;

    let ident = &init.args.ident;

    let init_struct_name = &init.init_struct_name;
    let init_struct_def = init.init_struct_def();
    let hidden_init_struct_def = init.hidden_init_struct_def();
    let init_value = init.init_value_with_hook();

    let build = init.build();

    let (impl_gen, ty_gen, where_clause) = init.args.generics.split_for_impl();

    let auto = &init.args.init.auto;
    let auto = if auto.is_present() {
        if !init.args.generics.is_empty() {
            return Err(syn::Error::new(
                auto.span(),
                "cannot automatically initialize a struct with generics",
            ));
        }
        Some(quote! {
            #[dijavu::__private::ctor::ctor(unsafe, anonymous, crate_path = dijavu::__private::ctor)]
            fn auto_init() {
                dijavu::hooks::add_global_before_build_hook(Box::new(|injector| Box::pin(async move {
                    injector.get::<#ident>().await?;
                    Ok(())
                })))
            }
        })
    } else {
        None
    };
    Ok(quote!(
        #init_struct_def

        const _: () = {
            #hidden_init_struct_def

            struct __DijavuInjectableKey<#impl_gen> (#ident<#ty_gen>) #where_clause;

            impl<#impl_gen> dijavu::DataKey for __DijavuInjectableKey<#ty_gen> #where_clause {
                type Value = #init_struct_name<#ty_gen>;
            }

            impl<#impl_gen> dijavu::Injectable for #ident<#ty_gen> #where_clause {
                type Error = dijavu::Error;
                type Init<'a> = &'a mut #init_struct_name<#ty_gen>;

                async fn init(
                    injector: &mut dijavu::InitInjector,
                    _token: dijavu::Restricted
                ) -> dijavu::Result<<Self as dijavu::Injectable>::Init<'_>, <Self as dijavu::Injectable>::Error> {
                    if injector.data_mut().contains_key::<__DijavuInjectableKey<#ty_gen>>() {
                        return Ok(injector
                            .data_mut()
                            .get_mut::<__DijavuInjectableKey<#ty_gen>>()
                            .unwrap());
                    }
                    let init = #init_value;
                    let dijavu::data::DataEntry::Vacant(entry) = injector.data_mut().entry::<__DijavuInjectableKey<#ty_gen>>() else {
                        unreachable!();
                    };
                    Ok(entry.insert(init))
                }

                async fn build(
                    builder: &mut dijavu::InjectorBuilder,
                    _token: dijavu::Restricted
                ) -> dijavu::Result<Self> {
                    let init = builder.init_data_mut().remove::<__DijavuInjectableKey<#ty_gen>>();
                    let Some(mut init) = init else {
                        return Err(dijavu::Error::msg("injectable was never initialized"));
                    };
                    Ok(#build)
                }
            }

            #auto
        };
    ))
}
