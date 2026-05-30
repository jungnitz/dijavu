use crate::{InitInjector, Result};
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::{LazyLock, Mutex};

static HOOKS: LazyLock<Mutex<Hooks>> = LazyLock::new(Mutex::default);

/// Asynchronous function that can be added as a before build hook.
pub type BeforeBuildHook = Box<
    dyn for<'a> Fn(&'a mut InitInjector) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
        + Send,
>;

#[derive(Default)]
struct Hooks {
    before_build: Vec<BeforeBuildHook>,
}

/// Adds a hook that is executed immediately before the injectables are built in
/// [`InitInjector::build`].
pub fn add_global_before_build_hook(on_build: BeforeBuildHook) {
    #[expect(
        clippy::missing_panics_doc,
        reason = "lock cannot be poisoned, panics are handled in the run method"
    )]
    HOOKS.lock().unwrap().before_build.push(on_build);
}

#[expect(
    clippy::await_holding_lock,
    reason = "there will be practically no contention on this mutex"
)]
pub(crate) async fn run_global_before_build_hooks(injector: &mut InitInjector) -> Result<()> {
    let hooks = &HOOKS.lock().unwrap();
    for on_build in &hooks.before_build {
        AssertUnwindSafe(on_build(injector))
            .catch_unwind()
            .await
            .map_err(|_| crate::Error::msg("a global before build hook panicked"))
            .flatten()?;
    }
    Ok(())
}
