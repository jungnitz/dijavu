use dijavu::{Visitable};

#[derive(Visitable)]
struct UnknownAttribute {
    #[visitable(nope)]
    value: (),
}

fn main() {}
