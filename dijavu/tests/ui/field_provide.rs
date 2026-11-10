use dijavu::InjectableField;

trait Service: Send + Sync {}
dijavu::unsizable!(Service);

#[derive(InjectableField)]
#[injectable(provide(dyn Service))]
struct ProvideOnField;

fn main() {}
