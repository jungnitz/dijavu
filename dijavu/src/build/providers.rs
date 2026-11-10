use std::any::{Any, TypeId};

use rustc_hash::FxHashMap;

use crate::{Injectable, UnsizeFrom, build::dependency_graph::NodeIndex, refs::Ref};

/// The injectables providing unsized types (e.g. trait objects), which are collected by
/// [`InjectAll`](crate::fields::InjectAll).
#[derive(Default)]
pub(super) struct Providers {
    /// Maps the `TypeId` of the provided type to its providers, in the order in which they were
    /// added to the build.
    entries: FxHashMap<TypeId, Vec<Provider>>,
}

struct Provider {
    node_index: NodeIndex,
    impl_type_id: TypeId,
    /// A `fn() -> Ref<D>` creating references to the providing injectable.
    new_ref: Box<dyn Any + Send>,
}

impl Providers {
    /// Adds the injectable `I`, which belongs to `node_index`, as a provider of `D`.
    ///
    /// Does nothing if `I` already provides `D`.
    pub(super) fn provide<D: ?Sized + UnsizeFrom<I>, I: Injectable>(
        &mut self,
        node_index: NodeIndex,
    ) {
        let providers = self.entries.entry(TypeId::of::<D>()).or_default();
        if providers
            .iter()
            .any(|provider| provider.impl_type_id == TypeId::of::<I>())
        {
            return;
        }
        let new_ref: fn() -> Ref<D> = Ref::unsized_from::<I>;
        // nodes are numbered in the order in which they were added to the build
        let position = providers
            .partition_point(|provider| provider.node_index.to_usize() < node_index.to_usize());
        providers.insert(
            position,
            Provider {
                node_index,
                impl_type_id: TypeId::of::<I>(),
                new_ref: Box::new(new_ref),
            },
        );
    }

    /// Returns references to the providers of `D` and the nodes they belong to.
    pub(super) fn get_refs_and_indices<D: ?Sized + Send + Sync + 'static>(
        &self,
    ) -> impl Iterator<Item = (Ref<D>, NodeIndex)> + '_ {
        self.entries
            .get(&TypeId::of::<D>())
            .into_iter()
            .flatten()
            .map(|provider| {
                let new_ref = provider
                    .new_ref
                    .as_ref()
                    .downcast_ref::<fn() -> Ref<D>>()
                    .unwrap();
                (new_ref(), provider.node_index)
            })
    }
}
