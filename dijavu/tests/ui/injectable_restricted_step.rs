use dijavu::{FieldPrepareContext, Injectable, PrepareContext};

#[derive(Injectable)]
struct Other;

#[derive(Injectable)]
struct Mine;

fn with_field_context(declaration: <Other as Injectable>::Declaration, ctx: FieldPrepareContext<'_>) {
    let _ = Other::prepare(declaration, ctx);
}

fn with_context_of_another_injectable(
    declaration: <Other as Injectable>::Declaration,
    ctx: PrepareContext<'_, Mine>,
) {
    let _ = Other::prepare(declaration, ctx);
}

fn main() {}
