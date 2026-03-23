use dijavu::{AppContainer, AppContainerBuilder, DataKey, Injectable, Result};
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

#[test]
fn from_data() -> Result<()> {
    pub struct Dependency(&'static str);

    struct DependencyKey;
    impl DataKey for DependencyKey {
        type Item = String;
    }

    impl Injectable for Dependency {
        type Error = Infallible;
        fn get(container: AppContainer) -> std::result::Result<Self, Self::Error> {
            Ok(Dependency(container.data().get::<DependencyKey>().unwrap()))
        }
    }

    let mut builder = AppContainerBuilder::default();
    builder.insert_app_data::<DependencyKey>("test".to_owned())?;
    let (container, _) = builder.build();
    assert_eq!(container.get::<Dependency>()?.0, "test".to_owned());
    Ok(())
}
