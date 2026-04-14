use std::sync::atomic::{AtomicBool, Ordering};

use dijavu::{AppContainer, Data, InitAppContainer, Result, StartValue, Value};
use dijavu_macros::InitInjectable;

#[derive(InitInjectable)]
pub struct Thing {
    #[inject(init)]
    conf: Value<String>,
}

#[tokio::test]
async fn simple() -> Result<()> {
    let mut container = InitAppContainer::default();
    let thing = container.get::<Thing>()?;
    thing.conf = String::from("hello world!");

    let container = container.build().await?;
    assert_eq!(&*container.get::<Thing>()?.conf, "hello world!");
    Ok(())
}

#[tokio::test]
async fn no_init_error() -> Result<()> {
    let container = InitAppContainer::default();
    let container = container.build().await?;
    assert!(container.get::<Thing>().is_err());
    Ok(())
}

#[tokio::test]
async fn auto_init() -> Result<()> {
    struct CrazyDefault(String);
    impl Default for CrazyDefault {
        fn default() -> Self {
            Self(String::from("crazy"))
        }
    }

    #[derive(InitInjectable)]
    #[inject(init(auto))]
    pub struct AnotherThing(#[inject(init)] Value<CrazyDefault>);

    let container = InitAppContainer::default();
    let container = container.build().await?;
    assert_eq!(&*container.get::<AnotherThing>()?.0.0, "crazy");

    Ok(())
}

#[tokio::test]
async fn same_type_values() -> Result<()> {
    #[derive(InitInjectable)]
    pub struct AnotherThing(#[inject(init)] Value<String>);

    let mut container = InitAppContainer::default();

    let thing = container.get::<Thing>()?;
    thing.conf = "hello world!".to_owned();

    let another_thing = container.get::<AnotherThing>()?;
    another_thing.0 = String::from("another thing!");

    let container = container.build().await?;

    assert_eq!(&*container.get::<Thing>()?.conf, "hello world!");
    assert_eq!(&*container.get::<AnotherThing>()?.0, "another thing!");
    Ok(())
}

#[tokio::test]
async fn start_value() -> Result<()> {
    #[derive(InitInjectable)]
    pub struct Thing(#[inject(init)] StartValue<String>);

    let mut container = InitAppContainer::default();
    container.get::<Thing>()?.0 = "hello!".to_owned();
    container.on_start(|_, start_data| {
        assert_eq!(
            StartValue::<String>::remove_from_start_data(start_data).as_deref(),
            Some("hello!")
        );
        Ok(())
    });
    container.build().await?;
    Ok(())
}

#[tokio::test]
async fn macro_hooks() -> Result<()> {
    static ON_CONSTRUCT: AtomicBool = AtomicBool::new(false);
    static ON_START: AtomicBool = AtomicBool::new(false);
    static ON_START_ASYNC: AtomicBool = AtomicBool::new(false);

    fn get_static_states() -> (bool, bool) {
        let on_construct = ON_CONSTRUCT.load(Ordering::SeqCst);
        let on_start = ON_START.load(Ordering::SeqCst);
        let on_start_async = ON_START_ASYNC.load(Ordering::SeqCst);
        assert_eq!(on_start, on_start_async);
        (on_construct, on_start)
    }

    #[derive(InitInjectable)]
    #[inject(init(on_construct = |_init: &mut InitAppContainer| {
        ON_CONSTRUCT.store(true, Ordering::SeqCst);
        Ok(())
    }, on_build = |value: &mut TInit, _, _| {
        value.0 = 42;
        Ok(())
    }, on_build_async = async |value: &mut TInit, _, _| {
        value.1 = 24;
        Ok(())
    }, on_start = |_: AppContainer, _: &mut Data| {
        ON_START.store(true, Ordering::SeqCst);
        Ok(())
    }, on_start_async = async |_: AppContainer, _: &mut Data| {
        ON_START_ASYNC.store(true, Ordering::SeqCst);
        Ok(())
    }))]
    pub struct T(#[inject(init)] Value<i32>, #[inject(init)] Value<i32>);

    ON_START.store(false, Ordering::SeqCst);
    ON_START_ASYNC.store(false, Ordering::SeqCst);

    let mut container = InitAppContainer::default();
    assert_eq!(get_static_states(), (false, false));

    container.get::<T>()?;
    assert_eq!(get_static_states(), (true, false));

    let container = container.build().await?;
    assert_eq!(get_static_states(), (true, true));
    assert_eq!(*container.get::<T>().unwrap().0, 42);
    assert_eq!(*container.get::<T>().unwrap().1, 24);

    Ok(())
}
