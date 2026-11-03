use dijavu::{
    Builder, Injectable,
    fields::{Depend, Inject},
};

trait Service: Send + Sync {}
dijavu::unsizable!(Service);

#[derive(Injectable)]
struct Impl;

impl Service for Impl {}

struct UsesInject(Inject<dyn Service>);

struct UsesDepend(Depend<dyn Service>);

fn main() {
    let _ = Builder::new().bind::<dyn Service, Impl>();
}
