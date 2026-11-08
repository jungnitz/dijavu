//! `InjectMap` references all implementations that were inserted by key during the prepare step.

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::atomic::{AtomicU8, Ordering},
};

use dijavu::{
    Builder, Error, FieldPrepareContext, Injectable, InjectableField, Injector, PrepareContext,
    fields::{Depend, Inject, InjectMap, InjectToken},
    state::{Declare, Preparing},
};
use futures::executor::block_on;

fn build<I: Injectable>() -> Result<Injector, Error> {
    let mut builder = Builder::new();
    builder.add::<I>();
    block_on(builder.build())
}

/// The sorted pairs of keys and names.
fn entries(services: &InjectMap<String, dyn Service>) -> Vec<(&str, &str)> {
    let mut entries: Vec<_> = services
        .iter()
        .map(|(key, service)| (key.as_str(), service.name()))
        .collect();
    entries.sort();
    entries
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
struct InsertsBoth {
    services: InjectMap<String, dyn Service>,
    first: Inject<First>,
    second: Inject<Second>,
}

impl InsertsBoth {
    async fn on_prepare(
        state: &mut InsertsBothState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        assert!(!state.services.insert("a".to_owned(), state.second));
        assert!(!state.services.insert("b".to_owned(), state.first));
        assert!(state.services.contains_key("a"));
        assert_eq!(state.services.len(), 2);
        Ok(())
    }
}

#[test]
fn looks_up_by_key() {
    let injector = build::<InsertsBoth>().unwrap();
    let services = &injector.get::<InsertsBoth>().services;
    assert_eq!(services.len(), 2);
    assert!(ptr::addr_eq(
        services.get("a").unwrap(),
        injector.get::<Second>()
    ));
    assert!(ptr::addr_eq(
        services.get("b").unwrap(),
        injector.get::<First>()
    ));
    assert!(services.get("c").is_none());
    assert!(services.contains_key("a"));
    assert!(!services.contains_key("c"));
}

#[test]
fn iterates_entries_keys_and_values() {
    let injector = build::<InsertsBoth>().unwrap();
    let services = &injector.get::<InsertsBoth>().services;
    assert_eq!(entries(services), [("a", "second"), ("b", "first")]);
    let mut keys: Vec<_> = services.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, ["a", "b"]);
    let mut names: Vec<_> = services.values().map(Service::name).collect();
    names.sort();
    assert_eq!(names, ["first", "second"]);
    let mut count = 0;
    for _ in services {
        count += 1;
    }
    assert_eq!(count, services.iter().len());
}

#[derive(Injectable)]
struct NoneInserted {
    services: InjectMap<String, dyn Service>,
}

#[test]
fn may_be_empty() {
    let injector = build::<NoneInserted>().unwrap();
    let services = &injector.get::<NoneInserted>().services;
    assert!(services.is_empty());
    assert_eq!(services.values().next().map(Service::name), None);
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct InsertsTwice {
    services: InjectMap<String, dyn Service>,
    first: Inject<First>,
    second: Inject<Second>,
}

impl InsertsTwice {
    async fn on_prepare(
        state: &mut InsertsTwiceState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.services.insert("a".to_owned(), state.first);
        assert!(state.services.insert("a".to_owned(), state.second));
        state.services.insert("b".to_owned(), state.first);
        state.services.insert("c".to_owned(), state.first);
        assert!(state.services.remove("c"));
        assert!(!state.services.remove("c"));
        Ok(())
    }
}

#[test]
fn inserting_again_replaces_and_removing_removes() {
    let injector = build::<InsertsTwice>().unwrap();
    assert_eq!(
        entries(&injector.get::<InsertsTwice>().services),
        [("a", "second"), ("b", "first")]
    );
}

#[derive(Injectable)]
struct Collects {
    services: InjectMap<String, dyn Service>,
}

impl CollectsState<Preparing<'_>> {
    fn add_service<T: Injectable + Service>(&mut self, key: &str, token: InjectToken<T>) {
        self.services.insert(key.to_owned(), token);
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
        ctx.get(&mut state.collects).add_service("x", state.first);
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
        ctx.get(&mut state.collects).add_service("y", state.second);
        Ok(())
    }
}

#[test]
fn other_injectables_can_insert_implementations() {
    let mut builder = Builder::new();
    builder.add::<ContributesFirst>().add::<ContributesSecond>();
    let injector = block_on(builder.build()).unwrap();
    assert_eq!(
        entries(&injector.get::<Collects>().services),
        [("x", "first"), ("y", "second")]
    );
}

/// Inserts itself as an implementation, which forms a cycle.
#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct InsertsItself {
    services: InjectMap<String, dyn Service>,
    first: Inject<First>,
}

impl InsertsItself {
    async fn on_prepare(
        state: &mut InsertsItselfState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.services.insert("self".to_owned(), ctx.inject_token());
        state.services.insert("first".to_owned(), state.first);
        Ok(())
    }
}

impl Service for InsertsItself {
    fn name(&self) -> &str {
        "itself"
    }
}

#[test]
fn injectable_can_insert_itself() {
    let injector = build::<InsertsItself>().unwrap();
    let this = injector.get::<InsertsItself>();
    assert!(ptr::addr_eq(this.services.get("self").unwrap(), this));
    assert_eq!(
        entries(&this.services),
        [("first", "first"), ("self", "itself")]
    );
}

/// 0: not dropped, 1: accessing panicked, 2: accessing succeeded
static DROP_ACCESS: AtomicU8 = AtomicU8::new(0);

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct AccessesOnDrop {
    services: InjectMap<String, dyn Service>,
    first: Inject<First>,
}

impl AccessesOnDrop {
    async fn on_prepare(
        state: &mut AccessesOnDropState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.services.insert("first".to_owned(), state.first);
        Ok(())
    }
}

impl Drop for AccessesOnDrop {
    fn drop(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = self.services.get("first").map(Service::name);
        }));
        DROP_ACCESS.store(if result.is_err() { 1 } else { 2 }, Ordering::SeqCst);
    }
}

#[test]
fn references_are_disabled_before_drop() {
    let injector = build::<AccessesOnDrop>().unwrap();
    assert_eq!(
        entries(&injector.get::<AccessesOnDrop>().services),
        [("first", "first")]
    );
    drop(injector);
    assert_eq!(DROP_ACCESS.load(Ordering::SeqCst), 1);
}

/// A reusable group of fields, which works the same as a field of an injectable.
#[derive(InjectableField)]
#[injectable(prepare = Self::on_prepare)]
struct Group {
    services: InjectMap<String, dyn Service>,
    first: Inject<First>,
}

impl Group {
    async fn on_prepare(
        state: &mut GroupState<Declare>,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<(), Error> {
        state.services.insert("first".to_owned(), state.first);
        Ok(())
    }
}

#[derive(Injectable)]
struct UsesGroup {
    group: Group,
}

#[test]
fn inject_map_in_a_reusable_field() {
    let injector = build::<UsesGroup>().unwrap();
    assert_eq!(
        entries(&injector.get::<UsesGroup>().group.services),
        [("first", "first")]
    );
}
