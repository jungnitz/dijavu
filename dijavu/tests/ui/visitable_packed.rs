use dijavu::{Visitable};

#[derive(Visitable)]
#[repr(packed)]
struct Packed {
    value: (),
}

fn main() {}
