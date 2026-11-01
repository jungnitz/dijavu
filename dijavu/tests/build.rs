//! Building an injector: references between injectables, bindings and build errors.

use std::{
    ops::Deref,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::atomic::{AtomicU8, Ordering},
};

use dijavu::{
    Builder, DeclareContext, Error, Injectable, Injector,
    error::{BindingConflictError, ErrorKind},
    fields::{Depend, Inject, InjectToken},
    state::Declare,
};
use futures::executor::block_on;

fn build<I: Injectable>() -> Injector {
    let mut builder = Builder::new();
    builder.add::<I>();
    block_on(builder.build()).unwrap()
}

#[derive(Injectable)]
struct Ping {
    pong: Inject<Pong>,
}

#[derive(Injectable)]
struct Pong {
    ping: Inject<Ping>,
}

#[derive(Injectable)]
struct DependsOnPing {
    ping: Depend<Ping>,
}

#[test]
fn cyclic_and_hard_references() {
    let injector = build::<DependsOnPing>();
    let ping = injector.get::<Ping>();
    let pong = injector.get::<Pong>();
    assert!(ptr::eq(&*ping.pong, pong));
    assert!(ptr::eq(&*pong.ping, ping));
    assert!(ptr::eq(&*injector.get::<DependsOnPing>().ping, ping));
}

/// 0: not dropped, 1: dereferencing panicked, 2: dereferencing succeeded
static DROP_DEREF: AtomicU8 = AtomicU8::new(0);

#[derive(Injectable)]
struct DerefsOnDrop {
    peer: Inject<DerefsOnDropPeer>,
}

impl Drop for DerefsOnDrop {
    fn drop(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = self.peer.deref();
        }));
        DROP_DEREF.store(if result.is_err() { 1 } else { 2 }, Ordering::SeqCst);
    }
}

#[derive(Injectable)]
struct DerefsOnDropPeer {
    late: Inject<DerefsOnDrop>,
}

#[test]
fn references_are_disabled_before_drop() {
    let injector = build::<DerefsOnDropPeer>();
    let peer = injector.get::<DerefsOnDropPeer>();
    assert!(ptr::eq(&*peer.late.peer, peer));
    drop(injector);
    assert_eq!(DROP_DEREF.load(Ordering::SeqCst), 1);
}

#[derive(Injectable)]
struct Unvisited {
    #[visitable(skip)]
    ping: Inject<Ping>,
}

#[test]
#[should_panic(expected = "is not valid")]
fn unvisited_reference_panics() {
    let injector = build::<Unvisited>();
    let _ = injector.get::<Unvisited>().ping.deref();
}

#[test]
#[should_panic(expected = "`()` is not part of this injector")]
fn getting_missing_injectable_names_it() {
    let injector = block_on(Builder::new().build()).unwrap();
    injector.get::<()>();
}

trait Service: Send + Sync {}
dijavu::bindable!(Service);

impl Service for Ping {}
impl Service for Pong {}

#[derive(Injectable)]
struct InjectsService {
    service: Inject<dyn Service>,
}

#[test]
fn bind_via_builder() {
    let mut builder = Builder::new();
    builder
        .add::<InjectsService>()
        .bind::<dyn Service, Ping>()
        .unwrap();
    let injector = block_on(builder.build()).unwrap();
    let service = &injector.get::<InjectsService>().service;
    assert!(ptr::addr_eq(&**service, injector.get::<Ping>()));
}

/// Binds `dyn Service` to `Pong` when declared.
#[derive(Injectable)]
#[injectable(declare = Self::bind_pong)]
struct BindsPong;

impl BindsPong {
    fn bind_pong(
        _state: &mut BindsPongState<Declare>,
        mut ctx: DeclareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.bind::<dyn Service, Pong>()?;
        Ok(())
    }
}

#[derive(Injectable)]
struct DependsOnService {
    service: Depend<dyn Service>,
}

#[test]
fn bind_via_injectable() {
    // `DependsOnService` is declared before `BindsPong`, so the binding does not exist yet when
    // `DependsOnService` is declared
    let mut builder = Builder::new();
    builder.add::<BindsPong>().add::<DependsOnService>();
    let injector = block_on(builder.build()).unwrap();
    let service = &injector.get::<DependsOnService>().service;
    assert!(ptr::addr_eq(&**service, injector.get::<Pong>()));
}

#[derive(Injectable)]
struct BoundToItself {
    this: Inject<dyn Service>,
}

impl Service for BoundToItself {}

#[test]
fn bind_to_self() {
    let mut builder = Builder::new();
    builder.bind::<dyn Service, BoundToItself>().unwrap();
    let injector = block_on(builder.build()).unwrap();
    let this = injector.get::<BoundToItself>();
    assert!(ptr::addr_eq(&*this.this, this));
}

#[derive(Injectable)]
struct CyclicService {
    dependent: Depend<DependsOnService>,
}

impl Service for CyclicService {}

#[test]
fn bound_hard_dependencies_are_checked_for_cycles() {
    // `DependsOnService` -> `dyn Service` = `CyclicService` -> `DependsOnService`
    let mut builder = Builder::new();
    builder.bind::<dyn Service, CyclicService>().unwrap();
    let error = block_on(builder.build()).err().unwrap();
    assert!(matches!(error.kind(), ErrorKind::Cycle(_)), "{error}");
}

#[test]
fn unbound_target_fails() {
    let mut builder = Builder::new();
    builder.add::<InjectsService>();
    let error = block_on(builder.build()).err().unwrap();
    assert!(matches!(error.kind(), ErrorKind::Unbound(_)), "{error}");
}

#[test]
fn repeating_a_binding_is_allowed() {
    let mut builder = Builder::new();
    builder
        .bind::<dyn Service, Ping>()
        .unwrap()
        .bind::<dyn Service, Ping>()
        .unwrap();
    assert!(block_on(builder.build()).is_ok());
}

#[test]
fn conflicting_binding_via_builder_fails_immediately() {
    let mut builder = Builder::new();
    builder.bind::<dyn Service, Ping>().unwrap();
    let _error: BindingConflictError = builder.bind::<dyn Service, Pong>().err().unwrap();
}

#[test]
fn conflicting_binding_via_injectable_fails_the_build() {
    let mut builder = Builder::new();
    builder
        .bind::<dyn Service, Ping>()
        .unwrap()
        .add::<BindsPong>();
    let error = block_on(builder.build()).err().unwrap();
    assert!(
        matches!(error.kind(), ErrorKind::BindingConflict(_)),
        "{error}"
    );
}

#[test]
fn rejected_binding_does_not_add_injectable() {
    let mut builder = Builder::new();
    builder.bind::<dyn Service, Ping>().unwrap();
    let _error: BindingConflictError = builder.bind::<dyn Service, BoundToItself>().err().unwrap();
    let injector = block_on(builder.build()).unwrap();
    assert!(injector.get_optional::<BoundToItself>().is_none());
}

// The following only need to compile.

fn _assert_send<T: Send>(_: T) {}

fn _build_is_send(builder: Builder) {
    _assert_send(builder.build());
}

fn _assert_copy<T: Copy>() {}

fn _tokens_are_copy() {
    _assert_copy::<InjectToken<Ping>>();
    _assert_copy::<InjectToken<dyn Service>>();
}
