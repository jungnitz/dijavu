use dijavu::{Visitable};

#[derive(Visitable)]
union NotVisitable {
    value: u32,
}

fn main() {}
