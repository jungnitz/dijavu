use dijavu::{Injectable, PrepareContext, Registered, state::Declare};

trait Plugin: Send + Sync {}

impl Registered for Box<dyn Plugin> {
    type Consumer = ();
}

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct Reader;

impl Reader {
    async fn on_prepare(
        _state: &mut ReaderState<Declare>,
        ctx: PrepareContext<'_, Self>,
    ) -> Result<(), dijavu::Error> {
        let _ = ctx.registered::<std::sync::Arc<dyn Plugin>>();
        Ok(())
    }
}

fn main() {}
