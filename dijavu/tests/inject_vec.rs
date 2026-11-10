//! `InjectVec` references all implementations that were added during the build.

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::atomic::{AtomicU8, Ordering},
};

use dijavu::{
    Builder, Error, FieldPrepareContext, Injectable, InjectableField, Injector, PrepareContext,
    fields::{Depend, Inject, InjectToken, InjectVec},
    state::{Declare, Preparing},
};
use futures::executor::block_on;

fn build<I: Injectable>() -> Result<Injector, Error> {
    let mut builder = Builder::new();
    builder.add::<I>();
    block_on(builder.build())
}

fn names(services: &InjectVec<dyn Service>) -> Vec<&str> {
    services.iter().map(Service::name).collect()
}

trait Service: Send + Sync {
    fn name(&self) -> &str;
}
dijavu::unsizable!(Service);

#[derive(Injectable)]
struct First;

impl Service for First {
    fn name(&self) -> &str {
        "first"
    }
}

#[derive(Injectable)]
struct Second;

impl Service for Second {
    fn name(&self) -> &str {
        "second"
    }
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct AddsBoth {
    services: InjectVec<dyn Service>,
    first: Inject<First>,
    second: Inject<Second>,
}

impl AddsBoth {
    async fn on_prepare(
        state: &mut AddsBothState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.services.add(state.second);
        state.services.add(state.first);
        Ok(())
    }
}

#[test]
fn keeps_the_order_of_adding() {
    let injector = build::<AddsBoth>().unwrap();
    let services = &injector.get::<AddsBoth>().services;
    assert_eq!(services.len(), 2);
    assert_eq!(names(services), ["second", "first"]);
    assert!(ptr::addr_eq(
        services.get(0).unwrap(),
        injector.get::<Second>()
    ));
    assert!(ptr::addr_eq(
        services.get(1).unwrap(),
        injector.get::<First>()
    ));
    assert!(services.get(2).is_none());
}

#[test]
fn iterates_by_reference_and_backwards() {
    let injector = build::<AddsBoth>().unwrap();
    let services = &injector.get::<AddsBoth>().services;
    let reversed: Vec<_> = services.iter().rev().map(Service::name).collect();
    assert_eq!(reversed, ["first", "second"]);
    let mut count = 0;
    for _ in services {
        count += 1;
    }
    assert_eq!(count, services.iter().len());
}

#[derive(Injectable)]
struct NoneAdded {
    services: InjectVec<dyn Service>,
}

#[test]
fn may_be_empty() {
    let injector = build::<NoneAdded>().unwrap();
    let services = &injector.get::<NoneAdded>().services;
    assert!(services.is_empty());
    assert_eq!(services.iter().next().map(Service::name), None);
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct AddsTwice {
    services: InjectVec<dyn Service>,
    first: Inject<First>,
}

impl AddsTwice {
    async fn on_prepare(
        state: &mut AddsTwiceState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.services.add(state.first);
        state.services.add(state.first);
        Ok(())
    }
}

#[test]
fn adding_again_references_again() {
    let injector = build::<AddsTwice>().unwrap();
    assert_eq!(
        names(&injector.get::<AddsTwice>().services),
        ["first", "first"]
    );
}

#[derive(Injectable)]
struct Collects {
    services: InjectVec<dyn Service>,
}

impl CollectsState<Preparing<'_>> {
    fn add_service<T: Injectable + Service>(&mut self, token: InjectToken<T>) {
        self.services.add(token);
    }
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct ContributesFirst {
    collects: Depend<Collects>,
    first: Inject<First>,
}

impl ContributesFirst {
    async fn on_prepare(
        state: &mut ContributesFirstState<Declare>,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.get(&mut state.collects).add_service(state.first);
        Ok(())
    }
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct ContributesSecond {
    collects: Depend<Collects>,
    second: Inject<Second>,
}

impl ContributesSecond {
    async fn on_prepare(
        state: &mut ContributesSecondState<Declare>,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.get(&mut state.collects).add_service(state.second);
        Ok(())
    }
}

#[test]
fn other_injectables_can_add_implementations() {
    let mut builder = Builder::new();
    builder.add::<ContributesFirst>().add::<ContributesSecond>();
    let injector = block_on(builder.build()).unwrap();
    let mut names = names(&injector.get::<Collects>().services);
    names.sort();
    assert_eq!(names, ["first", "second"]);
}

/// Adds itself as an implementation, which forms a cycle.
#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct AddsItself {
    services: InjectVec<dyn Service>,
    first: Inject<First>,
}

impl AddsItself {
    async fn on_prepare(
        state: &mut AddsItselfState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.services.add(ctx.inject_token());
        state.services.add(state.first);
        Ok(())
    }
}

impl Service for AddsItself {
    fn name(&self) -> &str {
        "itself"
    }
}

#[test]
fn injectable_can_add_itself() {
    let injector = build::<AddsItself>().unwrap();
    let this = injector.get::<AddsItself>();
    assert!(ptr::addr_eq(this.services.get(0).unwrap(), this));
    assert_eq!(names(&this.services), ["itself", "first"]);
}

/// 0: not dropped, 1: accessing panicked, 2: accessing succeeded
static DROP_ACCESS: AtomicU8 = AtomicU8::new(0);

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct AccessesOnDrop {
    services: InjectVec<dyn Service>,
    first: Inject<First>,
}

impl AccessesOnDrop {
    async fn on_prepare(
        state: &mut AccessesOnDropState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.services.add(state.first);
        Ok(())
    }
}

impl Drop for AccessesOnDrop {
    fn drop(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = self.services.iter().next().map(Service::name);
        }));
        DROP_ACCESS.store(if result.is_err() { 1 } else { 2 }, Ordering::SeqCst);
    }
}

#[test]
fn references_are_disabled_before_drop() {
    let injector = build::<AccessesOnDrop>().unwrap();
    assert_eq!(names(&injector.get::<AccessesOnDrop>().services), ["first"]);
    drop(injector);
    assert_eq!(DROP_ACCESS.load(Ordering::SeqCst), 1);
}

/// A reusable group of fields, which works the same as a field of an injectable.
#[derive(InjectableField)]
#[injectable(prepare = Self::on_prepare)]
struct Group {
    services: InjectVec<dyn Service>,
    first: Inject<First>,
}

impl Group {
    async fn on_prepare(
        state: &mut GroupState<Declare>,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<(), Error> {
        state.services.add(state.first);
        Ok(())
    }
}

#[derive(Injectable)]
struct UsesGroup {
    group: Group,
}

#[test]
fn inject_vec_in_a_reusable_field() {
    let injector = build::<UsesGroup>().unwrap();
    assert_eq!(
        names(&injector.get::<UsesGroup>().group.services),
        ["first"]
    );
}
