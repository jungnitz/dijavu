use dijavu::{AppContainer, AppContainerBuilder, DataKey, Injectable, Result, RuntimeData};
use std::convert::Infallible;

#[test]
fn simple() -> Result<()> {
    #[derive(Injectable)]
    struct Inject {
        _thing: (),
    }
    let container = AppContainer::empty();
    let _value: Inject = container.get()?;
    Ok(())
}

#[tokio::test]
async fn from_data() -> Result<()> {
    pub struct Dependency(&'static str);

    struct DependencyKey;
    impl DataKey for DependencyKey {
        type Item = String;
    }

    impl Injectable for Dependency {
        type Error = Infallible;
        fn get(data: &RuntimeData) -> std::result::Result<Self, Self::Error> {
            Ok(Dependency(data.get::<DependencyKey>().unwrap()))
        }
    }

    let mut builder = AppContainerBuilder::default();
    builder.insert_app_data::<DependencyKey>("test".to_owned())?;
    let container = builder.build().await?;
    assert_eq!(container.get::<Dependency>()?.0, "test".to_owned());
    Ok(())
}
