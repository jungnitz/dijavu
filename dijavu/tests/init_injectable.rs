use dijavu::{InitAppContainer, Result, Value};
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
