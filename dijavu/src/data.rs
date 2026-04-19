use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::collections::hash_map;
use std::convert::Infallible;
use std::marker::PhantomData;

/// Container storing heterogeneous values via key types
///
/// Each key `K: DataKey` maps to exactly one value related to type `K::Item`.
/// The relation between the value and `K::Item` is described using the `V` generic parameter.
/// For this parameter, we provide the implementations [`BoxValues`] and [`LeakedValues`] which
/// store the values as `Box<dyn Any + ...>` or `&'static (dyn Any + ...)` (by leaking them)
/// respectively.
///
/// ## Example
///
/// ```rust
/// # use dijavu::{Data, DataKey, data::BoxValues};
/// struct MyKey;
/// impl DataKey for MyKey {
///     type Item = i32;
/// }
///
/// let mut data = Data::<BoxValues>::default();
/// data.insert::<MyKey>(42);
///
/// assert_eq!(data.get::<MyKey>(), Some(&42));
/// ```
pub struct Data<V: DataValues = BoxValues>(
    // INVARIANT: TypeId::of::<K: DataKey>() maps to a Box<K::Item>
    FxHashMap<TypeId, V::EntryValue>,
);

impl<V: DataValues> Default for Data<V> {
    fn default() -> Self {
        Data(FxHashMap::default())
    }
}

impl<V: DataValues> Data<V> {
    /// Inserts a value for the given key type.
    ///
    /// Returns the previous value if one existed.
    pub fn insert<K: DataKey>(&mut self, data: K::Item) -> Option<V::OwnedValue<K::Item>> {
        self.0
            .insert(TypeId::of::<K>(), V::entry_value(data))
            .map(V::downcast)
    }

    /// Returns an entry API for in-place manipulation.
    ///
    /// Similar to [`HashMap::entry`](std::collections::HashMap::entry), but with a key type.
    pub fn entry<K: DataKey>(&mut self) -> DataEntry<'_, K, V> {
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
    pub fn remove<K: DataKey>(&mut self) -> Option<V::OwnedValue<K::Item>> {
        self.0.remove(&TypeId::of::<K>()).map(V::downcast)
    }

    /// Returns `true` if a value for key `K` exists.
    pub fn contains_key<K: DataKey>(&self) -> bool {
        self.0.contains_key(&TypeId::of::<K>())
    }

    /// Returns a reference to the value associated with key `K`.
    pub fn get<K: DataKey>(&self) -> Option<V::ReferenceValue<'_, K::Item>> {
        self.0.get(&TypeId::of::<K>()).map(V::downcast_ref)
    }

    /// Returns a mutable reference to the value associated with key `K`.
    pub fn get_mut<K: DataKey>(&mut self) -> Option<V::MutReferenceValue<'_, K::Item>> {
        self.0.get_mut(&TypeId::of::<K>()).map(V::downcast_mut)
    }
}

/// View into a single entry in a [`Data`] container
pub enum DataEntry<'a, K, V: DataValues> {
    /// Vacant entry (no value present)
    Vacant(VacantEntry<'a, K, V>),
    /// Occupied entry (value already present)
    Occupied(OccupiedEntry<'a, K, V>),
}

impl<'a, K, V: DataValues> DataEntry<'a, K, V>
where
    K: DataKey,
    V: DataValues,
{
    /// Ensures a value is present and returns an occupied entry.
    ///
    /// If vacant, inserts the value. If occupied, replaces it.
    pub fn insert_entry(self, item: K::Item) -> OccupiedEntry<'a, K, V> {
        match self {
            DataEntry::Vacant(entry) => entry.insert_entry(item),
            DataEntry::Occupied(mut entry) => {
                entry.insert(item);
                entry
            }
        }
    }

    pub fn or_insert_with(self, f: impl FnOnce() -> K::Item) -> V::MutReferenceValue<'a, K::Item> {
        match self.or_try_insert_with::<Infallible>(|| Ok(f())) {
            Ok(ok) => ok,
            Err(err) => match err {},
        }
    }

    /// Inserts the value provided by the fallible closure if no value is currently present.
    pub fn or_try_insert_with<E>(
        self,
        f: impl FnOnce() -> Result<K::Item, E>,
    ) -> Result<V::MutReferenceValue<'a, K::Item>, E> {
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
pub struct VacantEntry<'a, K, V: DataValues>(
    hash_map::VacantEntry<'a, TypeId, V::EntryValue>,
    PhantomData<fn(K)>,
);

impl<'a, K, V> VacantEntry<'a, K, V>
where
    K: DataKey,
    V: DataValues,
{
    /// Inserts a value and returns a mutable reference to it.
    pub fn insert(self, value: K::Item) -> V::MutReferenceValue<'a, K::Item> {
        V::downcast_mut(self.0.insert(V::entry_value(value)))
    }

    /// Inserts a value and returns an occupied entry.
    pub fn insert_entry(self, value: K::Item) -> OccupiedEntry<'a, K, V> {
        OccupiedEntry(self.0.insert_entry(V::entry_value(value)), PhantomData)
    }
}

/// An occupied entry in [`Data`]
pub struct OccupiedEntry<'a, K, V: DataValues>(
    hash_map::OccupiedEntry<'a, TypeId, V::EntryValue>,
    PhantomData<fn(K)>,
);

impl<'a, K, V> OccupiedEntry<'a, K, V>
where
    K: DataKey,
    V: DataValues,
{
    /// Replaces the value, returning the old one.
    pub fn insert(&mut self, value: K::Item) -> V::OwnedValue<K::Item> {
        V::downcast(self.0.insert(V::entry_value(value)))
    }

    /// Returns a shared reference to the value.
    pub fn get(&self) -> V::ReferenceValue<'_, K::Item> {
        V::downcast_ref(self.0.get())
    }

    /// Returns a mutable reference to the value.
    pub fn get_mut(&mut self) -> V::MutReferenceValue<'_, K::Item> {
        V::downcast_mut(self.0.get_mut())
    }

    /// Converts into a mutable reference with the entry lifetime.
    pub fn into_mut(self) -> V::MutReferenceValue<'a, K::Item> {
        V::downcast_mut(self.0.into_mut())
    }

    /// Removes and returns the value.
    pub fn remove(self) -> V::OwnedValue<K::Item> {
        V::downcast(self.0.remove())
    }
}

/// Key type for accessing values in [`Data`]
///
/// Each key type defines the associated value type it stores.
pub trait DataKey: 'static {
    /// The value type stored in an entry for this key
    type Item: DataValue;
}

/// Value that can be stored in [`Data`]
///
/// This is automatically implemented for all `Send + Sync + 'static` types.
pub trait DataValue: Any + Send + Sync {}

impl<T: Any + Send + Sync> DataValue for T {}

/// Describes how the values for a [`DataKey`] are stored in a [`Data`] instance.
pub trait DataValues {
    type OwnedValue<T: DataValue>: Borrow<T>;
    type ReferenceValue<'a, T: DataValue>: Borrow<T>;
    type MutReferenceValue<'a, T: DataValue>: Borrow<T>;
    type EntryValue;

    fn entry_value<T: DataValue>(v: T) -> Self::EntryValue;
    fn downcast<T: DataValue>(v: Self::EntryValue) -> Self::OwnedValue<T>;
    fn downcast_ref<T: DataValue>(v: &Self::EntryValue) -> Self::ReferenceValue<'_, T>;
    fn downcast_mut<T: DataValue>(v: &mut Self::EntryValue) -> Self::MutReferenceValue<'_, T>;
}

/// Store values as a `Box<dyn Any + ...>`
pub struct BoxValues;

impl DataValues for BoxValues {
    type OwnedValue<T: DataValue> = T;
    type ReferenceValue<'a, T: DataValue> = &'a T;
    type MutReferenceValue<'a, T: DataValue> = &'a mut T;
    type EntryValue = Box<dyn Any + Send + Sync>;

    fn entry_value<T: DataValue>(v: T) -> Self::EntryValue {
        Box::new(v)
    }

    fn downcast<T: DataValue>(v: Self::EntryValue) -> Self::OwnedValue<T> {
        *(Box::<dyn Any + Send + Sync>::downcast(v).unwrap())
    }

    fn downcast_ref<T: DataValue>(v: &Self::EntryValue) -> Self::ReferenceValue<'_, T> {
        Box::as_ref(v).downcast_ref::<T>().unwrap()
    }

    fn downcast_mut<T: DataValue>(v: &mut Self::EntryValue) -> Self::MutReferenceValue<'_, T> {
        Box::as_mut(v).downcast_mut::<T>().unwrap()
    }
}

/// Store values as a `&'static (dyn Any + ...)` by leaking them
pub struct LeakedValues;

impl DataValues for LeakedValues {
    type OwnedValue<T: DataValue> = &'static T;
    type ReferenceValue<'a, T: DataValue> = &'static T;
    type MutReferenceValue<'a, T: DataValue> = &'static T;
    type EntryValue = &'static (dyn Any + Send + Sync);

    fn entry_value<T: DataValue>(v: T) -> Self::EntryValue {
        Box::leak(Box::new(v))
    }

    fn downcast<T: DataValue>(v: Self::EntryValue) -> Self::OwnedValue<T> {
        Self::downcast_ref(&v)
    }

    fn downcast_ref<T: DataValue>(v: &Self::EntryValue) -> Self::ReferenceValue<'_, T> {
        v.downcast_ref::<T>().unwrap()
    }

    fn downcast_mut<T: DataValue>(v: &mut Self::EntryValue) -> Self::MutReferenceValue<'_, T> {
        Self::downcast_ref(v)
    }
}

macro_rules! define_data_wrapper {
    (
        $(#[doc = $doc:literal])*
        $vis:vis $name:ident;
    ) => {
        crate::data::define_data_wrapper!(
            $(#[doc = $doc])*
            $vis $name -> crate::data::BoxValues;
        );
    };
    (
        $(#[doc = $doc:literal])*
        $vis:vis $name:ident -> $ty:ty;
    ) => {
        $(#[doc = $doc])*
        #[derive(Default)]
        pub struct $name(crate::data::Data::<$ty>);

        impl std::ops::Deref for $name {
            type Target = crate::data::Data::<$ty>;

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
        type Item = u32;
    }

    struct KeyB;
    impl DataKey for KeyB {
        type Item = u32;
    }

    #[test]
    fn insert_and_get() {
        fn insert_and_get<V: DataValues>() {
            let mut data = Data::<V>::default();
            assert!(data.insert::<KeyA>(42).is_none());
            assert_eq!(data.get::<KeyA>().as_ref().map(Borrow::borrow), Some(&42));
            assert!(data.contains_key::<KeyA>());
            assert_eq!(
                data.remove::<KeyA>().as_ref().map(Borrow::borrow),
                Some(&42)
            );
            assert!(!data.contains_key::<KeyA>());
        }
        insert_and_get::<BoxValues>();
        insert_and_get::<LeakedValues>();
    }

    #[test]
    fn conflict() {
        fn conflict<V: DataValues>() {
            let mut data = Data::<V>::default();
            assert!(data.insert::<KeyA>(42).is_none());
            assert!(data.insert::<KeyB>(1).is_none());
            assert_eq!(data.get::<KeyA>().as_ref().map(Borrow::borrow), Some(&42));
            assert_eq!(data.get::<KeyB>().as_ref().map(Borrow::borrow), Some(&1));
        }
        conflict::<BoxValues>();
        conflict::<LeakedValues>();
    }

    #[test]
    fn insert_replace() {
        fn insert_replace<V: DataValues>() {
            let mut data = Data::<V>::default();
            assert!(data.insert::<KeyA>(42).is_none());
            assert_eq!(
                data.insert::<KeyA>(32).as_ref().map(Borrow::borrow),
                Some(&42)
            );
        }
        insert_replace::<BoxValues>();
        insert_replace::<LeakedValues>();
    }

    #[test]
    fn entry() {
        fn entry<V: DataValues>() {
            let mut data = Data::<V>::default();
            let DataEntry::Vacant(entry) = data.entry::<KeyA>() else {
                panic!("not vacant");
            };
            entry.insert(42);
            assert_eq!(data.get::<KeyA>().as_ref().map(Borrow::borrow), Some(&42));
            let DataEntry::Occupied(mut entry) = data.entry::<KeyA>() else {
                panic!("not occupied");
            };
            assert_eq!(entry.get().borrow(), &42);
            assert_eq!(entry.get_mut().borrow(), &42);
            assert_eq!(entry.remove().borrow(), &42);
            assert_eq!(
                data.entry::<KeyA>().insert_entry(32).into_mut().borrow(),
                &32
            );
        }
        entry::<BoxValues>();
        entry::<LeakedValues>();
    }
}
