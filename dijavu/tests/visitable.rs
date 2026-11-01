//! `#[derive(Visitable)]` must accept all kinds of types. These only need to compile.

#![allow(dead_code)]

use std::{cell::Cell, rc::Rc, sync::Mutex};

use dijavu::{
    Visitable,
    fields::{Depend, Inject},
};

#[derive(Visitable)]
struct Tuple(Inject<()>, #[visitable(skip)] Mutex<u32>, Depend<()>);

#[derive(Visitable)]
struct Generic<T, U: Visitable> {
    t: T,
    u: U,
    #[visitable(skip)]
    skipped: Rc<T>,
}

#[derive(Visitable)]
enum Enum<T> {
    Unit,
    Tuple(Inject<()>, T),
    Named {
        a: Depend<()>,
        #[visitable(skip)]
        skipped: Cell<u32>,
    },
}

#[derive(Visitable)]
struct Unit;

#[derive(Visitable)]
enum Never {}

fn assert_visitable<T: Visitable>() {}

fn all_visitable() {
    assert_visitable::<Tuple>();
    assert_visitable::<Generic<(), Unit>>();
    assert_visitable::<Enum<()>>();
    assert_visitable::<Unit>();
    assert_visitable::<Never>();
}
