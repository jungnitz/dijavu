use std::{
    any::{Any, TypeId, type_name},
    borrow::Cow,
};

use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry as MapEntry;

use crate::{
    Bindable, Injectable, Target, build::dependency_graph::NodeIndex, error::BindingConflictError,
    refs::Ref,
};

/// The injectables that unsized [`Target`]s (e.g. trait objects) are bound to.
#[derive(Default)]
pub(super) struct Bindings {
    entries: FxHashMap<TypeId, Binding>,
}

struct Binding {
    node_index: NodeIndex,
    impl_type_id: TypeId,
    impl_name: Cow<'static, str>,
    /// A `fn() -> Ref<B>` creating references to the bound injectable.
    new_ref: Box<dyn Any + Send>,
}

impl Bindings {
    /// Fails if `B` is already bound to an injectable other than `I`.
    ///
    /// Binding the same target to the same injectable multiple times is allowed.
    pub(super) fn check_conflict<B: ?Sized + Bindable<I>, I: Injectable>(
        &self,
    ) -> Result<(), BindingConflictError> {
        match self.entries.get(&TypeId::of::<B>()) {
            Some(existing) if existing.impl_type_id != TypeId::of::<I>() => Err(
                BindingConflictError::new(type_name::<B>(), existing.impl_name.clone(), I::name()),
            ),
            _ => Ok(()),
        }
    }

    /// Binds `B` to the injectable `I`, which belongs to `node_index`.
    ///
    /// Does nothing if `B` is already bound, so call [`check_conflict`](Self::check_conflict)
    /// first.
    pub(super) fn bind<B: ?Sized + Bindable<I>, I: Injectable>(&mut self, node_index: NodeIndex) {
        if let MapEntry::Vacant(vacant) = self.entries.entry(TypeId::of::<B>()) {
            let new_ref: fn() -> Ref<B> = Ref::bound::<I>;
            vacant.insert(Binding {
                node_index,
                impl_type_id: TypeId::of::<I>(),
                impl_name: I::name(),
                new_ref: Box::new(new_ref),
            });
        }
    }

    pub(super) fn get_index_by_type_id(&self, type_id: TypeId) -> Option<NodeIndex> {
        Some(self.entries.get(&type_id)?.node_index)
    }

    /// Returns a reference to the injectable `B` is bound to.
    pub(super) fn get_ref_and_index<B: ?Sized + Target>(&self) -> Option<(Ref<B>, NodeIndex)> {
        let entry = self.entries.get(&TypeId::of::<B>())?;
        let new_ref = entry
            .new_ref
            .as_ref()
            .downcast_ref::<fn() -> Ref<B>>()
            .unwrap();
        Some((new_ref(), entry.node_index))
    }
}
