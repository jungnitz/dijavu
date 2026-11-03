use dijavu::InjectableField;

#[derive(InjectableField)]
#[injectable(start = Self::on_start)]
struct StartOnField;

fn main() {}
