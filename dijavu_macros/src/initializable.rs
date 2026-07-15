use crate::common::StructOfInitializables;
use crate::utils::doc_hidden;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn derive_initializable(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let init = StructOfInitializables::new(input, "Init")?;
    init.args.check_for_initializable()?;
    let data = &init.args.init.data;
    if data.inner.is_some() {
        return Err(syn::Error::new(
            data.span,
            "`data` is only supported for `#[derive(Injectable)]`",
        ));
    }

    let ident = &init.args.ident;

    let init_struct_name = &init.init_data_struct_name;
    let init_struct_def = init.init_data_struct_def();
    let init_struct_def_attr = doc_hidden(init.args.init.hide.is_present());
    let init_value = init.init_data();
    let run_init_hook = init.args.init.hook.as_ref().map(|hook| {
        quote!(
            let _: () = (#hook)(&mut init, injector).await?;
        )
    });

    let build = init.build();

    let (impl_gen, ty_gen, where_clause) = init.args.generics.split_for_impl();

    let impl_new_init_value = (!init.args.init.manual.is_present()).then(|| {
        quote!(
            impl<#impl_gen> dijavu::NewInitValue for #ident<#ty_gen> #where_clause {
                type Error = dijavu::Error;

                async fn new_init(
                    injector: &mut dijavu::InitInjector
                ) -> dijavu::Result<<Self as dijavu::Initializable>::Init, Self::Error> {
                    let mut init = #init_value;
                    #run_init_hook
                    Ok(init)
                }
            }
        )
    });
    Ok(quote!(
        #init_struct_def_attr
        #init_struct_def

        const _: () = {
            impl<#impl_gen> dijavu::Initializable for #ident<#ty_gen> #where_clause {
                type Init = #init_struct_name<#ty_gen>;

                async fn build(
                    init: <Self as dijavu::Initializable>::Init,
                    builder: &mut dijavu::InjectorBuilder,
                ) -> dijavu::Result<Self> {
                    Ok(#build)
                }
            }

            #impl_new_init_value
        };
    ))
}
