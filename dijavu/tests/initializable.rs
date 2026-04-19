use dijavu::{AppContainerBuilder, InitAppContainer, Initializable, Value};

#[derive(Initializable)]
struct Test {
    str: Value<String>,
}

#[tokio::test]
async fn derive_initializable() -> dijavu::Result<()> {
    let mut init_container = InitAppContainer::default();

    let mut init = Test::new_init_value(&mut init_container)?;
    assert_eq!(init.str, "");
    init.str = "str".to_owned();

    let mut builder = AppContainerBuilder::default();
    let runtime = Box::leak(Box::new(Test::build_runtime_value(
        init,
        init_container.data_mut(),
        &mut builder,
    )?));
    let container = builder.build().await?;

    assert_eq!(
        Test::from_runtime_value(runtime, container.data())?
            .str
            .into_static_ref(),
        "str"
    );
    Ok(())
}
