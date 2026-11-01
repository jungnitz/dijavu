use dijavu::Injectable;

#[derive(Injectable)]
#[injectable(register(1u32))]
struct RegisterUnmarked;

fn main() {}
