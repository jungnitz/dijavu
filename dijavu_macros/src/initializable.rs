use crate::common::StructOfInitializables;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn derive_initializable(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let init = StructOfInitializables::new(input)?;
    let auto = &init.args.init.auto;
    if auto.is_present() {
        return Err(syn::Error::new(
            auto.span(),
            "`auto` is only supported for `#[derive(Injectable)]`",
        ));
    }

    let ident = &init.args.ident;

    let init_struct_name = &init.init_struct_name;
    let init_struct_def = init.init_struct_def();
    let hidden_init_struct_def = init.hidden_init_struct_def();
    let init_value = init.init_value_with_hook();

    let build = init.build();

    let (impl_gen, ty_gen, where_clause) = init.args.generics.split_for_impl();
    Ok(quote!(
        #init_struct_def

        const _: () = {
            #hidden_init_struct_def

            impl<#impl_gen> dijavu::Initializable for #ident<#ty_gen> #where_clause {
                type Init = #init_struct_name<#ty_gen>;

                async fn build(
                    init: <Self as dijavu::Initializable>::Init,
                    builder: &mut dijavu::InjectorBuilder,
                ) -> dijavu::Result<Self> {
                    Ok(#build)
                }
            }

            impl<#impl_gen> dijavu::NewInitValue for #ident<#ty_gen> #where_clause {
                type Error = dijavu::Error;

                async fn new_init(
                    injector: &mut dijavu::InitInjector
                ) -> dijavu::Result<<Self as dijavu::Initializable>::Init, Self::Error> {
                    Ok(#init_value)
                }
            }
        };
    ))
}
