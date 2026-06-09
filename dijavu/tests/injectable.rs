use dijavu::initializables::{Initializables, Inject, Value};
use dijavu::{InitInjector, Injectable, InjectorBuilder, Result};
use futures::FutureExt;
use futures::future::BoxFuture;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

#[derive(Injectable)]
pub struct WithStringValue {
    value: Value<String>,
}

#[tokio::test]
async fn simple() -> Result<()> {
    #[derive(Injectable)]
    struct Simple(Inject<WithStringValue>);

    let mut injector = InitInjector::default();

    injector.get::<Simple>().await?;

    let value = injector.get::<WithStringValue>().await?;
    value.value = String::from("hello world!");

    let injector = injector.build().await?;
    assert_eq!(&*injector.get::<WithStringValue>().value, "hello world!");
    assert_eq!(&*injector.get::<Simple>().0.value, "hello world!");
    Ok(())
}

#[tokio::test]
async fn uninitialized_is_none() -> Result<()> {
    let injector = InitInjector::default();
    let injector = injector.build().await?;
    assert!(injector.get_opt::<WithStringValue>().is_none());
    Ok(())
}

#[tokio::test]
async fn auto_init() -> Result<()> {
    struct Foo(String);
    impl Default for Foo {
        fn default() -> Self {
            Self(String::from("foo"))
        }
    }

    #[derive(Injectable)]
    #[inject(init(auto))]
    pub struct FooInjectable(Value<Foo>);

    let injector = InitInjector::default();
    let injector = injector.build().await?;
    assert_eq!(&*injector.get::<FooInjectable>().0.0, "foo");

    Ok(())
}

#[tokio::test]
async fn same_type_values() -> Result<()> {
    #[derive(Injectable)]
    pub struct WithAnotherStringValue(Value<String>);

    let mut injector = InitInjector::default();

    let thing = injector.get::<WithStringValue>().await?;
    thing.value = "hello world!".to_owned();

    let another_thing = injector.get::<WithAnotherStringValue>().await?;
    another_thing.0 = String::from("another thing!");

    let injector = injector.build().await?;

    assert_eq!(&*injector.get::<WithStringValue>().value, "hello world!");
    assert_eq!(
        &*injector.get::<WithAnotherStringValue>().0,
        "another thing!"
    );
    Ok(())
}

#[tokio::test]
async fn macro_hooks() -> Result<()> {
    static ON_INIT: AtomicI32 = AtomicI32::new(0);
    static ON_BUILD: AtomicI32 = AtomicI32::new(0);

    fn get_static_states() -> (i32, i32) {
        let on_init = ON_INIT.load(Ordering::SeqCst);
        let on_build = ON_BUILD.load(Ordering::SeqCst);
        (on_init, on_build)
    }

    #[derive(Injectable)]
    #[inject(init(hook = async |values: &mut ValuesInit, _i: &mut InitInjector| -> Result<()> {
        ON_INIT.fetch_add(1, Ordering::SeqCst);
        values.0 = 1;
        Ok(())
    }), build(hook = async |values: &mut ValuesInit, _builder: &mut InjectorBuilder| -> Result<()> {
        ON_BUILD.fetch_add(1, Ordering::SeqCst);
        values.1 = 2;
        Ok(())
    }))]
    pub struct Values(Value<i32>, Value<i32>);

    let mut injector = InitInjector::default();
    assert_eq!(get_static_states(), (0, 0));

    injector.get::<Values>().await?;
    assert_eq!(get_static_states(), (1, 0));

    assert_eq!(injector.get::<Values>().await?.0, 1);
    assert_eq!(injector.get::<Values>().await?.1, 0);

    let injector = injector.build().await?;
    assert_eq!(get_static_states(), (1, 1));

    assert_eq!(*injector.get_opt::<Values>().unwrap().0, 1);
    assert_eq!(*injector.get_opt::<Values>().unwrap().1, 2);

    Ok(())
}

#[tokio::test]
async fn generic() -> Result<()> {
    #[derive(Injectable)]
    struct GenericInjectable<T: Default + Send + Sync + 'static>(Value<T>);

    let mut injector = InitInjector::default();
    injector.get::<GenericInjectable<String>>().await?.0 = "bar".to_owned();
    injector.get::<GenericInjectable<i32>>().await?.0 = 42;

    let injector = injector.build().await?;
    assert_eq!(*injector.get::<GenericInjectable<String>>().0, "bar");
    assert_eq!(*injector.get::<GenericInjectable<i32>>().0, 42);

    Ok(())
}

#[tokio::test]
async fn self_ref() -> Result<()> {
    #[derive(Injectable)]
    #[inject(init(hook = init_fn))]
    struct SelfRefInjectable(Initializables<SelfRef>);

    struct SelfRef(Inject<SelfRefInjectable>);
    impl From<Inject<SelfRefInjectable>> for SelfRef {
        fn from(value: Inject<SelfRefInjectable>) -> Self {
            Self(value)
        }
    }

    static DID_INIT: AtomicBool = AtomicBool::new(false);
    fn init_fn<'a>(
        init: &'a mut SelfRefInjectableInit,
        injector: &'a mut InitInjector,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            DID_INIT
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .expect("should only run once");
            init.0.add::<Inject<SelfRefInjectable>>(injector).await?;
            Ok(())
        }
        .boxed()
    }

    let mut injector = InitInjector::default();
    injector.get::<SelfRefInjectable>().await?;

    let injector = injector.build().await?;
    let sri = injector.get::<SelfRefInjectable>();
    assert_eq!(
        sri as *const _,
        &*sri.0.iter().next().unwrap().0 as *const _
    );

    Ok(())
}
