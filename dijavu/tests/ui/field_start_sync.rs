use dijavu::InjectableField;

#[derive(InjectableField)]
#[injectable(start_sync = Self::on_start)]
struct StartOnField;

fn main() {}
