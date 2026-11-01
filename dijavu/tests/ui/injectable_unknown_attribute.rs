use dijavu::{Injectable};

#[derive(Injectable)]
#[injectable(nope = 1)]
struct UnknownAttribute;

fn main() {}
