use crate::common::StructOfInitializables;
use crate::utils::doc_hidden;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

pub fn derive_injectable(input: DeriveInput) -> syn::Result<TokenStream> {
    let init = StructOfInitializables::new(input, "InitData")?;
    init.args.check_for_injectable()?;
    let ident = &init.args.ident;
    let (impl_gen, ty_gen, where_clause) = init.args.generics.split_for_impl();

    let init_struct_name = format_ident!("{ident}Init");
    let init_struct_attrs = doc_hidden(init.args.init.hide.is_present());
    let vis = &init.args.vis;

    let init_data_struct_name = &init.init_data_struct_name;
    let init_data_struct_def = init.init_data_struct_def();
    let init_data_struct_attrs = doc_hidden(
        init.args
            .init
            .data
            .inner
            .as_ref()
            .and_then(|data| data.hide.as_ref())
            .map(|lit| lit.value)
            .unwrap_or(true),
    );
    let init_data = init.init_data();

    let run_init_hook = init.args.init.hook.as_ref().map(|hook| {
        quote!(
            let _: () = (#hook)(
                #init_struct_name::<'_, #ty_gen>(dijavu::InjectableInit::new(&mut data, injector))
            )
            .await
            .map_err(Into::<dijavu::Error>::into)?;
        )
    });

    let build = init.build();

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
        #init_struct_attrs
        #vis struct #init_struct_name<'init, #impl_gen>(
            dijavu::InjectableInit<'init, #ident<#ty_gen>>
        ) #where_clause;

        #init_data_struct_attrs
        #init_data_struct_def

        const _: () = {
            impl<#impl_gen> dijavu::Injectable for #ident<#ty_gen> #where_clause {
                type Error = dijavu::Error;
                type Data = #init_data_struct_name<#ty_gen>;
                type Init<'init> = #init_struct_name<'init, #ty_gen>;

                #[allow(clippy::unused_async_trait_impl)]
                async fn new_init_data(
                    injector: &mut dijavu::InitInjector,
                    _token: dijavu::Restricted<Self>,
                ) -> dijavu::Result<
                    <Self as dijavu::Injectable>::Data,
                    <Self as dijavu::Injectable>::Error
                > {
                    let mut data = #init_data;
                    #run_init_hook
                    Ok(data)
                }

                fn new_init(init: dijavu::InjectableInit<'_, Self>) -> #init_struct_name<'_, #ty_gen> {
                    #init_struct_name(init)
                }

                #[allow(clippy::unused_async_trait_impl)]
                async fn build(
                    mut init: <Self as dijavu::Injectable>::Data,
                    builder: &mut dijavu::InjectorBuilder,
                    _token: dijavu::Restricted<Self>,
                ) -> dijavu::Result<Self> {
                    Ok(#build)
                }
            }

            #auto
        };
    ))
}
