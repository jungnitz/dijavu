use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::collections::hash_map;
use std::convert::Infallible;
use std::marker::PhantomData;

/// Container storing heterogeneous values via key types
///
/// Each entry is identified by its key type `K: DataKey` and contains a value of type `K::Value`.
///
/// ## Example
///
/// ```
/// # use dijavu::{Data, DataKey};
/// struct MyKey;
/// impl DataKey for MyKey {
///     type Value = i32;
/// }
///
/// let mut data = Data::default();
/// data.insert::<MyKey>(42);
///
/// assert_eq!(data.get::<MyKey>(), Some(&42));
/// ```
#[derive(Default)]
pub struct Data(FxHashMap<TypeId, Value>);

impl Data {
    /// Inserts a value for the given key type.
    ///
    /// Returns the previous value if one existed.
    pub fn insert<K: DataKey>(&mut self, data: K::Value) -> Option<K::Value> {
        self.0
            .insert(TypeId::of::<K>(), Value::new(data))
            .map(Value::downcast)
    }

    /// Returns an entry API for in-place manipulation.
    ///
    /// Similar to [`HashMap::entry`](std::collections::HashMap::entry), but with a key type.
    #[must_use]
    pub fn entry<K: DataKey>(&mut self) -> DataEntry<'_, K> {
        match self.0.entry(TypeId::of::<K>()) {
            hash_map::Entry::Vacant(vacant) => DataEntry::Vacant(VacantEntry(vacant, PhantomData)),
            hash_map::Entry::Occupied(occupied) => {
                DataEntry::Occupied(OccupiedEntry(occupied, PhantomData))
            }
        }
    }

    /// Removes the value associated with the key.
    ///
    /// Returns the value if it existed.
    pub fn remove<K: DataKey>(&mut self) -> Option<K::Value> {
        self.0.remove(&TypeId::of::<K>()).map(Value::downcast)
    }

    /// Returns `true` if a value for key `K` exists.
    #[must_use]
    pub fn contains_key<K: DataKey>(&self) -> bool {
        self.0.contains_key(&TypeId::of::<K>())
    }

    /// Returns a reference to the value associated with key `K`.
    #[must_use]
    pub fn get<K: DataKey>(&self) -> Option<&K::Value> {
        self.0.get(&TypeId::of::<K>()).map(Value::downcast_ref)
    }

    /// Returns a mutable reference to the value associated with key `K`.
    #[must_use]
    pub fn get_mut<K: DataKey>(&mut self) -> Option<&mut K::Value> {
        self.0.get_mut(&TypeId::of::<K>()).map(Value::downcast_mut)
    }
}

/// View into a single entry in a [`Data`] container
pub enum DataEntry<'a, K> {
    /// Vacant entry (no value present)
    Vacant(VacantEntry<'a, K>),
    /// Occupied entry (value already present)
    Occupied(OccupiedEntry<'a, K>),
}

impl<'a, K> DataEntry<'a, K>
where
    K: DataKey,
{
    /// Ensures a value is present and returns an occupied entry.
    ///
    /// If vacant, inserts the value. If occupied, replaces it.
    pub fn insert_entry(self, value: K::Value) -> OccupiedEntry<'a, K> {
        match self {
            DataEntry::Vacant(entry) => entry.insert_entry(value),
            DataEntry::Occupied(mut entry) => {
                entry.insert(value);
                entry
            }
        }
    }

    pub fn or_insert_with(self, f: impl FnOnce() -> K::Value) -> &'a mut K::Value {
        match self.or_try_insert_with::<Infallible>(|| Ok(f())) {
            Ok(ok) => ok,
            Err(err) => match err {},
        }
    }

    /// Inserts the value provided by the fallible closure if no value is currently present.
    ///
    /// # Errors
    ///
    /// This method forwards any errors from `f`.
    pub fn or_try_insert_with<E>(
        self,
        f: impl FnOnce() -> Result<K::Value, E>,
    ) -> Result<&'a mut K::Value, E> {
        match self {
            DataEntry::Occupied(entry) => Ok(entry.into_mut()),
            DataEntry::Vacant(entry) => match f() {
                Ok(value) => Ok(entry.insert(value)),
                Err(err) => Err(err),
            },
        }
    }
}

/// Vacant entry in [`Data`]
pub struct VacantEntry<'a, K>(hash_map::VacantEntry<'a, TypeId, Value>, PhantomData<fn(K)>);

impl<'a, K> VacantEntry<'a, K>
where
    K: DataKey,
{
    /// Inserts a value and returns a mutable reference to it.
    pub fn insert(self, value: K::Value) -> &'a mut K::Value {
        self.0.insert(Value::new(value)).downcast_mut()
    }

    /// Inserts a value and returns an occupied entry.
    pub fn insert_entry(self, value: K::Value) -> OccupiedEntry<'a, K> {
        OccupiedEntry(self.0.insert_entry(Value::new(value)), PhantomData)
    }
}

/// An occupied entry in [`Data`]
pub struct OccupiedEntry<'a, K>(
    hash_map::OccupiedEntry<'a, TypeId, Value>,
    PhantomData<fn(K)>,
);

impl<'a, K> OccupiedEntry<'a, K>
where
    K: DataKey,
{
    /// Replaces the value, returning the old one.
    pub fn insert(&mut self, value: K::Value) -> K::Value {
        self.0.insert(Value::new(value)).downcast()
    }

    /// Returns a shared reference to the value.
    #[must_use]
    pub fn get(&self) -> &K::Value {
        self.0.get().downcast_ref()
    }

    /// Returns a mutable reference to the value.
    #[must_use]
    pub fn get_mut(&mut self) -> &mut K::Value {
        self.0.get_mut().downcast_mut()
    }

    /// Converts into a mutable reference with the entry lifetime.
    #[must_use]
    pub fn into_mut(self) -> &'a mut K::Value {
        self.0.into_mut().downcast_mut()
    }

    /// Removes and returns the value.
    #[expect(clippy::must_use_candidate)]
    pub fn remove(self) -> K::Value {
        self.0.remove().downcast()
    }
}

/// Key type for accessing values in [`Data`]
///
/// Each key type defines the associated value type it stores.
pub trait DataKey: 'static {
    /// The value type stored in an entry for this key
    type Value: DataValue;
}

/// Value that can be stored in [`Data`]
///
/// This is automatically implemented for all `Send + Sync + 'static` types.
pub trait DataValue: Any + Send + Sync {}

impl<T: Any + Send + Sync> DataValue for T {}

struct Value(Box<dyn Any + Send + Sync>);

impl Value {
    fn new<T: DataValue>(v: T) -> Self {
        Self(Box::new(v))
    }

    fn downcast<T: DataValue>(self) -> T {
        *(Box::<dyn Any + Send + Sync>::downcast(self.0).unwrap())
    }

    fn downcast_ref<T: DataValue>(&self) -> &T {
        Box::as_ref(&self.0).downcast_ref::<T>().unwrap()
    }

    fn downcast_mut<T: DataValue>(&mut self) -> &mut T {
        Box::as_mut(&mut self.0).downcast_mut::<T>().unwrap()
    }
}

macro_rules! define_data_wrapper {
    (
        $(#[doc = $doc:literal])*
        $vis:vis $name:ident;
    ) => {
        $(#[doc = $doc])*
        #[derive(Default)]
        $vis struct $name(crate::data::Data);

        impl std::ops::Deref for $name {
            type Target = crate::data::Data;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

pub(crate) use define_data_wrapper;

#[cfg(test)]
mod tests {
    use super::*;

    struct KeyA;
    impl DataKey for KeyA {
        type Value = u32;
    }

    struct KeyB;
    impl DataKey for KeyB {
        type Value = u32;
    }

    #[test]
    fn insert_and_get() {
        let mut data = Data::default();
        assert!(data.insert::<KeyA>(42).is_none());
        assert_eq!(data.get::<KeyA>(), Some(&42));
        assert!(data.contains_key::<KeyA>());
        assert_eq!(data.remove::<KeyA>(), Some(42));
        assert!(!data.contains_key::<KeyA>());
    }

    #[test]
    fn conflict() {
        let mut data = Data::default();
        assert!(data.insert::<KeyA>(42).is_none());
        assert!(data.insert::<KeyB>(1).is_none());
        assert_eq!(data.get::<KeyA>(), Some(&42));
        assert_eq!(data.get::<KeyB>(), Some(&1));
    }

    #[test]
    fn insert_replace() {
        let mut data = Data::default();
        assert!(data.insert::<KeyA>(42).is_none());
        assert_eq!(data.insert::<KeyA>(32), Some(42));
    }

    #[test]
    fn entry() {
        let mut data = Data::default();
        let DataEntry::Vacant(entry) = data.entry::<KeyA>() else {
            panic!("not vacant");
        };
        entry.insert(42);
        assert_eq!(data.get::<KeyA>(), Some(&42));
        let DataEntry::Occupied(mut entry) = data.entry::<KeyA>() else {
            panic!("not occupied");
        };
        assert_eq!(entry.get(), &42);
        assert_eq!(entry.get_mut(), &42);
        assert_eq!(entry.remove(), 42);
        assert_eq!(data.entry::<KeyA>().insert_entry(32).into_mut(), &32);
    }
}
