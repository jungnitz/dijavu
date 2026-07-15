use crate::utils::{GenericsHelper, WithSpan};
use darling::util::Flag;
use darling::{FromDeriveInput, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Ident, LitBool, Visibility};

#[expect(clippy::needless_continue, reason = "emitted by FromDeriveInput")]
mod derive_args {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(FromDeriveInput)]
    #[darling(attributes(inject))]
    pub struct DeriveArgs {
        pub vis: Visibility,
        pub ident: Ident,

        pub generics: GenericsHelper,

        #[darling(default)]
        pub init: InitArgs,
        #[darling(default)]
        pub build: BuildArgs,
    }
}
pub use derive_args::DeriveArgs;

#[derive(FromMeta, Default)]
pub struct InitArgs {
    pub hide: Flag,
    pub auto: Flag,
    pub manual: Flag,
    pub hook: Option<Expr>,

    #[darling(default)]
    pub data: WithSpan<Option<InitDataArgs>>,
}

impl InitArgs {
    fn check_for_initializable(&self) -> syn::Result<()> {
        if self.auto.is_present() {
            return Err(syn::Error::new(
                self.auto.span(),
                "`auto` is not supported for `Initializable`",
            ));
        }
        Ok(())
    }
    fn check_for_injectable(&self) -> syn::Result<()> {
        if self.manual.is_present() {
            return Err(syn::Error::new(
                self.manual.span(),
                "`manual` is not supported for `Injectable`",
            ));
        }
        Ok(())
    }
}

impl InitArgs {}

#[derive(FromMeta)]
pub struct InitDataArgs {
    pub hide: Option<LitBool>,
}

#[derive(FromMeta, Default)]
pub struct BuildArgs {
    pub hook: Option<Expr>,
}

impl DeriveArgs {
    pub fn check_for_initializable(&self) -> syn::Result<()> {
        self.init.check_for_initializable()?;
        Ok(())
    }

    pub fn check_for_injectable(&self) -> syn::Result<()> {
        self.init.check_for_injectable()?;
        Ok(())
    }

    pub fn run_build_hook(&self) -> TokenStream {
        if let Some(build) = &self.build.hook {
            quote!(let _: () = (#build)(&mut init, builder).await?;)
        } else {
            quote!()
        }
    }
}
