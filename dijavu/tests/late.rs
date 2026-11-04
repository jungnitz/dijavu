use dijavu::{
    Builder, Error, Injectable, Injector, PrepareContext,
    error::ErrorKind,
    fields::{Depend, Late},
    state::{Declare, Preparing},
};
use futures::executor::block_on;

fn build<I: Injectable>() -> Result<Injector, Error> {
    let mut builder = Builder::new();
    builder.add::<I>();
    block_on(builder.build())
}

/// Not `Default`.
struct Connection(&'static str);

#[derive(Injectable)]
#[injectable(prepare = Self::connect)]
struct Database {
    connection: Late<Connection>,
}

impl Database {
    async fn connect(
        state: &mut DatabaseState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.connection = Some(Connection("database"));
        Ok(())
    }
}

#[test]
fn set_by_hook() {
    let injector = build::<Database>().unwrap();
    assert_eq!(injector.get::<Database>().connection.0, "database");
}

#[derive(Injectable)]
struct Unset {
    _connection: Late<Connection>,
}

#[test]
fn unset_fails_build() {
    let Err(error) = build::<Unset>() else {
        panic!("build should fail")
    };
    assert!(matches!(error.kind(), ErrorKind::UnsetLate(_)));
    assert!(error.to_string().contains("Late<late::Connection>"));
}

#[derive(Injectable)]
struct Configured {
    connection: Late<Connection>,
}

impl ConfiguredState<Preparing<'_>> {
    fn set_connection(&mut self, connection: Connection) {
        self.connection = Some(connection);
    }
}

#[derive(Injectable)]
#[injectable(prepare = Self::configure)]
struct Configurer {
    configured: Depend<Configured>,
}

impl Configurer {
    async fn configure(
        state: &mut ConfigurerState<Declare>,
        mut ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        ctx.get(&mut state.configured)
            .set_connection(Connection("configurer"));
        Ok(())
    }
}

#[test]
fn set_by_other_injectable() {
    let injector = build::<Configurer>().unwrap();
    assert_eq!(injector.get::<Configured>().connection.0, "configurer");
}
