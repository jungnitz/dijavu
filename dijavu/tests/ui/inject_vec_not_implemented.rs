use dijavu::{
    Error, Injectable, PrepareContext,
    fields::{Inject, InjectVec},
    state::Declare,
};

trait Service: Send + Sync {}
dijavu::unsizable!(Service);

#[derive(Injectable)]
struct NotAService;

#[derive(Injectable)]
#[injectable(prepare = Self::on_prepare)]
struct AddsWrongType {
    services: InjectVec<dyn Service>,
    not_a_service: Inject<NotAService>,
}

impl AddsWrongType {
    async fn on_prepare(
        state: &mut AddsWrongTypeState<Declare>,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<(), Error> {
        state.services.add(state.not_a_service);
        Ok(())
    }
}

fn main() {}
