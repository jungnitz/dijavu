use std::any::Any;

use crate::build::dependency_graph::NodeIndex;

/// Storage for `Declaration`, `Preparation` and `Start` instances of each injectable.
///
/// The values are stored as `Box<dyn Any + Send>` and accessed by the index of the injectable's
/// node in the dependency graph.
#[derive(Default)]
pub(super) struct ValuePerInjectable {
    inner: Vec<Option<Box<dyn Any + Send>>>,
    len: usize,
}

impl ValuePerInjectable {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: std::iter::repeat_with(|| None).take(capacity).collect(),
            len: 0,
        }
    }

    pub(super) fn insert<T: Send + 'static>(&mut self, index: NodeIndex, value: T) {
        if self.inner.len() <= index.to_usize() {
            self.inner.resize_with(index.to_usize() + 1, || None);
        }
        let previous = self.inner[index.to_usize()].replace(Box::new(value));
        assert!(
            previous.is_none(),
            "only one value should only be inserted per injectable"
        );
        self.len += 1
    }

    pub(super) fn remove<T: Send + 'static>(&mut self, index: NodeIndex) -> T {
        self.try_remove(index)
            .expect("value should have been inserted")
    }

    fn try_remove<T: Send + 'static>(&mut self, index: NodeIndex) -> Option<T> {
        let removed = self.inner.get_mut(index.to_usize())?.take()?;
        self.len -= 1;
        Some(*removed.downcast::<T>().unwrap())
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }
}
