use dijavu::Injectable;

#[derive(Injectable)]
#[injectable(register = 1)]
struct RegisterNotAList;

fn main() {}
