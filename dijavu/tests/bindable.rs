//! `bindable!` must work outside of `dijavu`, despite the blanket `Target` impl for injectables.

use dijavu::{
    Bindable, InjectableField, Target,
    fields::{Depend, Inject},
};

trait WithSupertraits: Send + Sync {}
dijavu::bindable!(WithSupertraits);

trait WithoutSupertraits {}
dijavu::bindable!(WithoutSupertraits + Send + Sync);

trait Generic<T>: Send + Sync {}
dijavu::bindable!(Generic<u32>);

impl WithSupertraits for () {}
impl WithoutSupertraits for () {}
impl Generic<u32> for () {}

fn assert_bindable<B: ?Sized + Bindable<()>>() {}
fn assert_field<F: InjectableField>() {}
fn assert_target<T: ?Sized + Target>() {}

#[test]
fn bindable_outside_of_crate() {
    assert_bindable::<dyn WithSupertraits>();
    assert_bindable::<dyn WithoutSupertraits + Send + Sync>();
    assert_bindable::<dyn Generic<u32>>();
    assert_target::<()>();
    assert_field::<Inject<dyn WithSupertraits>>();
    assert_field::<Depend<dyn WithoutSupertraits + Send + Sync>>();
    assert_field::<Inject<()>>();
}
