use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::collections::hash_map;
use std::convert::Infallible;
use std::marker::PhantomData;

/// Container storing heterogeneous values via key types
///
/// Each key `K: DataKey` maps to exactly one value of type `K::Item`.
///
/// ## Example
///
/// ```rust
/// # use dijavu::{Data, DataKey};
/// struct MyKey;
/// impl DataKey for MyKey {
///     type Item = i32;
/// }
///
/// let mut data = Data::default();
/// data.insert::<MyKey>(42);
///
/// assert_eq!(data.get::<MyKey>(), Some(&42));
/// ```
#[derive(Default)]
pub struct Data(
    // INVARIANT: TypeId::of::<K: DataKey>() maps to a Box<K::Item>
    FxHashMap<TypeId, Box<dyn DataItem>>,
);

impl Data {
    /// Inserts a value for the given key type.
    ///
    /// Returns the previous value if one existed.
    pub fn insert<K: DataKey>(&mut self, data: K::Item) -> Option<K::Item> {
        self.0
            .insert(TypeId::of::<K>(), Box::new(data))
            .map(K::downcast)
    }

    /// Returns an entry API for in-place manipulation.
    ///
    /// Similar to [`HashMap::entry`](std::collections::HashMap::entry), but with a key type.
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
    pub fn remove<K: DataKey>(&mut self) -> Option<K::Item> {
        self.0.remove(&TypeId::of::<K>()).map(K::downcast)
    }

    /// Returns `true` if a value for key `K` exists.
    pub fn contains_key<K: DataKey>(&self) -> bool {
        self.0.contains_key(&TypeId::of::<K>())
    }

    /// Returns a reference to the value associated with key `K`.
    pub fn get<K: DataKey>(&self) -> Option<&K::Item> {
        self.0.get(&TypeId::of::<K>()).map(K::downcast_ref)
    }

    /// Returns a mutable reference to the value associated with key `K`.
    pub fn get_mut<K: DataKey>(&mut self) -> Option<&mut K::Item> {
        self.0.get_mut(&TypeId::of::<K>()).map(K::downcast_mut)
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
    pub fn insert_entry(self, item: K::Item) -> OccupiedEntry<'a, K> {
        match self {
            DataEntry::Vacant(entry) => entry.insert_entry(item),
            DataEntry::Occupied(mut entry) => {
                entry.insert(item);
                entry
            }
        }
    }

    pub fn or_insert_with(self, f: impl FnOnce() -> K::Item) -> &'a mut K::Item {
        match self.or_try_insert_with::<Infallible>(|| Ok(f())) {
            Ok(ok) => ok,
            Err(err) => match err {},
        }
    }

    /// Inserts the value provided by the fallible closure if no value is currently present.
    pub fn or_try_insert_with<E>(
        self,
        f: impl FnOnce() -> Result<K::Item, E>,
    ) -> Result<&'a mut K::Item, E> {
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
pub struct VacantEntry<'a, K>(
    hash_map::VacantEntry<'a, TypeId, Box<dyn DataItem>>,
    PhantomData<fn(K)>,
);

impl<'a, K> VacantEntry<'a, K>
where
    K: DataKey,
{
    /// Inserts a value and returns a mutable reference to it.
    pub fn insert(self, value: K::Item) -> &'a mut K::Item {
        K::downcast_mut(self.0.insert(Box::new(value)))
    }

    /// Inserts a value and returns an occupied entry.
    pub fn insert_entry(self, value: K::Item) -> OccupiedEntry<'a, K> {
        OccupiedEntry(self.0.insert_entry(Box::new(value)), PhantomData)
    }
}

/// An occupied entry in [`Data`]
pub struct OccupiedEntry<'a, K>(
    hash_map::OccupiedEntry<'a, TypeId, Box<dyn DataItem>>,
    PhantomData<fn(K)>,
);

impl<'a, K> OccupiedEntry<'a, K>
where
    K: DataKey,
{
    /// Replaces the value, returning the old one.
    pub fn insert(&mut self, value: K::Item) -> K::Item {
        K::downcast(self.0.insert(Box::new(value)))
    }

    /// Returns a shared reference to the value.
    pub fn get(&self) -> &K::Item {
        K::downcast_ref(self.0.get())
    }

    /// Returns a mutable reference to the value.
    pub fn get_mut(&mut self) -> &mut K::Item {
        K::downcast_mut(self.0.get_mut())
    }

    /// Converts into a mutable reference with the entry lifetime.
    pub fn into_mut(self) -> &'a mut K::Item {
        K::downcast_mut(self.0.into_mut())
    }

    /// Removes and returns the value.
    pub fn remove(self) -> K::Item {
        K::downcast(self.0.remove())
    }
}

/// Key type for accessing values in [`Data`]
///
/// Each key type defines the associated value type it stores.
pub trait DataKey: 'static {
    /// The value type stored in an entry for this key
    type Item: DataItem;
}

/// Internal helper trait for downcasting
///
/// This relies on the invariant that the stored type always matches `K::Item`.
trait KeyHelpers: DataKey {
    fn downcast(v: Box<dyn DataItem>) -> Self::Item {
        *Box::<dyn Any + Send + Sync>::downcast(v).unwrap()
    }
    #[expect(clippy::borrowed_box)]
    fn downcast_ref(r: &Box<dyn DataItem>) -> &Self::Item {
        (Box::as_ref(r) as &(dyn Any + Send + Sync))
            .downcast_ref::<Self::Item>()
            .unwrap()
    }
    fn downcast_mut(r: &mut Box<dyn DataItem>) -> &mut Self::Item {
        (Box::as_mut(r) as &mut (dyn Any + Send + Sync))
            .downcast_mut::<Self::Item>()
            .unwrap()
    }
}

impl<K> KeyHelpers for K where K: DataKey {}

/// Value that can be stored in [`Data`]
///
/// This is automatically implemented for all `Send + Sync + 'static` types.
pub trait DataItem: Any + Send + Sync {}

impl<T: Any + Send + Sync> DataItem for T {}

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
        assert_eq!(entry.get_mut(), &mut 42);
        assert_eq!(entry.remove(), 42);
        assert_eq!(data.entry::<KeyA>().insert_entry(32).into_mut(), &mut 32);
    }
}
