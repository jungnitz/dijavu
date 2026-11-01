use dijavu::{Injectable};

#[derive(Injectable)]
#[injectable(prepare = Self::hook, prepare = Self::hook)]
struct DuplicateAttribute;

fn main() {}
