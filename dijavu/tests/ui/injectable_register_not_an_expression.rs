use dijavu::Injectable;

#[derive(Injectable)]
#[injectable(register(1 +))]
struct RegisterNotAnExpression;

fn main() {}
