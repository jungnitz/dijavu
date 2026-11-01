use std::any::{Any, TypeId, type_name};

use rustc_hash::FxHashMap;

use crate::{Injectable, error::MissingInputError, fields::Input};

/// The values set via [`Builder::set_input`](crate::Builder::set_input), one per type.
#[derive(Default)]
pub(super) struct Inputs {
    /// Maps the `TypeId` of `T` to an `Input<T>`.
    entries: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Inputs {
    pub(super) fn set<T: Send + Sync + 'static>(&mut self, value: T) {
        self.entries
            .insert(TypeId::of::<T>(), Box::new(Input::new(value)));
    }

    /// Returns the input of type `T` for the injectable `I`, which is only used for the error.
    pub(super) fn get<I: Injectable, T: Send + Sync + 'static>(
        &self,
    ) -> Result<Input<T>, MissingInputError> {
        let Some(input) = self.entries.get(&TypeId::of::<T>()) else {
            return Err(MissingInputError::new(type_name::<T>(), I::name()));
        };
        Ok(input.downcast_ref::<Input<T>>().unwrap().clone())
    }
}
