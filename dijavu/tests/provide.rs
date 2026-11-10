//! `InjectAll` references all providers in the build, `InjectVec` only the added implementations.

use dijavu::{
    Builder, DeclareContext, Error, Injectable, Injector, PrepareContext,
    fields::{Depend, DependToken, Inject, InjectAll, InjectToken, InjectVec},
    state::Declare,
};
use futures::executor::block_on;

fn build(add: impl FnOnce(&mut Builder)) -> Result<Injector, Error> {
    let mut builder = Builder::new();
    add(&mut builder);
    block_on(builder.build())
}

fn names<'a>(services: impl IntoIterator<Item = &'a (dyn Service + 'static)>) -> Vec<&'a str> {
    services.into_iter().map(Service::name).collect()
}

trait Service: Send + Sync {
    fn name(&self) -> &str;
}
dijavu::unsizable!(Service);

trait Other: Send + Sync {}
dijavu::unsizable!(Other);

#[derive(Injectable)]
#[injectable(provide(dyn Service, dyn Other))]
struct First;

impl Service for First {
    fn name(&self) -> &str {
        "first"
    }
}

impl Other for First {}

#[derive(Injectable)]
#[injectable(provide(dyn Service), provide(dyn Service))]
struct Second;

impl Service for Second {
    fn name(&self) -> &str {
        "second"
    }
}

#[derive(Injectable)]
struct Added;

impl Service for Added {
    fn name(&self) -> &str {
        "added"
    }
}

/// Collects the providers, and adds `Added` and the provider `First` to an `InjectVec`.
#[derive(Injectable)]
#[injectable(prepare = on_prepare)]
struct Services {
    services: InjectAll<dyn Service>,
    others: InjectAll<dyn Other>,
    added: InjectVec<dyn Service>,
    first: Inject<First>,
    added_service: Inject<Added>,
}

impl ServicesState<Declare> {
    #[expect(clippy::unused_async, reason = "hooks are async")]
    async fn on_prepare(&mut self, _ctx: PrepareContext<'_, Services>) {
        self.added.add(self.added_service);
        self.added.add(self.first);
    }
}

#[test]
fn inject_all_references_the_providers() {
    let injector = build(|builder| {
        builder.add::<Services>().add::<First>().add::<Second>();
    })
    .unwrap();
    let services = injector.get::<Services>();
    // in the order in which they were added to the build, each provider once
    assert_eq!(names(&services.services), ["first", "second"]);
    assert_eq!(services.others.len(), 1);
}

#[test]
fn inject_vec_ignores_providers() {
    let injector = build(|builder| {
        builder.add::<Services>().add::<Second>();
    })
    .unwrap();
    assert_eq!(names(&injector.get::<Services>().added), ["added", "first"]);
}

#[test]
fn providers_added_via_fields_are_referenced() {
    let injector = build(|builder| {
        builder.add::<Services>().add::<Second>();
    })
    .unwrap();
    // `First` is part of the build via the `Inject` field, which is declared after `Second` was
    // added
    assert_eq!(
        names(&injector.get::<Services>().services),
        ["second", "first"]
    );
}

#[derive(Injectable)]
struct OnlyProviders {
    services: InjectAll<dyn Service>,
}

#[test]
fn providers_outside_the_build_are_not_referenced() {
    let injector = build(|builder| {
        builder.add::<OnlyProviders>().add::<Second>();
    })
    .unwrap();
    assert_eq!(names(&injector.get::<OnlyProviders>().services), ["second"]);
}

#[test]
fn builder_provides() {
    let injector = build(|builder| {
        builder
            .add::<OnlyProviders>()
            .provide::<dyn Service, Added>();
    })
    .unwrap();
    assert_eq!(names(&injector.get::<OnlyProviders>().services), ["added"]);
}

/// Adds its dependencies via the declare context instead of fields.
#[derive(Injectable)]
#[injectable(declare = add_services)]
struct DeclaresServices {
    services: InjectAll<dyn Service>,
}

impl DeclaresServicesState<Declare> {
    #[expect(clippy::unnecessary_wraps, reason = "hooks may fail")]
    fn add_services(&mut self, mut ctx: DeclareContext<'_, DeclaresServices>) -> Result<(), Error> {
        let _: InjectToken<First> = ctx.inject::<First>();
        let _: DependToken<Second> = ctx.depend::<Second>();
        ctx.provide::<dyn Service, Added>();
        Ok(())
    }
}

#[test]
fn declare_context_adds_injectables() {
    let injector = build(|builder| {
        builder.add::<DeclaresServices>();
    })
    .unwrap();
    assert_eq!(
        names(&injector.get::<DeclaresServices>().services),
        ["first", "second", "added"]
    );
}

/// Depends on `Second` via the declare context, so `Second` is assembled first.
#[derive(Injectable)]
#[injectable(declare = depend_on_second)]
struct DependsViaContext {
    second: Depend<Second>,
}

impl DependsViaContextState<Declare> {
    fn depend_on_second(&mut self, mut ctx: DeclareContext<'_, DependsViaContext>) {
        ctx.depend::<Second>();
    }
}

#[test]
fn declare_context_depends() {
    let injector = build(|builder| {
        builder.add::<DependsViaContext>();
    })
    .unwrap();
    assert_eq!(injector.get::<DependsViaContext>().second.name(), "second");
}
