use std::{any::TypeId, borrow::Cow};

use futures::future::BoxFuture;
use petgraph::{Graph, algo::toposort, visit::EdgeFiltered};
use rustc_hash::FxHashMap;

use crate::{
    Error, Injectable,
    build::{BuildState, assemble::AssembleStep, prepare::PrepareStep},
    error::CycleError,
};

pub enum DependencyType {
    /// A hard dependency, i.e. if `A -> B`, then `A` must be initialized after `B`
    Hard,
    /// A soft dependency, i.e. `A` uses `B` only after build
    Soft,
}

#[derive(Default)]
pub struct DependencyGraph {
    /// Add-only graph (so that node indices remain stable)
    graph: Graph<Node, DependencyType>,
    node_indices: FxHashMap<TypeId, NodeIndex>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct NodeIndex(petgraph::graph::NodeIndex);

impl NodeIndex {
    pub(super) fn to_usize(self) -> usize {
        self.0.index()
    }

    pub(super) fn from_usize(idx: usize) -> Self {
        Self(petgraph::graph::NodeIndex::new(idx))
    }
}

impl DependencyGraph {
    pub(super) fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub(super) fn get_or_add<I: Injectable>(&mut self) -> (NodeIndex, bool) {
        let mut added = false;
        let index = *self
            .node_indices
            .entry(TypeId::of::<I>())
            .or_insert_with(|| {
                added = true;
                NodeIndex(self.graph.add_node(Node::new::<I>()))
            });
        (index, added)
    }

    pub(super) fn get_index<I: Injectable>(&self) -> NodeIndex {
        self.node_indices[&TypeId::of::<I>()]
    }

    pub(super) fn get(&self, index: NodeIndex) -> &Node {
        &self.graph[index.0]
    }

    pub(super) fn add_dependency(&mut self, from: NodeIndex, to: NodeIndex, typ: DependencyType) {
        self.graph.add_edge(from.0, to.0, typ);
    }

    pub(super) fn topological_sort(&self) -> Result<Vec<NodeIndex>, CycleError> {
        let hard_deps_only = EdgeFiltered::from_fn(&self.graph, |edge| {
            matches!(edge.weight(), DependencyType::Hard)
        });
        let node_ids = toposort(&hard_deps_only, None)
            .map_err(|cycle| CycleError::new(self.graph[cycle.node_id()].name().clone()))?;
        Ok(node_ids.into_iter().rev().map(NodeIndex).collect())
    }
}

/// Runs a build step for the injectable belonging to the given node.
type StepFn<Step> = fn(&mut BuildState<Step>, NodeIndex) -> BoxFuture<'_, Result<(), Error>>;

pub struct Node {
    name: Cow<'static, str>,
    pub(super) prepare: StepFn<PrepareStep>,
    pub(super) assemble: StepFn<AssembleStep>,
}

impl Node {
    fn new<T: Injectable>() -> Self {
        Self {
            name: T::name(),
            prepare: |state, index| Box::pin(state.prepare_one::<T>(index)),
            assemble: |state, index| Box::pin(state.assemble_one::<T>(index)),
        }
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}
