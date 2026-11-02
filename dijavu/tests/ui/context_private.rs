use dijavu::{FieldPrepareContext, PrepareContext};

fn forge(field: FieldPrepareContext<'_>) {
    let _ = PrepareContext::<()>::new(field);
}

fn main() {}
