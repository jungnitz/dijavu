use dijavu::{Injectable, fields::Value};

#[derive(Injectable)]
struct NamedCtx {
    ctx: Value<u32>,
}

fn main() {}
