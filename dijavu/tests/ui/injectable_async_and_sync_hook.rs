use dijavu::Injectable;

#[derive(Injectable)]
#[injectable(prepared = on_prepared, prepared_sync = on_prepared)]
struct Both;

fn main() {}
