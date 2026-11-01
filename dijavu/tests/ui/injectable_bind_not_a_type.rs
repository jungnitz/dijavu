use dijavu::{Injectable};

#[derive(Injectable)]
#[injectable(bind(not a type))]
struct BindNotAType;

fn main() {}
