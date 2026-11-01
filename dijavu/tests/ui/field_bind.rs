use dijavu::{InjectableField};

trait Service {}

#[derive(InjectableField)]
#[injectable(bind(dyn Service))]
struct BindOnField;

fn main() {}
