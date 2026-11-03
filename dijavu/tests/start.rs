use std::sync::{
    Mutex,
    atomic::{AtomicU32, Ordering},
};

use dijavu::{
    Builder, Error, Injectable, InjectableField, Injector, StartContext,
    fields::{Depend, Inject, Value},
    state::Start,
};
use futures::executor::block_on;

fn build<I: Injectable>() -> Result<Injector, Error> {
    let mut builder = Builder::new();
    builder.add::<I>();
    block_on(builder.build())
}

#[derive(Injectable)]
#[injectable(start = Self::on_start)]
struct Counter {
    started: Value<AtomicU32>,
}

impl Counter {
    async fn on_start(
        &self,
        _state: CounterState<Start>,
        _ctx: StartContext<'_, Self>,
    ) -> Result<(), std::convert::Infallible> {
        self.started.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[test]
fn start_hook_runs_once() {
    let injector = build::<Counter>().unwrap();
    assert_eq!(injector.get::<Counter>().started.load(Ordering::Relaxed), 1);
}

#[derive(Injectable)]
struct WithoutStart {
    counter: Depend<Counter>,
}

#[test]
fn start_is_optional() {
    let injector = build::<WithoutStart>().unwrap();
    let counter = &injector.get::<WithoutStart>().counter;
    assert_eq!(counter.started.load(Ordering::Relaxed), 1);
}

#[derive(Injectable)]
#[injectable(start = Self::on_start)]
struct Log {
    entries: Value<Mutex<Vec<&'static str>>>,
}

impl Log {
    async fn on_start(
        &self,
        _state: LogState<Start>,
        _ctx: StartContext<'_, Self>,
    ) -> Result<(), std::convert::Infallible> {
        self.entries.lock().unwrap().push("log");
        Ok(())
    }
}

#[derive(Injectable)]
#[injectable(start = Self::on_start)]
struct Dependent {
    log: Depend<Log>,
    // cyclic soft references must not prevent starting
    this: Inject<Dependent>,
}

impl Dependent {
    async fn on_start(
        &self,
        _state: DependentState<Start>,
        ctx: StartContext<'_, Self>,
    ) -> Result<(), std::convert::Infallible> {
        assert!(std::ptr::eq(&*self.this, ctx.get::<Self>()));
        self.log.entries.lock().unwrap().push("dependent");
        Ok(())
    }
}

#[test]
fn dependencies_start_first() {
    let injector = build::<Dependent>().unwrap();
    let entries = injector.get::<Log>().entries.lock().unwrap();
    assert_eq!(*entries, ["log", "dependent"]);
}

#[derive(Injectable)]
#[injectable(start = Self::on_start)]
struct KeepsInjector {
    injector: Value<Mutex<Option<Injector>>>,
}

impl KeepsInjector {
    async fn on_start(
        &self,
        _state: KeepsInjectorState<Start>,
        ctx: StartContext<'_, Self>,
    ) -> Result<(), std::convert::Infallible> {
        // e.g. to move it into a spawned task, the cycle is broken below
        *self.injector.lock().unwrap() = Some(ctx.injector());
        Ok(())
    }
}

#[test]
fn context_provides_injector_clones() {
    let injector = build::<KeepsInjector>().unwrap();
    let clone = injector
        .get::<KeepsInjector>()
        .injector
        .lock()
        .unwrap()
        .take()
        .unwrap();
    drop(injector);
    // the clone keeps the injectables alive
    assert!(
        clone
            .get::<KeepsInjector>()
            .injector
            .lock()
            .unwrap()
            .is_none()
    );
}

#[derive(Debug, thiserror::Error)]
#[error("start failed")]
struct StartError;

impl From<StartError> for Error {
    fn from(error: StartError) -> Self {
        Error::custom(Box::new(error))
    }
}

#[derive(Injectable)]
#[injectable(start = Self::fail)]
struct FailsToStart(Value<()>);

impl FailsToStart {
    async fn fail(
        &self,
        _state: FailsToStartState<Start>,
        _ctx: StartContext<'_, Self>,
    ) -> Result<(), StartError> {
        Err(StartError)
    }
}

#[test]
fn start_error_fails_build() {
    let Err(error) = build::<FailsToStart>() else {
        panic!("build should fail")
    };
    assert!(error.into_custom().unwrap().is::<StartError>());
}

/// A field whose `Start` is its preparation.
#[derive(dijavu::Visitable)]
struct Number;

impl InjectableField for Number {
    type Declaration = u32;
    type Preparation = u32;
    type Start = u32;

    fn declare(_ctx: dijavu::FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(1)
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: dijavu::FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration + 1)
    }

    async fn assemble(
        preparation: Self::Preparation,
        _ctx: dijavu::FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        Ok((Self, preparation))
    }
}

/// A reusable field combining the `Start` of its fields.
#[derive(InjectableField)]
struct Numbers {
    first: Number,
    second: Number,
}

#[derive(Injectable)]
#[injectable(start = Self::on_start)]
struct WithNumbers {
    numbers: Numbers,
    received: Value<Mutex<Vec<u32>>>,
}

impl WithNumbers {
    async fn on_start(
        &self,
        state: WithNumbersState<Start>,
        _ctx: StartContext<'_, Self>,
    ) -> Result<(), std::convert::Infallible> {
        let NumbersState { first, second } = state.numbers;
        self.received.lock().unwrap().extend([first, second]);
        Ok(())
    }
}

#[test]
fn start_contains_the_start_of_each_field() {
    let injector = build::<WithNumbers>().unwrap();
    let received = injector.get::<WithNumbers>().received.lock().unwrap();
    assert_eq!(*received, [2, 2]);
}
