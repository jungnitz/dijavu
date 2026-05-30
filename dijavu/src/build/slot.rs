use crate::DataKey;
use crate::data::DataValue;
use std::marker::PhantomData;
use std::sync::OnceLock;

/// A leaked write-once memory location.
pub(crate) struct Slot<T: 'static>(&'static OnceLock<T>);

impl<T> Slot<T> {
    pub fn uninit() -> Self {
        Self(Box::leak(Box::new(OnceLock::new())))
    }

    /// Stores `value` in this slot.
    pub fn set(self, value: T) {
        assert!(
            self.0.set(value).is_ok(),
            "slot value was already set. this is a bug."
        );
    }

    /// Returns the value stored in this slot
    #[must_use]
    pub fn get(self) -> Option<&'static T> {
        self.0.get()
    }
}

impl<T: 'static> Copy for Slot<T> {}

impl<T: 'static> Clone for Slot<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub(crate) struct SlotKey<T: 'static>(PhantomData<T>);

impl<T: DataValue> DataKey for SlotKey<T> {
    type Value = Slot<T>;
}
