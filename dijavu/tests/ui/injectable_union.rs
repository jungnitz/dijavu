use dijavu::{Injectable};

#[derive(Injectable)]
union NotAStruct {
    value: u32,
}

fn main() {}
