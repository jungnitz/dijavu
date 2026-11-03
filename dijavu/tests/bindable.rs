//! `bindable!` must work outside of `dijavu`, despite the blanket `Target` impl for injectables.

use dijavu::{
    Bindable, InjectableField, Target, UnsizeFrom,
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

trait UnsizableWithSupertraits: Send + Sync {}
dijavu::unsizable!(UnsizableWithSupertraits);

trait UnsizableWithoutSupertraits {}
dijavu::unsizable!(UnsizableWithoutSupertraits + Send + Sync);

trait UnsizableGeneric<T>: Send + Sync {}
dijavu::unsizable!(UnsizableGeneric<u32>);

fn assert_unsizable<U: ?Sized + UnsizeFrom<()>>() {}
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

impl UnsizableWithSupertraits for () {}
impl UnsizableWithoutSupertraits for () {}
impl UnsizableGeneric<u32> for () {}

#[test]
fn unsizable_outside_of_crate() {
    assert_unsizable::<dyn UnsizableWithSupertraits>();
    assert_unsizable::<dyn UnsizableWithoutSupertraits + Send + Sync>();
    assert_unsizable::<dyn UnsizableGeneric<u32>>();
    // `bindable!` includes `unsizable!`
    assert_unsizable::<dyn WithSupertraits>();
}
