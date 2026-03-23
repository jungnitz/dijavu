use dijavu::{InitAppContainer, Result, Value};
use dijavu_macros::InitInjectable;

#[derive(InitInjectable)]
pub struct Thing {
    #[inject(init)]
    conf: Value<String>,
}

#[test]
fn simple() -> Result<()> {
    let mut container = InitAppContainer::default();
    let thing = container.get::<Thing>()?;
    thing.conf = String::from("hello world!");

    let (container, _) = container.build()?;
    assert_eq!(&*container.get::<Thing>()?.conf, "hello world!");
    Ok(())
}

#[test]
fn no_init_error() -> Result<()> {
    let container = InitAppContainer::default();
    let (container, _) = container.build()?;
    assert!(container.get::<Thing>().is_err());
    Ok(())
}

#[test]
fn auto_init() -> Result<()> {
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
    let (container, _) = container.build()?;
    assert_eq!(&*container.get::<AnotherThing>()?.0.0, "crazy");

    Ok(())
}

#[test]
fn same_type_values() -> Result<()> {
    #[derive(InitInjectable)]
    pub struct AnotherThing(#[inject(init)] Value<String>);

    let mut container = InitAppContainer::default();

    let thing = container.get::<Thing>()?;
    thing.conf = "hello world!".to_owned();

    let another_thing = container.get::<AnotherThing>()?;
    another_thing.0 = String::from("another thing!");

    let (container, _) = container.build()?;

    assert_eq!(&*container.get::<Thing>()?.conf, "hello world!");
    assert_eq!(&*container.get::<AnotherThing>()?.0, "another thing!");
    Ok(())
}
