use dijavu::{Injectable, fields::Value};

#[derive(Injectable)]
struct UsesSelf {
    this: Value<Vec<Self>>,
}

fn main() {}
