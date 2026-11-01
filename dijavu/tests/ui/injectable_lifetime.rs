use dijavu::{Injectable};

#[derive(Injectable)]
struct HasLifetime<'a> {
    value: &'a u32,
}

fn main() {}
