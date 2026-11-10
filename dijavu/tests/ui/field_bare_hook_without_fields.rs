use dijavu::InjectableField;

#[derive(InjectableField)]
#[injectable(prepare_sync = on_prepare)]
struct Empty;

fn main() {}
