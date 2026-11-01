use dijavu::{Injectable};

#[derive(Injectable)]
#[injectable(bind = 1)]
struct BindNotAList;

fn main() {}
