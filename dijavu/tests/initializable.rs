use dijavu::{InitInjector, Initializable, initializables::Value};
use dijavu_macros::Injectable;

#[derive(Initializable)]
struct Test<T: Default + Send + Sync + 'static> {
    value: Value<T>,
}

#[tokio::test]
async fn derive_initializable() -> dijavu::Result<()> {
    #[derive(Injectable)]
    struct TestInject<T: Default + Send + Sync + 'static>(Test<T>);

    let mut injector = InitInjector::default();
    injector.get::<TestInject<String>>().await?.0.value = "foo".to_owned();
    injector.get::<TestInject<i32>>().await?.0.value = 1;

    let injector = injector.build().await?;

    assert_eq!(
        *injector.get::<TestInject<String>>().0.value,
        "foo".to_owned()
    );
    assert_eq!(*injector.get::<TestInject<i32>>().0.value, 1);

    Ok(())
}
