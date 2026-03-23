use crate::Result;
use crate::init::InitAppContainer;
use std::sync::{LazyLock, Mutex};

static HOOKS: LazyLock<Mutex<Hooks>> = LazyLock::new(Mutex::default);

#[derive(Default)]
#[expect(clippy::type_complexity)]
struct Hooks {
    before_build: Vec<Box<dyn Fn(&mut InitAppContainer) -> Result<()> + Send + Sync>>,
}

/// Adds a hook that is executed before each build process.
pub fn add_global_before_build_hook(
    on_build: impl Fn(&mut InitAppContainer) -> Result<()> + Send + Sync + 'static,
) {
    HOOKS.lock().unwrap().before_build.push(Box::new(on_build));
}

pub(crate) fn run_global_before_build_hooks(container: &mut InitAppContainer) -> Result<()> {
    for on_build in &HOOKS.lock().unwrap().before_build {
        on_build(container)?;
    }
    Ok(())
}
