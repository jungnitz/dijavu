//! Hooks can be sync or async, may return `()` if they cannot fail, and can be given by name.

use std::sync::Mutex;

use dijavu::{
    AssembleContext, Builder, DeclareContext, Error, FieldAssembleContext, FieldPrepareContext,
    Injectable, InjectableField, Injector, PrepareContext, StartContext,
    fields::Value,
    state::{Declare, Prepare, Preparing, Start},
};
use futures::executor::block_on;

fn build<I: Injectable>() -> Result<Injector, Error> {
    let mut builder = Builder::new();
    builder.add::<I>();
    block_on(builder.build())
}

/// Records the hooks in the order in which they ran.
#[derive(Injectable)]
#[injectable(
    declare = on_declare,
    prepare_sync = on_prepare,
    prepared_sync = on_prepared,
    assemble_sync = on_assemble,
    start_sync = on_start,
)]
struct Sync {
    log: Value<Vec<&'static str>>,
    started: Value<Mutex<bool>>,
    field: SyncField,
}

impl SyncState<Declare> {
    fn on_declare(&mut self, _ctx: DeclareContext<'_, Sync>) {
        self.log.push("declare");
    }

    fn on_prepare(&mut self, _ctx: PrepareContext<'_, Sync>) {
        self.log.push("prepare");
    }
}

impl SyncState<Preparing<'_>> {
    fn on_prepared(&mut self) {
        self.log.push("prepared");
    }
}

impl SyncState<Prepare> {
    fn on_assemble(&mut self, _ctx: AssembleContext<'_, Sync>) {
        self.log.push("assemble");
    }
}

impl Sync {
    fn on_start(&self, _state: SyncState<Start>, _ctx: StartContext<'_, Self>) {
        *self.started.lock().unwrap() = true;
    }
}

#[derive(InjectableField)]
#[injectable(prepare_sync = on_prepare, prepared_sync = on_prepared, assemble_sync = on_assemble)]
struct SyncField {
    log: Value<Vec<&'static str>>,
}

impl SyncFieldState<Declare> {
    fn on_prepare(&mut self, _ctx: FieldPrepareContext<'_>) {
        self.log.push("prepare");
    }
}

impl SyncFieldState<Prepare> {
    fn on_prepared(&mut self, _ctx: FieldPrepareContext<'_>) {
        self.log.push("prepared");
    }

    fn on_assemble(&mut self, _ctx: FieldAssembleContext<'_>) {
        self.log.push("assemble");
    }
}

#[test]
fn sync_hooks_by_name() {
    let injector = build::<Sync>().unwrap();
    let sync = injector.get::<Sync>();
    assert_eq!(*sync.log, ["declare", "prepare", "prepared", "assemble"]);
    assert_eq!(*sync.field.log, ["prepare", "prepared", "assemble"]);
    assert!(*sync.started.lock().unwrap());
}

/// Async hooks returning `()` and a sync hook returning a `Result`.
#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare, prepared = on_prepared, start_sync = Self::on_start)]
struct Mixed {
    log: Value<Vec<&'static str>>,
}

impl Mixed {
    #[expect(clippy::unused_async, reason = "tests async hooks")]
    async fn on_prepare(state: &mut MixedState<Declare>, _ctx: PrepareContext<'_, Self>) {
        state.log.push("prepare");
    }

    fn on_start(
        &self,
        _state: MixedState<Start>,
        _ctx: StartContext<'_, Self>,
    ) -> Result<(), Error> {
        assert_eq!(*self.log, ["prepare", "prepared"]);
        Ok(())
    }
}

impl MixedState<Preparing<'_>> {
    #[expect(clippy::unused_async, reason = "tests async hooks")]
    async fn on_prepared(&mut self) {
        self.log.push("prepared");
    }
}

#[test]
fn hooks_returning_unit_or_result() {
    let injector = build::<Mixed>().unwrap();
    assert_eq!(*injector.get::<Mixed>().log, ["prepare", "prepared"]);
}

#[derive(Debug)]
struct Failure;

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("sync hook failed")
    }
}

impl std::error::Error for Failure {}

impl From<Failure> for Error {
    fn from(failure: Failure) -> Self {
        Error::custom(Box::new(failure))
    }
}

#[derive(Injectable)]
#[injectable(prepared_sync = fail)]
struct Failing {
    value: Value<u32>,
}

impl FailingState<Preparing<'_>> {
    fn fail(&mut self) -> Result<(), Failure> {
        Err(Failure)
    }
}

#[test]
fn sync_hook_errors_fail_the_build() {
    let error = build::<Failing>().err().unwrap();
    assert!(error.into_custom().unwrap().is::<Failure>());
}
