use dijavu::{DeclareContext, Injectable, Registered, state::Declare};

trait Plugin: Send + Sync {}

impl Registered for Box<dyn Plugin> {
    type Consumer = ();
}

struct MyPlugin;

impl Plugin for MyPlugin {}

#[derive(Injectable)]
#[injectable(declare = Self::on_declare)]
struct Registrar;

impl Registrar {
    fn on_declare(
        _state: &mut RegistrarState<Declare>,
        mut ctx: DeclareContext<'_, Self>,
    ) -> Result<(), dijavu::Error> {
        // infers `Box<MyPlugin>` instead of `Box<dyn Plugin>`
        ctx.register(Box::new(MyPlugin));
        Ok(())
    }
}

fn main() {}
